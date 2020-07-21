use std::convert::TryInto;
use std::time::Instant;

use crate::types::GUID;
use crate::behavior::types::Duration;
use crate::messages::{AckNack, Data, Gap, Heartbeat};
use crate::stateful_reader::{WriterProxy, StatefulReader};
use crate::messages::receiver::ReaderReceiveMessage;
use crate::messages::types::Count;

use crate::messages::Endianness;
use super::cache_change_from_data;

pub struct StatefulReaderBehavior {
    must_send_ack: bool,
    time_heartbeat_received: Instant,
    ackanck_count: Count,
    highest_received_heartbeat_count: Count,
}

impl StatefulReaderBehavior {
    pub fn new() -> Self {
        Self {
            must_send_ack: false,
            time_heartbeat_received: Instant::now(),
            ackanck_count: 0,
            highest_received_heartbeat_count: 0,
        }
    }

    fn must_send_ack(&self) -> bool {
        self.must_send_ack
    }

    fn set_must_send_ack(&mut self) {
        self.time_heartbeat_received  = Instant::now();
        self.must_send_ack = true;
    }

    fn reset_must_send_ack(&mut self) {
        self.must_send_ack = false;
    }

    fn duration_since_heartbeat_received(&self) -> Duration {
        self.time_heartbeat_received.elapsed().try_into().unwrap()
    }

    fn ackanck_count(&self) -> &Count {
        &self.ackanck_count
    }

    pub fn increment_acknack_count(&mut self) {
        self.ackanck_count += 1;
    }
}

pub struct BestEfforStatefulReaderBehavior {}

impl BestEfforStatefulReaderBehavior {
    pub fn run(writer_proxy: &WriterProxy, stateful_reader: &StatefulReader) {
        Self::waiting_state(writer_proxy, stateful_reader);
    }

    fn waiting_state(writer_proxy: &WriterProxy, stateful_reader: &StatefulReader) {
        if let Some(received_message) = writer_proxy.pop_received_message() {
            match received_message {
                ReaderReceiveMessage::Data(data) => Self::transition_t2(writer_proxy, stateful_reader, &data),
                ReaderReceiveMessage::Gap(gap) => Self::transition_t4(writer_proxy, &gap),
                ReaderReceiveMessage::Heartbeat(_) => (),
            }
        }
    }

    fn transition_t2(writer_proxy: &WriterProxy, stateful_reader: &StatefulReader, data: &Data) {
        let expected_seq_number = writer_proxy.available_changes_max() + 1;
        if data.writer_sn() >= expected_seq_number {
            let cache_change = cache_change_from_data(data, writer_proxy.remote_writer_guid().prefix());
            stateful_reader.reader_cache().add_change(cache_change);
            writer_proxy.received_change_set(data.writer_sn());
            writer_proxy.lost_changes_update(data.writer_sn());
        }
    }

    fn transition_t4(writer_proxy: &WriterProxy, gap: &Gap) {
        for seq_num in gap.gap_start() .. gap.gap_list().base() - 1 {
            writer_proxy.irrelevant_change_set(seq_num);
        }

        for &seq_num in gap.gap_list().set() {
            writer_proxy.irrelevant_change_set(seq_num);
        }
    }
}
pub struct ReliableStatefulReaderBehavior {}

impl ReliableStatefulReaderBehavior {
    pub fn run(writer_proxy: &WriterProxy, stateful_reader: &StatefulReader) {
        let heartbeat = Self::ready_state(writer_proxy, stateful_reader);
        if writer_proxy.behavior().must_send_ack() {
            Self::must_send_ack_state(writer_proxy, stateful_reader)
        } else {
            Self::waiting_heartbeat_state(writer_proxy, heartbeat);
        }
    }

    fn ready_state(writer_proxy: &WriterProxy, stateful_reader: &StatefulReader) -> Option<Heartbeat>{
        if let Some(received_message) = writer_proxy.pop_received_message() {
            match received_message {
                ReaderReceiveMessage::Data(data) => {
                    Self::transition_t8(writer_proxy, stateful_reader, &data);
                    None
                },
                ReaderReceiveMessage::Gap(gap) => {
                    Self::transition_t9(writer_proxy, &gap);
                    None
                },
                ReaderReceiveMessage::Heartbeat(heartbeat) => {
                    Self::transition_t7(writer_proxy, &heartbeat);
                    Some(heartbeat)
                },
            }
        } else {
            None
        }
    }

    fn transition_t8(writer_proxy: &WriterProxy, stateful_reader: &StatefulReader, data: &Data) {
        let expected_seq_number = writer_proxy.available_changes_max() + 1;
        if data.writer_sn() >= expected_seq_number {
            let cache_change = cache_change_from_data(data, writer_proxy.remote_writer_guid().prefix());
            stateful_reader.reader_cache().add_change(cache_change);
            writer_proxy.received_change_set(data.writer_sn());
        }
    }

    fn transition_t9(writer_proxy: &WriterProxy, gap: &Gap) {
        for seq_num in gap.gap_start() .. gap.gap_list().base() - 1 {
            writer_proxy.irrelevant_change_set(seq_num);
        }

        for &seq_num in gap.gap_list().set() {
            writer_proxy.irrelevant_change_set(seq_num);
        }
    }

    fn transition_t7(writer_proxy: &WriterProxy, heartbeat: &Heartbeat) {
        writer_proxy.missing_changes_update(heartbeat.last_sn());
        writer_proxy.lost_changes_update(heartbeat.first_sn());
    }

    fn waiting_heartbeat_state(writer_proxy: &WriterProxy, heartbeat_message: Option<Heartbeat>) {            
        if let Some(heartbeat) = heartbeat_message {
            writer_proxy.missing_changes_update(heartbeat.last_sn());
            writer_proxy.lost_changes_update(heartbeat.first_sn());
            if !heartbeat.is_final() || 
                (heartbeat.is_final() && !writer_proxy.missing_changes().is_empty()) {
                writer_proxy.behavior().set_must_send_ack();
            } 
        }
    }

    fn must_send_ack_state(writer_proxy: &WriterProxy, stateful_reader: &StatefulReader) {
        if writer_proxy.behavior().duration_since_heartbeat_received() >  stateful_reader.heartbeat_response_delay() {
            Self::transition_t5(writer_proxy, stateful_reader.guid())
        }
    }

    fn transition_t5(writer_proxy: &WriterProxy, reader_guid: &GUID) {
        writer_proxy.behavior().reset_must_send_ack();
 
        writer_proxy.behavior().increment_acknack_count();
        let _acknack = AckNack::new(
            *reader_guid.entity_id(), 
            *writer_proxy.remote_writer_guid().entity_id(),
            writer_proxy.available_changes_max(),
            writer_proxy.missing_changes().clone(),
            *writer_proxy.behavior().ackanck_count(),
            true,
            Endianness::LittleEndian); // TODO: Make endianness selectable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChangeKind, };
    use crate::types::constants::{
        ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, };
    use crate::cache::CacheChange;
    use crate::messages::{Data, Payload, Heartbeat, ParameterList};
    use crate::messages::Endianness;
    use crate::inline_qos_types::{KeyHash, StatusInfo, };

    // #[test]
    // fn run_best_effort_data_only() {
    //     let history_cache = HistoryCache::new();
    //     let remote_writer_guid = GUID::new([1;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

    //     let mut inline_qos = ParameterList::new();
    //     inline_qos.push(StatusInfo::from(ChangeKind::Alive));
    //     inline_qos.push(KeyHash([1;16]));

    //     let data1 = Data::new(
    //         Endianness::LittleEndian, 
    //         ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, 
    //         ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, 
    //         3,
    //         Some(inline_qos),
    //         Payload::Data(vec![1,2,3]));
    //     writer_proxy.push_received_message(ReaderReceiveMessage::Data(data1));

    //     BestEfforStatefulReaderBehavior::run(&writer_proxy, &history_cache);

    //     let expected_change_1 = CacheChange::new(
    //         ChangeKind::Alive,
    //         remote_writer_guid,
    //         [1;16],
    //         3,
    //         Some(vec![1,2,3]),
    //         None,
    //     );

    //     assert_eq!(history_cache.changes().len(), 1);
    //     assert!(history_cache.changes().contains(&expected_change_1));
    //     assert_eq!(writer_proxy.available_changes_max(), 3);

    //     // Run waiting state without any received message and verify nothing changes
    //     BestEfforStatefulReaderBehavior::run(&writer_proxy, &history_cache);
    //     assert_eq!(history_cache.changes().len(), 1);
    //     assert_eq!(writer_proxy.available_changes_max(), 3);
    // }

    // #[test]
    // fn run_reliable_data_only() {
    //     let reader_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let history_cache = HistoryCache::new();
    //     let remote_writer_guid = GUID::new([1;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

    //     let mut inline_qos = ParameterList::new();
    //     inline_qos.push(StatusInfo::from(ChangeKind::Alive));
    //     inline_qos.push(KeyHash([1;16]));

    //     let data1 = Data::new(
    //         Endianness::LittleEndian, 
    //         *reader_guid.entity_id(), 
    //         ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, 
    //         3,
    //         Some(inline_qos),
    //         Payload::Data(vec![1,2,3]));

    //     writer_proxy.push_received_message(ReaderReceiveMessage::Data(data1));

    //     ReliableStatefulReaderBehavior::run(&writer_proxy, &reader_guid, &history_cache, Duration::from_millis(300));

    //     let expected_change_1 = CacheChange::new(
    //         ChangeKind::Alive,
    //         remote_writer_guid,
    //         [1;16],
    //         3,
    //         Some(vec![1,2,3]),
    //         None,
    //     );

    //     assert_eq!(history_cache.changes().len(), 1);
    //     assert!(history_cache.changes().contains(&expected_change_1));
    //     assert_eq!(writer_proxy.available_changes_max(), 0);

    //     // Run ready state without any received message and verify nothing changes
    //     ReliableStatefulReaderBehavior::ready_state(&writer_proxy, &history_cache);
    //     assert_eq!(history_cache.changes().len(), 1);
    //     assert_eq!(writer_proxy.available_changes_max(), 0);
    // }

    // #[test]
    // fn run_reliable_non_final_heartbeat() {
    //     let reader_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let history_cache = HistoryCache::new();
    //     let remote_writer_guid = GUID::new([1;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

    //     let heartbeat = Heartbeat::new(
    //         *reader_guid.entity_id(),
    //         *remote_writer_guid.entity_id(),
    //         3,
    //         6,
    //         1,
    //         false,
    //         false,
    //         Endianness::LittleEndian,
    //     );
    
    //     writer_proxy.push_received_message(ReaderReceiveMessage::Heartbeat(heartbeat));

    //     ReliableStatefulReaderBehavior::run(&writer_proxy, &reader_guid, &history_cache, Duration::from_millis(300));
    //     assert_eq!(writer_proxy.missing_changes(), [3, 4, 5, 6].iter().cloned().collect());
    //     assert_eq!(writer_proxy.behavior().must_send_ack(), true);

    //     // TODO: Test that AckNack is sent after heartbeat_response_delay
    // }
    
    // #[test]
    // fn run_reliable_final_heartbeat_with_missing_changes() {
    //     let reader_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let history_cache = HistoryCache::new();
    //     let remote_writer_guid = GUID::new([1;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

    //     let heartbeat = Heartbeat::new(
    //         ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
    //         *remote_writer_guid.entity_id(),
    //         2,
    //         3,
    //         1,
    //         true,
    //         false,
    //         Endianness::LittleEndian,
    //     );
    //     writer_proxy.push_received_message(ReaderReceiveMessage::Heartbeat(heartbeat));

    //     let heartbeat_response_delay = Duration::from_millis(300);
    //     ReliableStatefulReaderBehavior::run(&writer_proxy, &reader_guid, &history_cache, heartbeat_response_delay);
    //     assert_eq!(writer_proxy.missing_changes(), [2, 3].iter().cloned().collect());
    //     assert_eq!(writer_proxy.behavior().must_send_ack(), true);

    //     std::thread::sleep(heartbeat_response_delay.into());

    //     ReliableStatefulReaderBehavior::run(&writer_proxy, &reader_guid, &history_cache, heartbeat_response_delay);
    //     assert_eq!(writer_proxy.behavior().must_send_ack(), false);

    //     // TODO: Test that AckNack is sent after duration
    // }

    // #[test]
    // fn run_reliable_final_heartbeat_without_missing_changes() {
    //     let reader_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let history_cache = HistoryCache::new();
    //     let remote_writer_guid = GUID::new([1;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

    //     let heartbeat = Heartbeat::new(
    //         ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
    //         *remote_writer_guid.entity_id(),
    //         1,
    //         0,
    //         1,
    //         true,
    //         false,
    //         Endianness::LittleEndian,
    //     );
    //     writer_proxy.push_received_message(ReaderReceiveMessage::Heartbeat(heartbeat));

    //     ReliableStatefulReaderBehavior::run(&writer_proxy, &reader_guid, &history_cache, Duration::from_millis(300));
    //     assert_eq!(writer_proxy.missing_changes(), [].iter().cloned().collect());
    //     assert_eq!(writer_proxy.behavior().must_send_ack(), false);
    // }
}