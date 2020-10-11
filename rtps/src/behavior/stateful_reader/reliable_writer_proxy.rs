use std::convert::TryInto;
use std::time::Instant;
use std::collections::VecDeque;
use std::sync::Mutex;

use crate::types::{GuidPrefix, GUID, EntityId, Locator};
use crate::structure::HistoryCache;
use crate::behavior::WriterProxy;
use crate::messages::RtpsSubmessage;
use crate::messages::submessages::{AckNack, Data, Gap, Heartbeat,};
use crate::messages::types::Count;
use crate::messages::message_sender::Sender;

use crate::behavior::types::Duration;
use crate::behavior::{cache_change_from_data, BEHAVIOR_ENDIANNESS};
use super::stateful_reader::WriterProxyOps;

pub struct ReliableWriterProxy {
    writer_proxy: WriterProxy,
    reader_entity_id: EntityId,
    heartbeat_response_delay: Duration,

    must_send_ack: bool,
    time_heartbeat_received: Instant,
    ackanck_count: Count,
    highest_received_heartbeat_count: Count,

    received_messages: Mutex<VecDeque<(GuidPrefix, RtpsSubmessage)>>,
    sent_messages: Mutex<VecDeque<RtpsSubmessage>>
}

impl ReliableWriterProxy {
    pub fn new(writer_proxy: WriterProxy, reader_entity_id: EntityId, heartbeat_response_delay: Duration,) -> Self {
        Self {
            writer_proxy,
            reader_entity_id,
            heartbeat_response_delay,
            must_send_ack: false,
            time_heartbeat_received: Instant::now(),
            ackanck_count: 0,
            highest_received_heartbeat_count: 0,
            received_messages: Mutex::new(VecDeque::new()),
            sent_messages: Mutex::new(VecDeque::new()),
        }
    }

    fn ready_state(&mut self, history_cache: &HistoryCache) -> Option<Heartbeat>{
        let received = self.received_messages.lock().unwrap().pop_front();
        if let Some((_, received_message)) = received {
            match received_message {
                RtpsSubmessage::Data(data) => {
                    self.transition_t8(history_cache, data);
                    None
                },
                RtpsSubmessage::Gap(gap) => {
                    self.transition_t9(&gap);
                    None
                },
                RtpsSubmessage::Heartbeat(heartbeat) => {
                    self.transition_t7(&heartbeat);
                    Some(heartbeat)
                },
                _ => panic!("Unexpected reader message received"),
            }
        } else {
            None
        }
    }

    fn transition_t8(&mut self, history_cache: &HistoryCache, data: Data) {
        let expected_seq_number = self.writer_proxy.available_changes_max() + 1;
        if data.writer_sn() >= expected_seq_number {
            self.writer_proxy.received_change_set(data.writer_sn());
            let cache_change = cache_change_from_data(data, &self.writer_proxy.remote_writer_guid().prefix());
            history_cache.add_change(cache_change).unwrap();
            
        }
    }

    fn transition_t9(&mut self, gap: &Gap) {
        for seq_num in gap.gap_start() .. gap.gap_list().base() - 1 {
            self.writer_proxy.irrelevant_change_set(seq_num);
        }

        for &seq_num in gap.gap_list().set() {
            self.writer_proxy.irrelevant_change_set(seq_num);
        }
    }

    fn transition_t7(&mut self, heartbeat: &Heartbeat) {
        self.writer_proxy.missing_changes_update(heartbeat.last_sn());
        self.writer_proxy.lost_changes_update(heartbeat.first_sn());
    }

    fn waiting_heartbeat_state(&mut self, heartbeat_message: Option<Heartbeat>) {            
        if let Some(heartbeat) = heartbeat_message {
            if !heartbeat.is_final() || 
                (heartbeat.is_final() && !self.writer_proxy.missing_changes().is_empty()) {
                    self.set_must_send_ack();
            } 
        }
    }

    fn must_send_ack_state(&mut self) {
        if self.duration_since_heartbeat_received() >  self.heartbeat_response_delay {
            self.transition_t5()
        }
    }

    fn transition_t5(&mut self) {
        self.reset_must_send_ack();
 
        self.increment_acknack_count();
        let acknack = AckNack::new(
            BEHAVIOR_ENDIANNESS,
            self.reader_entity_id, 
            self.writer_proxy.remote_writer_guid().entity_id(),
            self.writer_proxy.available_changes_max(),
            self.writer_proxy.missing_changes().clone(),
            *self.ackanck_count(),
            true);

        self.sent_messages.lock().unwrap().push_back(RtpsSubmessage::AckNack(acknack));
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

impl WriterProxyOps for ReliableWriterProxy {
    fn run(&mut self, history_cache: &HistoryCache) {
        // The heartbeat message triggers also a transition in the parallel state-machine
        // relating to the acknack sending so it is returned from the ready_state for
        // further processing.
        let heartbeat = self.ready_state(history_cache);
        if self.must_send_ack {
            self.must_send_ack_state()
        } else {
            self.waiting_heartbeat_state(heartbeat);
        }
    }

    fn push_receive_message(&self, src_guid_prefix: GuidPrefix, submessage: RtpsSubmessage) {
        assert!(self.is_submessage_destination(&src_guid_prefix, &submessage));

        self.received_messages.lock().unwrap().push_back((src_guid_prefix, submessage));
    }

    fn is_submessage_destination(&self, src_guid_prefix: &GuidPrefix, submessage: &RtpsSubmessage) -> bool {
        let writer_id = match submessage {
            RtpsSubmessage::Data(data) => data.writer_id(),
            RtpsSubmessage::Gap(gap) => gap.writer_id(),
            RtpsSubmessage::Heartbeat(heartbeat) => heartbeat.writer_id(),
            _ => return false,
        };

        let writer_guid = GUID::new(*src_guid_prefix, writer_id);

        self.writer_proxy.remote_writer_guid() == &writer_guid
    }
}

impl Sender for ReliableWriterProxy {
    fn pop_send_messages(&self) -> Vec<(Vec<Locator>, VecDeque<RtpsSubmessage>)> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChangeKind, GUID};
    use crate::types::constants::{
        ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR};
    use crate::structure::CacheChange;
    use crate::messages::submessages::data_submessage::Payload;
    use crate::serialized_payload::ParameterList;
    use crate::inline_qos_types::KeyHash;
    use crate::messages::Endianness;
    use crate::behavior::change_kind_to_status_info;

    use rust_dds_interface::qos_policy::ResourceLimitsQosPolicy;

    #[test]
    fn run_reliable_data_only() {        
        let history_cache = HistoryCache::new(&ResourceLimitsQosPolicy::default());
        let heartbeat_response_delay = Duration::from_millis(500);
        let reader_entity_id = ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR;
        
        let remote_writer_guid_prefix = [1;12];
        let remote_writer_guid = GUID::new(remote_writer_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);
        let mut reliable_writer_proxy = ReliableWriterProxy::new(writer_proxy, reader_entity_id, heartbeat_response_delay);

        let mut inline_qos = ParameterList::new();
        inline_qos.push(change_kind_to_status_info(ChangeKind::Alive));
        inline_qos.push(KeyHash([1;16]));

        let data1 = Data::new(
            Endianness::LittleEndian,
            reader_entity_id, 
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, 
            3,
            Some(inline_qos),
            Payload::Data(vec![1,2,3]));

        reliable_writer_proxy.push_receive_message(remote_writer_guid_prefix, RtpsSubmessage::Data(data1));


        reliable_writer_proxy.run(&history_cache);

        let expected_change_1 = CacheChange::new(
            ChangeKind::Alive,
            remote_writer_guid,
            [1;16],
            3,
            Some(vec![1,2,3]),
            None,
        );

        assert_eq!(history_cache.changes().len(), 1);
        assert!(history_cache.changes().contains(&expected_change_1));
        assert_eq!(reliable_writer_proxy.writer_proxy.available_changes_max(), 0);

        // Run without any received message and verify nothing changes
        reliable_writer_proxy.run(&history_cache);
        assert_eq!(history_cache.changes().len(), 1);
        assert_eq!(reliable_writer_proxy.writer_proxy.available_changes_max(), 0);
    }

    #[test]
    fn run_reliable_non_final_heartbeat() {
        let history_cache = HistoryCache::new(&ResourceLimitsQosPolicy::default());
        let heartbeat_response_delay = Duration::from_millis(500);
        let reader_entity_id = ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR;
        
        let remote_writer_guid_prefix = [1;12];
        let remote_writer_guid = GUID::new(remote_writer_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);
        let mut reliable_writer_proxy = ReliableWriterProxy::new(writer_proxy, reader_entity_id, heartbeat_response_delay);

        let heartbeat = Heartbeat::new(
            Endianness::LittleEndian,
            reader_entity_id,
            remote_writer_guid.entity_id(),
            3,
            6,
            1,
            false,
            false,
        );
    
        reliable_writer_proxy.push_receive_message(remote_writer_guid_prefix, RtpsSubmessage::Heartbeat(heartbeat));

        reliable_writer_proxy.run(&history_cache);
        assert_eq!(reliable_writer_proxy.writer_proxy.missing_changes(), [3, 4, 5, 6].iter().cloned().collect());
        assert_eq!(reliable_writer_proxy.must_send_ack(), true);
    }
    
    #[test]
    fn run_reliable_final_heartbeat_with_missing_changes() {
        let history_cache = HistoryCache::new(&ResourceLimitsQosPolicy::default());
        let heartbeat_response_delay = Duration::from_millis(300);
        let reader_entity_id = ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR;
        
        let remote_writer_guid_prefix = [1;12];
        let remote_writer_guid = GUID::new(remote_writer_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);
        let mut reliable_writer_proxy = ReliableWriterProxy::new(writer_proxy, reader_entity_id, heartbeat_response_delay);

        let heartbeat = Heartbeat::new(
            Endianness::LittleEndian,
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
            remote_writer_guid.entity_id(),
            2,
            3,
            1,
            true,
            false,
        );
        reliable_writer_proxy.push_receive_message(remote_writer_guid_prefix, RtpsSubmessage::Heartbeat(heartbeat));

        reliable_writer_proxy.run(&history_cache);
        assert_eq!(reliable_writer_proxy.writer_proxy.missing_changes(), [2, 3].iter().cloned().collect());
        assert_eq!(reliable_writer_proxy.must_send_ack(), true);

        std::thread::sleep(heartbeat_response_delay.into());

        reliable_writer_proxy.run(&history_cache);
        assert_eq!(reliable_writer_proxy.must_send_ack(), false);

        // TODO: Test that AckNack is sent after duration
    }

    #[test]
    fn run_reliable_final_heartbeat_without_missing_changes() {
        let history_cache = HistoryCache::new(&ResourceLimitsQosPolicy::default());
        let heartbeat_response_delay = Duration::from_millis(500);
        let reader_entity_id = ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR;
        
        let remote_writer_guid_prefix = [1;12];
        let remote_writer_guid = GUID::new(remote_writer_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);
        let mut reliable_writer_proxy = ReliableWriterProxy::new(writer_proxy, reader_entity_id, heartbeat_response_delay);

        let heartbeat = Heartbeat::new(
            Endianness::LittleEndian,
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
            remote_writer_guid.entity_id(),
            1,
            0,
            1,
            true,
            false,
        );
        reliable_writer_proxy.push_receive_message(remote_writer_guid_prefix, RtpsSubmessage::Heartbeat(heartbeat));

        reliable_writer_proxy.run(&history_cache);
        assert_eq!(reliable_writer_proxy.writer_proxy.missing_changes(), [].iter().cloned().collect());
        assert_eq!(reliable_writer_proxy.must_send_ack, false);
    }
}