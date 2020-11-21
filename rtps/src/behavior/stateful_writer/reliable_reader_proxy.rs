use std::collections::BTreeSet;
use std::convert::TryInto;
use std::time::Instant;

use crate::behavior::ReaderProxy;
use crate::messages::submessages::{AckNack, Gap, Heartbeat};
use crate::messages::types::Count;
use crate::messages::RtpsSubmessage;
use crate::types::{EntityId, GuidPrefix, GUID};

use crate::behavior::types::Duration;
use crate::behavior::{data_from_cache_change, BEHAVIOR_ENDIANNESS};

use rust_dds_interface::history_cache::HistoryCache;
use rust_dds_interface::types::SequenceNumber;

pub struct ReliableReaderProxy {
    reader_proxy: ReaderProxy,

    heartbeat_count: Count,
    time_last_sent_data: Instant,
    time_nack_received: Instant,
    highest_nack_count_received: Count,
}

impl ReliableReaderProxy {
    pub fn new(reader_proxy: ReaderProxy) -> Self {
        Self {
            reader_proxy,
            heartbeat_count: 0,
            time_last_sent_data: Instant::now(),
            time_nack_received: Instant::now(),
            highest_nack_count_received: 0,
        }
    }

    pub fn produce_messages(
        &mut self,
        history_cache: &HistoryCache,
        writer_entity_id: EntityId,
        last_change_sequence_number: SequenceNumber,
        heartbeat_period: Duration,
        nack_response_delay: Duration,
    ) -> Vec<RtpsSubmessage> {
        let mut message_queue = Vec::new();

        if self
            .reader_proxy
            .unacked_changes(last_change_sequence_number)
            .is_empty()
        {
            // Idle
        } else if !self
            .reader_proxy
            .unsent_changes(last_change_sequence_number)
            .is_empty()
        {
            self.pushing_state(
                history_cache,
                last_change_sequence_number,
                writer_entity_id,
                &mut message_queue,
            );
        } else if !self
            .reader_proxy
            .unacked_changes(last_change_sequence_number)
            .is_empty()
        {
            self.announcing_state(
                history_cache,
                last_change_sequence_number,
                writer_entity_id,
                heartbeat_period,
                &mut message_queue,
            );
        }

        if !self.reader_proxy.requested_changes().is_empty() {
            let duration_since_nack_received: Duration =
                self.time_nack_received.elapsed().try_into().unwrap();
            if duration_since_nack_received > nack_response_delay {
                self.repairing_state(history_cache, writer_entity_id, &mut message_queue);
            }
        }

        message_queue
    }

    pub fn try_process_message(
        &mut self,
        src_guid_prefix: GuidPrefix,
        submessage: &mut Option<RtpsSubmessage>,
    ) {
        if let Some(RtpsSubmessage::AckNack(acknack)) = submessage {
            let reader_guid = GUID::new(src_guid_prefix, acknack.reader_id());
            if self.reader_proxy.remote_reader_guid == reader_guid {
                if let RtpsSubmessage::AckNack(acknack) = submessage.take().unwrap() {
                    if acknack.count() > self.highest_nack_count_received {
                        self.highest_nack_count_received = acknack.count();
                        if self.reader_proxy.requested_changes().is_empty() {
                            self.waiting_state(acknack);
                        } else {
                            self.must_repair_state(acknack);
                        }
                    }
                }
            }
        }
    }

    fn pushing_state(
        &mut self,
        history_cache: &HistoryCache,
        last_change_sequence_number: SequenceNumber,
        writer_entity_id: EntityId,
        message_queue: &mut Vec<RtpsSubmessage>,
    ) {
        while let Some(next_unsent_seq_num) = self
            .reader_proxy
            .next_unsent_change(last_change_sequence_number)
        {
            self.transition_t4(
                history_cache,
                next_unsent_seq_num,
                writer_entity_id,
                message_queue,
            );
        }
        self.time_last_sent_data = Instant::now();
    }

    fn transition_t4(
        &mut self,
        history_cache: &HistoryCache,
        next_unsent_seq_num: SequenceNumber,
        writer_entity_id: EntityId,
        message_queue: &mut Vec<RtpsSubmessage>,
    ) {
        if let Some(cache_change) = history_cache.get_change(next_unsent_seq_num) {
            let reader_id = self.reader_proxy.remote_reader_guid.entity_id();
            let data = data_from_cache_change(cache_change, reader_id);
            let mut dst_locator = self.reader_proxy.unicast_locator_list.clone();
            dst_locator.extend(&self.reader_proxy.unicast_locator_list);
            dst_locator.extend(&self.reader_proxy.multicast_locator_list);
            message_queue.push(RtpsSubmessage::Data(data));
        } else {
            let gap = Gap::new(
                BEHAVIOR_ENDIANNESS,
                self.reader_proxy.remote_reader_guid.entity_id(),
                writer_entity_id,
                next_unsent_seq_num,
                BTreeSet::new(),
            );

            let mut dst_locator = self.reader_proxy.unicast_locator_list.clone();
            dst_locator.extend(&self.reader_proxy.unicast_locator_list);
            dst_locator.extend(&self.reader_proxy.multicast_locator_list);
            message_queue.push(RtpsSubmessage::Gap(gap));
        }
    }

    fn announcing_state(
        &mut self,
        history_cache: &HistoryCache,
        last_change_sequence_number: SequenceNumber,
        writer_entity_id: EntityId,
        heartbeat_period: Duration,
        message_queue: &mut Vec<RtpsSubmessage>,
    ) {
        let duration_since_last_sent_data: Duration =
            self.time_last_sent_data.elapsed().try_into().unwrap();
        if duration_since_last_sent_data > heartbeat_period {
            self.transition_t7(
                history_cache,
                last_change_sequence_number,
                writer_entity_id,
                message_queue,
            );
            self.time_last_sent_data = Instant::now();
        }
    }

    fn transition_t7(
        &mut self,
        history_cache: &HistoryCache,
        last_change_sequence_number: SequenceNumber,
        writer_entity_id: EntityId,
        message_queue: &mut Vec<RtpsSubmessage>,
    ) {
        let first_sn = if let Some(seq_num) = history_cache.get_seq_num_min() {
            seq_num
        } else {
            last_change_sequence_number + 1
        };
        self.heartbeat_count += 1;

        let heartbeat = Heartbeat::new(
            BEHAVIOR_ENDIANNESS,
            self.reader_proxy.remote_reader_guid.entity_id(),
            writer_entity_id,
            first_sn,
            last_change_sequence_number,
            self.heartbeat_count,
            false,
            false,
        );

        let mut dst_locator = self.reader_proxy.unicast_locator_list.clone();
        dst_locator.extend(&self.reader_proxy.unicast_locator_list);
        dst_locator.extend(&self.reader_proxy.multicast_locator_list);

        message_queue.push(RtpsSubmessage::Heartbeat(heartbeat));
    }

    fn waiting_state(&mut self, acknack: AckNack) {
        self.transition_t8(acknack);
        self.time_nack_received = Instant::now();
    }

    fn transition_t8(&mut self, acknack: AckNack) {
        self.reader_proxy
            .acked_changes_set(acknack.reader_sn_state().base() - 1);
        self.reader_proxy
            .requested_changes_set(acknack.reader_sn_state().set().clone());
    }

    fn must_repair_state(&mut self, acknack: AckNack) {
        self.transition_t8(acknack);
    }

    fn repairing_state(
        &mut self,
        history_cache: &HistoryCache,
        writer_entity_id: EntityId,
        message_queue: &mut Vec<RtpsSubmessage>,
    ) {
        // This state is only valid if there are requested changes
        debug_assert!(!self.reader_proxy.requested_changes().is_empty());

        while let Some(next_requested_seq_num) = self.reader_proxy.next_requested_change() {
            self.transition_t12(
                history_cache,
                next_requested_seq_num,
                writer_entity_id,
                message_queue,
            );
        }
    }

    fn transition_t12(
        &mut self,
        history_cache: &HistoryCache,
        next_requested_seq_num: SequenceNumber,
        writer_entity_id: EntityId,
        message_queue: &mut Vec<RtpsSubmessage>,
    ) {
        if let Some(cache_change) = history_cache.get_change(next_requested_seq_num) {
            let data = data_from_cache_change(
                cache_change,
                self.reader_proxy.remote_reader_guid.entity_id(),
            );
            let mut dst_locator = self.reader_proxy.unicast_locator_list.clone();
            dst_locator.extend(&self.reader_proxy.unicast_locator_list);
            dst_locator.extend(&self.reader_proxy.multicast_locator_list);
            message_queue.push(RtpsSubmessage::Data(data));
        } else {
            let gap = Gap::new(
                BEHAVIOR_ENDIANNESS,
                self.reader_proxy.remote_reader_guid.entity_id(),
                writer_entity_id,
                next_requested_seq_num,
                BTreeSet::new(),
            );

            let mut dst_locator = self.reader_proxy.unicast_locator_list.clone();
            dst_locator.extend(&self.reader_proxy.unicast_locator_list);
            dst_locator.extend(&self.reader_proxy.multicast_locator_list);
            message_queue.push(RtpsSubmessage::Gap(gap));
        }
    }
}

impl std::ops::Deref for ReliableReaderProxy {
    type Target = ReaderProxy;

    fn deref(&self) -> &Self::Target {
        &self.reader_proxy
    }
}

impl std::ops::DerefMut for ReliableReaderProxy {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader_proxy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::types::Endianness;
    use crate::types::constants::{
        ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER, ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
    };
    use crate::types::{Locator, GUID};

    use rust_dds_interface::cache_change::CacheChange;
    use rust_dds_interface::types::ChangeKind;

    #[test]
    fn produce_empty() {
        let remote_reader_guid = GUID::new([5; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER);
        let unicast_locator_list = vec![Locator::new_udpv4(7400, [127, 0, 0, 1])];
        let multicast_locator_list = vec![];
        let expects_inline_qos = false;
        let is_active = true;
        let reader_proxy = ReaderProxy::new(
            remote_reader_guid,
            unicast_locator_list,
            multicast_locator_list,
            expects_inline_qos,
            is_active,
        );
        let mut reliable_reader_proxy = ReliableReaderProxy::new(reader_proxy);

        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;
        let history_cache = HistoryCache::default();

        // Run without any change being created or added in the cache
        let heartbeat_period = Duration::from_secs(1);
        let nack_response_delay = Duration::from_secs(1);
        let last_change_sequence_number = 0;
        let messages_vec = reliable_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
            heartbeat_period,
            nack_response_delay,
        );

        assert!(messages_vec.is_empty());
    }

    #[test]
    fn produce_data_message() {
        let remote_reader_guid = GUID::new([5; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER);
        let unicast_locator_list = vec![Locator::new_udpv4(7400, [127, 0, 0, 1])];
        let multicast_locator_list = vec![];
        let expects_inline_qos = false;
        let is_active = true;
        let reader_proxy = ReaderProxy::new(
            remote_reader_guid,
            unicast_locator_list,
            multicast_locator_list,
            expects_inline_qos,
            is_active,
        );
        let mut reliable_reader_proxy = ReliableReaderProxy::new(reader_proxy);

        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;
        let mut history_cache = HistoryCache::default();

        // Add one change to the history cache
        let writer_guid = GUID::new([5; 12], writer_entity_id);
        let instance_handle = [1; 16];
        let cache_change1 = CacheChange::new(
            ChangeKind::Alive,
            writer_guid.into(),
            instance_handle,
            1,
            Some(vec![1, 2, 3]),
            None,
        );
        history_cache.add_change(cache_change1.clone()).unwrap();

        // Run with the last change sequence number equal to the added cache change
        let last_change_sequence_number = 1;
        let heartbeat_period = Duration::from_secs(1);
        let nack_response_delay = Duration::from_secs(1);
        let messages_vec = reliable_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
            heartbeat_period,
            nack_response_delay,
        );

        let expected_data_submessage = RtpsSubmessage::Data(data_from_cache_change(
            &cache_change1,
            ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER,
        ));
        assert_eq!(messages_vec.len(), 1);
        assert!(messages_vec.contains(&expected_data_submessage));
    }

    #[test]
    fn produce_gap_message() {
        let remote_reader_guid = GUID::new([5; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER);
        let unicast_locator_list = vec![Locator::new_udpv4(7400, [127, 0, 0, 1])];
        let multicast_locator_list = vec![];
        let expects_inline_qos = false;
        let is_active = true;
        let reader_proxy = ReaderProxy::new(
            remote_reader_guid,
            unicast_locator_list,
            multicast_locator_list,
            expects_inline_qos,
            is_active,
        );
        let mut reliable_reader_proxy = ReliableReaderProxy::new(reader_proxy);

        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;
        let history_cache = HistoryCache::default();

        // Run with the a sequence number of 1 without adding any change to the history cache
        let last_change_sequence_number = 1;
        let heartbeat_period = Duration::from_secs(1);
        let nack_response_delay = Duration::from_secs(1);
        let messages_vec = reliable_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
            heartbeat_period,
            nack_response_delay,
        );

        let expected_gap_submessage = RtpsSubmessage::Gap(Gap::new(
            BEHAVIOR_ENDIANNESS,
            ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER,
            writer_entity_id,
            1,
            BTreeSet::new(),
        ));
        assert_eq!(messages_vec.len(), 1);
        assert!(messages_vec.contains(&expected_gap_submessage));
    }

    #[test]
    fn produce_data_and_gap_messages() {
        let remote_reader_guid = GUID::new([5; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER);
        let unicast_locator_list = vec![Locator::new_udpv4(7400, [127, 0, 0, 1])];
        let multicast_locator_list = vec![];
        let expects_inline_qos = false;
        let is_active = true;
        let reader_proxy = ReaderProxy::new(
            remote_reader_guid,
            unicast_locator_list,
            multicast_locator_list,
            expects_inline_qos,
            is_active,
        );
        let mut reliable_reader_proxy = ReliableReaderProxy::new(reader_proxy);

        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;
        let mut history_cache = HistoryCache::default();

        // Add one change to the history cache
        let writer_guid = GUID::new([5; 12], writer_entity_id);
        let instance_handle = [1; 16];
        let cache_change1 = CacheChange::new(
            ChangeKind::Alive,
            writer_guid.into(),
            instance_handle,
            1,
            Some(vec![1, 2, 3]),
            None,
        );
        history_cache.add_change(cache_change1.clone()).unwrap();

        // Run with the last change sequence number one above the added cache change
        let last_change_sequence_number = 2;
        let heartbeat_period = Duration::from_secs(1);
        let nack_response_delay = Duration::from_secs(1);
        let messages_vec = reliable_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
            heartbeat_period,
            nack_response_delay,
        );

        let expected_data_submessage = RtpsSubmessage::Data(data_from_cache_change(
            &cache_change1,
            ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER,
        ));
        let expected_gap_submessage = RtpsSubmessage::Gap(Gap::new(
            BEHAVIOR_ENDIANNESS,
            ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER,
            writer_entity_id,
            2,
            BTreeSet::new(),
        ));
        assert_eq!(messages_vec.len(), 2);
        assert!(messages_vec.contains(&expected_data_submessage));
        assert!(messages_vec.contains(&expected_gap_submessage));
    }

    #[test]
    fn try_process_acknack_message_only_acknowledge() {
        let remote_reader_guid_prefix = [5; 12];
        let remote_reader_guid_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER;
        let remote_reader_guid = GUID::new(remote_reader_guid_prefix, remote_reader_guid_entity_id);
        let unicast_locator_list = vec![Locator::new_udpv4(7400, [127, 0, 0, 1])];
        let multicast_locator_list = vec![];
        let expects_inline_qos = false;
        let is_active = true;
        let reader_proxy = ReaderProxy::new(
            remote_reader_guid,
            unicast_locator_list,
            multicast_locator_list,
            expects_inline_qos,
            is_active,
        );
        let mut reliable_reader_proxy = ReliableReaderProxy::new(reader_proxy);

        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;

        let acknack = AckNack::new(
            Endianness::LittleEndian,
            remote_reader_guid.entity_id(),
            writer_entity_id,
            2,
            vec![].iter().cloned().collect(),
            1,
            true,
        );

        reliable_reader_proxy.try_process_message(
            remote_reader_guid_prefix,
            &mut Some(RtpsSubmessage::AckNack(acknack)),
        );

        assert_eq!(reliable_reader_proxy.highest_nack_count_received, 1);
        assert!(reliable_reader_proxy
            .reader_proxy
            .unacked_changes(1)
            .is_empty()); // If 1 is the last change sequence number there are no unacked changes
        assert!(reliable_reader_proxy
            .reader_proxy
            .unacked_changes(2)
            .contains(&2)); // If 2 is the last change sequence number, then 2 is an unacked change
    }

    #[test]
    fn try_process_acknack_message_acknowledge_and_request() {
        let remote_reader_guid_prefix = [5; 12];
        let remote_reader_guid_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER;
        let remote_reader_guid = GUID::new(remote_reader_guid_prefix, remote_reader_guid_entity_id);
        let unicast_locator_list = vec![Locator::new_udpv4(7400, [127, 0, 0, 1])];
        let multicast_locator_list = vec![];
        let expects_inline_qos = false;
        let is_active = true;
        let reader_proxy = ReaderProxy::new(
            remote_reader_guid,
            unicast_locator_list,
            multicast_locator_list,
            expects_inline_qos,
            is_active,
        );
        let mut reliable_reader_proxy = ReliableReaderProxy::new(reader_proxy);

        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;

        let acknack = AckNack::new(
            Endianness::LittleEndian,
            remote_reader_guid.entity_id(),
            writer_entity_id,
            4,
            vec![1, 3].iter().cloned().collect(),
            1,
            true,
        );

        reliable_reader_proxy.try_process_message(
            remote_reader_guid_prefix,
            &mut Some(RtpsSubmessage::AckNack(acknack)),
        );

        let requested_changes = reliable_reader_proxy.reader_proxy.requested_changes();
        assert!(requested_changes.contains(&1));
        assert!(requested_changes.contains(&3));
    }

    #[test]
    fn ignore_try_process_acknack_message_with_same_count_number() {
        let remote_reader_guid_prefix = [5; 12];
        let remote_reader_guid_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER;
        let remote_reader_guid = GUID::new(remote_reader_guid_prefix, remote_reader_guid_entity_id);
        let unicast_locator_list = vec![Locator::new_udpv4(7400, [127, 0, 0, 1])];
        let multicast_locator_list = vec![];
        let expects_inline_qos = false;
        let is_active = true;
        let reader_proxy = ReaderProxy::new(
            remote_reader_guid,
            unicast_locator_list,
            multicast_locator_list,
            expects_inline_qos,
            is_active,
        );
        let mut reliable_reader_proxy = ReliableReaderProxy::new(reader_proxy);

        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;

        let acknack = AckNack::new(
            Endianness::LittleEndian,
            remote_reader_guid.entity_id(),
            writer_entity_id,
            4,
            vec![1, 3].iter().cloned().collect(),
            1,
            true,
        );

        reliable_reader_proxy.try_process_message(
            remote_reader_guid_prefix,
            &mut Some(RtpsSubmessage::AckNack(acknack)),
        );

        let acknack_same_count = AckNack::new(
            Endianness::LittleEndian,
            remote_reader_guid.entity_id(),
            writer_entity_id,
            6,
            vec![1, 2, 3].iter().cloned().collect(),
            1,
            true,
        );

        reliable_reader_proxy.try_process_message(
            remote_reader_guid_prefix,
            &mut Some(RtpsSubmessage::AckNack(acknack_same_count)),
        );

        assert_eq!(reliable_reader_proxy.highest_nack_count_received, 1);
        let requested_changes = reliable_reader_proxy.reader_proxy.requested_changes();
        assert!(requested_changes.contains(&1));
        assert!(requested_changes.contains(&3));
    }

    #[test]
    fn produce_heartbeat_message() {
        let remote_reader_guid = GUID::new([5; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER);
        let unicast_locator_list = vec![Locator::new_udpv4(7400, [127, 0, 0, 1])];
        let multicast_locator_list = vec![];
        let expects_inline_qos = false;
        let is_active = true;
        let reader_proxy = ReaderProxy::new(
            remote_reader_guid,
            unicast_locator_list,
            multicast_locator_list,
            expects_inline_qos,
            is_active,
        );
        let mut reliable_reader_proxy = ReliableReaderProxy::new(reader_proxy);

        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;
        let mut history_cache = HistoryCache::default();

        // Add one change to the history cache
        let writer_guid = GUID::new([5; 12], writer_entity_id);
        let instance_handle = [1; 16];
        let cache_change1 = CacheChange::new(
            ChangeKind::Alive,
            writer_guid.into(),
            instance_handle,
            1,
            Some(vec![1, 2, 3]),
            None,
        );
        history_cache.add_change(cache_change1.clone()).unwrap();

        let writer_guid = GUID::new([5; 12], writer_entity_id);
        let instance_handle = [1; 16];
        let cache_change1 = CacheChange::new(
            ChangeKind::Alive,
            writer_guid.into(),
            instance_handle,
            2,
            Some(vec![4, 5, 6]),
            None,
        );
        history_cache.add_change(cache_change1.clone()).unwrap();

        let last_change_sequence_number = 2;
        let heartbeat_period = Duration::from_secs(0);
        let nack_response_delay = Duration::from_secs(1);

        // The first produce should generate the data/gap messages and no heartbeat so we ignore it
        reliable_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
            heartbeat_period,
            nack_response_delay,
        );

        let messages_vec1 = reliable_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
            heartbeat_period,
            nack_response_delay,
        );

        let messages_vec2 = reliable_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
            heartbeat_period,
            nack_response_delay,
        );

        let expected_heartbeat_message1 = RtpsSubmessage::Heartbeat(Heartbeat::new(
            Endianness::LittleEndian,
            ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER,
            ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
            1,
            2,
            1,
            false,
            false,
        ));

        let expected_heartbeat_message2 = RtpsSubmessage::Heartbeat(Heartbeat::new(
            Endianness::LittleEndian,
            ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER,
            ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER,
            1,
            2,
            2,
            false,
            false,
        ));

        assert_eq!(messages_vec1.len(), 1);
        assert!(messages_vec1.contains(&expected_heartbeat_message1));
        assert_eq!(messages_vec2.len(), 1);
        assert!(messages_vec2.contains(&expected_heartbeat_message2));
    }
    // use super::*;
    // use crate::types::GUID;
    // use crate::behavior::types::constants::DURATION_ZERO;
    // use crate::types::constants::{
    //     ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR };

    // use crate::messages::receiver::WriterReceiveMessage;
    // use crate::stateful_writer::StatefulWriter;

    // use rust_dds_interface::qos_policy::ResourceLimitsQosPolicy;
    // use std::collections::BTreeSet;
    // use std::thread::sleep;

    // #[test]
    // fn process_repair_message_acknowledged_and_requests() {
    //     let remote_reader_guid = GUID::new([1,2,3,4,5,6,7,8,9,10,11,12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);

    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);

    //     let acknack = AckNack::new(
    //         *remote_reader_guid.entity_id(),
    //        *writer_guid.entity_id(),
    //        3,
    //         vec![3, 5, 6].iter().cloned().collect(),
    //        1,
    //         true,
    //         Endianness::LittleEndian);
    //     let received_message = RtpsMessage::new(*remote_reader_guid.prefix(), vec![RtpsSubmessage::AckNack(acknack)]);

    //     StatefulWriterBehavior::process_repair_message(&mut reader_proxy, &writer_guid, &received_message);

    //     assert_eq!(reader_proxy.acked_changes(), 2);
    //     assert_eq!(reader_proxy.requested_changes(), vec![3, 5, 6].iter().cloned().collect());
    // }

    // #[test]
    // fn process_repair_message_different_conditions() {
    //     let remote_reader_guid = GUID::new([1,2,3,4,5,6,7,8,9,10,11,12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);

    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);

    //     // Test message with different reader guid
    //     let mut submessages = Vec::new();
    //     let other_reader_guid = GUID::new([9;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let acknack = AckNack::new(
    //         *other_reader_guid.entity_id(),
    //        *writer_guid.entity_id(),
    //        3,
    //         vec![3, 5, 6].iter().cloned().collect(),
    //        1,
    //         true,
    //         Endianness::LittleEndian);
    //     submessages.push(RtpsSubmessage::AckNack(acknack));
    //     let received_message = RtpsMessage::new(*other_reader_guid.prefix(), submessages);
    //     StatefulWriterBehavior::process_repair_message(&mut reader_proxy, &writer_guid, &received_message);

    //     // Verify that message was ignored
    //     // assert_eq!(reader_proxy.highest_sequence_number_acknowledged, 0);
    //     assert!(reader_proxy.requested_changes().is_empty());

    //     // Test message with different writer guid
    //     let mut submessages = Vec::new();
    //     let acknack = AckNack::new(
    //         *remote_reader_guid.entity_id(),
    //         ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
    //        3,
    //        vec![5, 6].iter().cloned().collect(),
    //        1,
    //         true,
    //         Endianness::LittleEndian);
    //     submessages.push(RtpsSubmessage::AckNack(acknack));
    //     let received_message = RtpsMessage::new(*remote_reader_guid.prefix(), submessages);

    //     StatefulWriterBehavior::process_repair_message(&mut reader_proxy, &writer_guid, &received_message);

    //     // Verify that message was ignored
    //     assert_eq!(reader_proxy.acked_changes(), 0);
    //     assert!(reader_proxy.requested_changes().is_empty());

    //     // Test duplicate acknack message
    //     let mut submessages = Vec::new();
    //     let acknack = AckNack::new(
    //         *remote_reader_guid.entity_id(),
    //         *writer_guid.entity_id(),
    //        3,
    //         vec![3, 5, 6].iter().cloned().collect(),
    //        1,
    //         true,
    //         Endianness::LittleEndian);
    //     submessages.push(RtpsSubmessage::AckNack(acknack));
    //     let received_message = RtpsMessage::new(*remote_reader_guid.prefix(), submessages);

    //     StatefulWriterBehavior::process_repair_message(&mut reader_proxy, &writer_guid, &received_message);

    //     // Verify message was correctly processed
    //     assert_eq!(reader_proxy.acked_changes(), 2);
    //     assert_eq!(reader_proxy.requested_changes(), vec![3, 5, 6].iter().cloned().collect());

    //     // Clear the requested sequence numbers and reprocess the message
    //     while  reader_proxy.next_requested_change() != None {
    //         // do nothing
    //     }
    //     StatefulWriterBehavior::process_repair_message(&mut reader_proxy, &writer_guid, &received_message);

    //     // Verify that the requested sequence numbers remain empty
    //     assert!(reader_proxy.requested_changes().is_empty());
    // }

    // #[test]
    // fn process_repair_message_only_acknowledged() {
    //     let remote_reader_guid = GUID::new([1,2,3,4,5,6,7,8,9,10,11,12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);

    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);

    //     let mut submessages = Vec::new();
    //     let acknack = AckNack::new(
    //         *remote_reader_guid.entity_id(),
    //         *writer_guid.entity_id(),
    //        5,
    //        vec![].iter().cloned().collect(),
    //        1,
    //         true,
    //         Endianness::LittleEndian);
    //     submessages.push(RtpsSubmessage::AckNack(acknack));

    //     let received_message = RtpsMessage::new(*remote_reader_guid.prefix(), submessages);
    //     StatefulWriterBehavior::process_repair_message(&mut reader_proxy, &writer_guid, &received_message);

    //     assert_eq!(reader_proxy.acked_changes(), 4);
    //     assert!(reader_proxy.requested_changes().is_empty());
    // }

    // // #[test]
    // fn run_pushing_state_only_data_messages() {
    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let history_cache = HistoryCache::new();

    //     let instance_handle = [1;16];

    //     let cache_change_seq1 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 1, Some(vec![1,2,3]), None);
    //     let cache_change_seq2 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 2, Some(vec![2,3,4]), None);
    //     history_cache.add_change(cache_change_seq1);
    //     history_cache.add_change(cache_change_seq2);
    //     let last_change_sequence_number  = 2;

    //     let remote_reader_guid = GUID::new([1,2,3,4,5,6,7,8,9,10,11,12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);

    //     let submessages = StatefulWriterBehavior::run_pushing_state(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number);
    //     assert_eq!(submessages.len(), 3);
    //     if let RtpsSubmessage::InfoTs(message_1) = &submessages[0] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Data(data_message_1) = &submessages[1] {
    //         assert_eq!(data_message_1.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(data_message_1.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(data_message_1.writer_sn(), 1);
    //         assert_eq!(data_message_1.serialized_payload(), Some(&vec![1, 2, 3]));

    //     } else {
    //         panic!("Wrong message type");
    //     };

    //     if let RtpsSubmessage::Data(data_message_2) = &submessages[2] {
    //         assert_eq!(data_message_2.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(data_message_2.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(data_message_2.writer_sn(), 2);
    //         assert_eq!(data_message_2.serialized_payload(), Some(&vec![2, 3, 4]));
    //     } else {
    //         panic!("Wrong message type");
    //     };
    // }

    // #[test]
    // fn run_pushing_state_only_gap_message() {
    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let history_cache = HistoryCache::new();

    //     // Don't add any change to the history cache so that gap message has to be sent
    //     // let instance_handle = [1;16];

    //     // let cache_change_seq1 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 1, None, Some(vec![1,2,3]));
    //     // let cache_change_seq2 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 2, None, Some(vec![2,3,4]));
    //     // history_cache.add_change(cache_change_seq1);
    //     // history_cache.add_change(cache_change_seq2);

    //     let last_change_sequence_number  = 2;

    //     let remote_reader_guid = GUID::new([1,2,3,4,5,6,7,8,9,10,11,12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);

    //     let submessages = StatefulWriterBehavior::run_pushing_state(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number);
    //     assert_eq!(submessages.len(), 3);
    //     if let RtpsSubmessage::InfoTs(message_1) = &submessages[0] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Gap(gap_message_1) = &submessages[1] {
    //         assert_eq!(gap_message_1.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(gap_message_1.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(gap_message_1.gap_start(), 1);
    //     } else {
    //         panic!("Wrong message type");
    //     };
    //     if let RtpsSubmessage::Gap(gap_message_2) = &submessages[2] {
    //         assert_eq!(gap_message_2.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(gap_message_2.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(gap_message_2.gap_start(), 2);
    //     } else {
    //         panic!("Wrong message type");
    //     };
    // }

    // #[test]
    // fn run_pushing_state_gap_and_data_message() {
    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let history_cache = HistoryCache::new();

    //     // Add one change to the history cache so that data and gap messages have to be sent
    //     let instance_handle = [1;16];

    //     // let cache_change_seq1 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 1, None, Some(vec![1,2,3]));
    //     let cache_change_seq2 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 2, Some(vec![2,3,4]), None);
    //     // history_cache.add_change(cache_change_seq1);
    //     history_cache.add_change(cache_change_seq2);

    //     let last_change_sequence_number  = 2;

    //     let remote_reader_guid = GUID::new([1,2,3,4,5,6,7,8,9,10,11,12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);

    //     let submessages = StatefulWriterBehavior::run_pushing_state(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number);
    //     assert_eq!(submessages.len(), 3);
    //     if let RtpsSubmessage::InfoTs(message_1) = &submessages[0] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Gap(gap_message) = &submessages[1] {
    //         assert_eq!(gap_message.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(gap_message.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(gap_message.gap_start(), 1);
    //     } else {
    //         panic!("Wrong message type");
    //     };
    //     if let RtpsSubmessage::Data(data_message) = &submessages[2] {
    //         assert_eq!(data_message.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(data_message.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(data_message.writer_sn(), 2);
    //         assert_eq!(data_message.serialized_payload(), Some(&vec![2, 3, 4]));
    //     } else {
    //         panic!("Wrong message type");
    //     };
    // }

    // #[test]
    // fn run_repairing_state_only_data_messages() {
    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let history_cache = HistoryCache::new();

    //     let instance_handle = [1;16];

    //     let cache_change_seq1 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 1, Some(vec![1,2,3]), None);
    //     let cache_change_seq2 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 2, Some(vec![2,3,4]), None);
    //     history_cache.add_change(cache_change_seq1);
    //     history_cache.add_change(cache_change_seq2);

    //     let remote_reader_guid = GUID::new([1,2,3,4,5,6,7,8,9,10,11,12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);
    //     reader_proxy.requested_changes_set(vec![1, 2].iter().cloned().collect());

    //     let submessages = StatefulWriterBehavior::run_repairing_state(&mut reader_proxy, &writer_guid, &history_cache);
    //     assert_eq!(submessages.len(), 3);
    //     if let RtpsSubmessage::InfoTs(message_1) = &submessages[0] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Data(data_message_1) = &submessages[1] {
    //         assert_eq!(data_message_1.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(data_message_1.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(data_message_1.writer_sn(), 1);
    //         assert_eq!(data_message_1.serialized_payload(), Some(&vec![1, 2, 3]));

    //     } else {
    //         panic!("Wrong message type");
    //     };

    //     if let RtpsSubmessage::Data(data_message_2) = &submessages[2] {
    //         assert_eq!(data_message_2.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(data_message_2.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(data_message_2.writer_sn(), 2);
    //         assert_eq!(data_message_2.serialized_payload(), Some(&vec![2, 3, 4]));
    //     } else {
    //         panic!("Wrong message type");
    //     };
    // }

    // #[test]
    // fn run_repairing_state_only_gap_messages() {
    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let history_cache = HistoryCache::new();

    //     let remote_reader_guid = GUID::new([1,2,3,4,5,6,7,8,9,10,11,12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);
    //     reader_proxy.requested_changes_set(vec![1, 2].iter().cloned().collect());

    //     let submessages = StatefulWriterBehavior::run_repairing_state(&mut reader_proxy, &writer_guid, &history_cache);
    //     assert_eq!(submessages.len(), 3);
    //     if let RtpsSubmessage::InfoTs(message_1) = &submessages[0] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Gap(gap_message_1) = &submessages[1] {
    //         assert_eq!(gap_message_1.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(gap_message_1.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(gap_message_1.gap_start(), 1);
    //     } else {
    //         panic!("Wrong message type");
    //     };
    //     if let RtpsSubmessage::Gap(gap_message_2) = &submessages[2] {
    //         assert_eq!(gap_message_2.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(gap_message_2.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(gap_message_2.gap_start(), 2);
    //     } else {
    //         panic!("Wrong message type");
    //     };
    // }

    // #[test]
    // fn run_best_effort_reader_proxy() {
    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let history_cache = HistoryCache::new();

    //     let remote_reader_guid = GUID::new([1;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);
    //     let last_change_sequence_number = 0;

    //     assert!(StatefulWriterBehavior::run_best_effort(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number).is_none());

    //     let instance_handle = [1;16];

    //     let cache_change_seq1 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 1, Some(vec![1,2,3]), None);
    //     let cache_change_seq2 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 2, Some(vec![2,3,4]), None);
    //     history_cache.add_change(cache_change_seq1);
    //     history_cache.add_change(cache_change_seq2);
    //     let last_change_sequence_number = 2;

    //     let submessages = StatefulWriterBehavior::run_best_effort(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number).unwrap();
    //     assert_eq!(submessages.len(), 3);
    //     if let RtpsSubmessage::InfoTs(message_1) = &submessages[0] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Data(data_message_1) = &submessages[1] {
    //         assert_eq!(data_message_1.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(data_message_1.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(data_message_1.writer_sn(), 1);
    //         assert_eq!(data_message_1.serialized_payload(), Some(&vec![1, 2, 3]));

    //     } else {
    //         panic!("Wrong message type");
    //     };

    //     if let RtpsSubmessage::Data(data_message_2) = &submessages[2] {
    //         assert_eq!(data_message_2.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(data_message_2.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(data_message_2.writer_sn(), 2);
    //         assert_eq!(data_message_2.serialized_payload(), Some(&vec![2, 3, 4]));
    //     } else {
    //         panic!("Wrong message type");
    //     };

    //     assert!(StatefulWriterBehavior::run_best_effort(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number).is_none());
    // }

    // #[test]
    // fn run_reliable_reader_proxy() {
    //     let heartbeat_period = Duration::from_millis(200);
    //     let nack_response_delay = Duration::from_millis(200);
    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let history_cache = HistoryCache::new();

    //     let remote_reader_guid = GUID::new([1;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);
    //     let last_change_sequence_number = 0;

    //     // Check that immediately after creation no message is sent
    //     assert!(StatefulWriterBehavior::run_reliable(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number, heartbeat_period, nack_response_delay, None).is_none());

    //     // Add two changes to the history cache and check that two data messages are sent
    //     let instance_handle = [1;16];

    //     let cache_change_seq1 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 1, Some(vec![1,2,3]), None);
    //     let cache_change_seq2 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 2, Some(vec![2,3,4]), None);
    //     history_cache.add_change(cache_change_seq1);
    //     history_cache.add_change(cache_change_seq2);
    //     let last_change_sequence_number = 2;

    //     let submessages = StatefulWriterBehavior::run_reliable(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number, heartbeat_period, nack_response_delay, None).unwrap();
    //     assert_eq!(submessages.len(), 3);
    //     if let RtpsSubmessage::InfoTs(message_1) = &submessages[0] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Data(data_message_1) = &submessages[1] {
    //         assert_eq!(data_message_1.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(data_message_1.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(data_message_1.writer_sn(), 1);
    //         assert_eq!(data_message_1.serialized_payload(), Some(&vec![1, 2, 3]));

    //     } else {
    //         panic!("Wrong message type");
    //     };

    //     if let RtpsSubmessage::Data(data_message_2) = &submessages[2] {
    //         assert_eq!(data_message_2.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(data_message_2.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(data_message_2.writer_sn(), 2);
    //         assert_eq!(data_message_2.serialized_payload(), Some(&vec![2, 3, 4]));
    //     } else {
    //         panic!("Wrong message type");
    //     };

    //     // Check that immediately after sending the data nothing else is sent
    //     assert!(StatefulWriterBehavior::run_reliable(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number, heartbeat_period, nack_response_delay, None).is_none());

    //     // Check that a heartbeat is sent after the heartbeat period
    //     sleep(heartbeat_period.into());

    //     let submessages = StatefulWriterBehavior::run_reliable(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number, heartbeat_period, nack_response_delay, None).unwrap();
    //     assert_eq!(submessages.len(), 2);
    //     if let RtpsSubmessage::InfoTs(message_1) = &submessages[0] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Heartbeat(heartbeat_message) = &submessages[1] {
    //         assert_eq!(heartbeat_message.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(heartbeat_message.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(heartbeat_message.first_sn(), 1);
    //         assert_eq!(heartbeat_message.last_sn(), 2);
    //         assert_eq!(heartbeat_message.count(), 1);
    //         assert_eq!(heartbeat_message.is_final(), false);

    //     } else {
    //         panic!("Wrong message type");
    //     };

    //     // Check that if a sample is requested it gets sent after the nack_response_delay. In this case it comes together with a heartbeat
    //     let acknack = AckNack::new(
    //         *remote_reader_guid.entity_id(),
    //         *writer_guid.entity_id(),
    //        1,
    //        vec![2].iter().cloned().collect(),
    //        1,
    //        true,
    //        Endianness::LittleEndian);
    //     let received_message = RtpsMessage::new(*remote_reader_guid.prefix(), vec![RtpsSubmessage::AckNack(acknack)]);

    //     let submessages = StatefulWriterBehavior::run_reliable(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number, heartbeat_period, nack_response_delay, Some(&received_message));
    //     assert!(submessages.is_none());

    //     sleep(nack_response_delay.into());

    //     let submessages = StatefulWriterBehavior::run_reliable(&mut reader_proxy, &writer_guid, &history_cache, last_change_sequence_number, heartbeat_period, nack_response_delay, Some(&received_message)).unwrap();
    //     assert_eq!(submessages.len(), 4);
    //     if let RtpsSubmessage::InfoTs(message_1) = &submessages[0] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Heartbeat(heartbeat_message) = &submessages[1] {
    //         assert_eq!(heartbeat_message.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(heartbeat_message.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(heartbeat_message.first_sn(), 1);
    //         assert_eq!(heartbeat_message.last_sn(), 2);
    //         assert_eq!(heartbeat_message.count(), 2);
    //         assert_eq!(heartbeat_message.is_final(), false);
    //     }
    //     if let RtpsSubmessage::InfoTs(message_1) = &submessages[2] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Data(data_message_1) = &submessages[3] {
    //         assert_eq!(data_message_1.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(data_message_1.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(data_message_1.writer_sn(), 2);
    //         assert_eq!(data_message_1.serialized_payload(), Some(&vec![2, 3, 4]));

    //     } else {
    //         panic!("Wrong message type");
    //     };

    // }

    // #[test]
    // fn best_effort_stateful_writer_run() {
    //     let mut writer = StatefulWriter::new(
    //         GUID::new([0; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER),
    //         TopicKind::WithKey,
    //         ReliabilityKind::BestEffort,
    //         vec![Locator::new(0, 7400, [0; 16])],
    //         vec![],
    //         false,
    //         DURATION_ZERO,
    //         DURATION_ZERO,
    //         DURATION_ZERO,
    //     );

    //     let reader_guid = GUID::new([1;12], ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR);
    //     let reader_proxy = ReaderProxy::new(reader_guid, vec![], vec![], false, true);

    //     writer.matched_reader_add(reader_proxy);

    //     let cache_change_seq1 = writer.new_change(
    //         ChangeKind::Alive,
    //         Some(vec![1, 2, 3]),
    //         None,
    //         [1; 16],
    //     );

    //     let cache_change_seq2 = writer.new_change(
    //         ChangeKind::Alive,
    //         Some(vec![4, 5, 6]),
    //         None,
    //         [1; 16],
    //     );

    //     writer.writer_cache().add_change(cache_change_seq1);
    //     writer.writer_cache().add_change(cache_change_seq2);

    //     // let reader_proxy = writer.matched_reader_lookup(& reader_guid).unwrap();
    //     let writer_data = writer.run(&reader_guid, None).unwrap();
    //     assert_eq!(writer_data.submessages().len(), 3);
    //     if let RtpsSubmessage::InfoTs(message_1) = &writer_data.submessages()[0] {
    //         println!("{:?}", message_1);
    //     } else {
    //         panic!("Wrong message type");
    //     }
    //     if let RtpsSubmessage::Data(data_message_1) = &writer_data.submessages()[1] {
    //         assert_eq!(data_message_1.reader_id(), ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR);
    //         assert_eq!(data_message_1.writer_id(), ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER);
    //         assert_eq!(data_message_1.writer_sn(), 1);
    //         assert_eq!(data_message_1.serialized_payload(), Some(&vec![1, 2, 3]));

    //     } else {
    //         panic!("Wrong message type");
    //     };

    //     if let RtpsSubmessage::Data(data_message_2) = &writer_data.submessages()[2] {
    //         assert_eq!(data_message_2.reader_id(), ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR);
    //         assert_eq!(data_message_2.writer_id(), ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER);
    //         assert_eq!(data_message_2.writer_sn(), 2);
    //         assert_eq!(data_message_2.serialized_payload(), Some(&vec![4, 5, 6]));
    //     } else {
    //         panic!("Wrong message type");
    //     };

    //     // Test that nothing more is sent after the first time
    //     let writer_data = writer.run(&reader_guid, None);
    //     assert_eq!(writer_data.is_none(), true);
    // }
}
