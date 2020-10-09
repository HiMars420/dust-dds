use std::convert::TryInto;
use std::time::Instant;
use std::collections::VecDeque;
use std::sync::Mutex;

use crate::types::{GuidPrefix, EntityId};
use crate::types::constants::LOCATOR_INVALID;
use crate::structure::HistoryCache;
use crate::behavior::WriterProxy;
use crate::messages::RtpsSubmessage;
use crate::messages::submessages::{AckNack, Data, Gap, Heartbeat,};
use crate::messages::types::Count;
use crate::messages::message_sender::Sender;

use super::types::Duration;
use super::{cache_change_from_data, BEHAVIOR_ENDIANNESS};

pub struct ReliableWriterProxy {
    writer_proxy: WriterProxy,
    reader_entity_id: EntityId,
    heartbeat_response_delay: Duration,

    must_send_ack: bool,
    time_heartbeat_received: Instant,
    ackanck_count: Count,
    highest_received_heartbeat_count: Count,

    received_messages: Mutex<VecDeque<(GuidPrefix, RtpsSubmessage)>>,
    send_messages: Mutex<VecDeque<RtpsSubmessage>>
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
            send_messages: Mutex::new(VecDeque::new()),
        }
    }

    pub fn run(&mut self, history_cache: &HistoryCache) {
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

    fn ready_state(&self, history_cache: &HistoryCache) -> Option<Heartbeat>{
        if let Some((_, received_message)) = self.received_messages.lock().unwrap().pop_front() {
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

    fn transition_t8(&self, history_cache: &HistoryCache, data: Data) {
        let expected_seq_number = self.writer_proxy.available_changes_max() + 1;
        if data.writer_sn() >= expected_seq_number {
            self.writer_proxy.received_change_set(data.writer_sn());
            let cache_change = cache_change_from_data(data, &self.writer_proxy.remote_writer_guid().prefix());
            history_cache.add_change(cache_change).unwrap();
            
        }
    }

    fn transition_t9(&self, gap: &Gap) {
        for seq_num in gap.gap_start() .. gap.gap_list().base() - 1 {
            self.writer_proxy.irrelevant_change_set(seq_num);
        }

        for &seq_num in gap.gap_list().set() {
            self.writer_proxy.irrelevant_change_set(seq_num);
        }
    }

    fn transition_t7(&self, heartbeat: &Heartbeat) {
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

        self.send_messages.lock().unwrap().push_back(RtpsSubmessage::AckNack(acknack));
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::types::{ChangeKind, TopicKind, GUID};
//     use crate::types::constants::{
//         ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, };
//     use crate::structure::CacheChange;
//     use crate::messages::submessages::data_submessage::Payload;
//     use crate::serialized_payload::ParameterList;
//     use crate::inline_qos_types::KeyHash;
//     use crate::messages::Endianness;
//     use super::super::change_kind_to_status_info;

//     use rust_dds_interface::qos::DataReaderQos;
//     use rust_dds_interface::qos_policy::ReliabilityQosPolicyKind;

//     #[test]
//     fn run_best_effort_data_only() {
//         let reader_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
//         let mut reader_qos = DataReaderQos::default();
//         reader_qos.reliability.kind = ReliabilityQosPolicyKind::BestEffortReliabilityQos;

//         let stateful_reader = StatefulReader::new(
//             reader_guid,
//             TopicKind::WithKey,
//             &reader_qos);

//         let remote_writer_guid_prefix = [1;12];
//         let remote_writer_guid = GUID::new(remote_writer_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
//         let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);
//         stateful_reader.matched_writer_add(writer_proxy);

//         let mut inline_qos = ParameterList::new();
//         inline_qos.push(change_kind_to_status_info(ChangeKind::Alive));
//         inline_qos.push(KeyHash([1;16]));

//         let data1 = Data::new(
//             Endianness::LittleEndian,
//             ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, 
//             ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, 
//             3,
//             Some(inline_qos),
//             Payload::Data(vec![1,2,3]));
//         stateful_reader.push_receive_message(remote_writer_guid_prefix, RtpsSubmessage::Data(data1));

//         let matched_writers = stateful_reader.matched_writers();
//         let writer_proxy = matched_writers.get(&remote_writer_guid).unwrap();
//         BestEfforStatefulReaderBehavior::run(writer_proxy, &stateful_reader);

//         let expected_change_1 = CacheChange::new(
//             ChangeKind::Alive,
//             remote_writer_guid,
//             [1;16],
//             3,
//             Some(vec![1,2,3]),
//             None,
//         );

//         assert_eq!(stateful_reader.reader_cache().changes().len(), 1);
//         assert!(stateful_reader.reader_cache().changes().contains(&expected_change_1));
//         assert_eq!(writer_proxy.available_changes_max(), 3);

//         // Run waiting state without any received message and verify nothing changes
//         BestEfforStatefulReaderBehavior::run(&writer_proxy, &stateful_reader);
//         assert_eq!(stateful_reader.reader_cache().changes().len(), 1);
//         assert_eq!(writer_proxy.available_changes_max(), 3);
//     }

//     #[test]
//     fn run_reliable_data_only() {
//         let reader_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
//         let mut reader_qos = DataReaderQos::default();
//         reader_qos.reliability.kind = ReliabilityQosPolicyKind::ReliableReliabilityQos;

//         let stateful_reader = StatefulReader::new(
//             reader_guid,
//             TopicKind::WithKey,
//             &reader_qos);

//         let remote_writer_guid_prefix = [1;12];
//         let remote_writer_guid = GUID::new(remote_writer_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
//         let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);
//         stateful_reader.matched_writer_add(writer_proxy);

//         let mut inline_qos = ParameterList::new();
//         inline_qos.push(change_kind_to_status_info(ChangeKind::Alive));
//         inline_qos.push(KeyHash([1;16]));

//         let data1 = Data::new(
//             Endianness::LittleEndian,
//             reader_guid.entity_id(), 
//             ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, 
//             3,
//             Some(inline_qos),
//             Payload::Data(vec![1,2,3]));

//         stateful_reader.push_receive_message(remote_writer_guid_prefix, RtpsSubmessage::Data(data1));

//         let matched_writers = stateful_reader.matched_writers();
//         let writer_proxy = matched_writers.get(&remote_writer_guid).unwrap();
//         ReliableStatefulReaderBehavior::run(writer_proxy, &stateful_reader);

//         let expected_change_1 = CacheChange::new(
//             ChangeKind::Alive,
//             remote_writer_guid,
//             [1;16],
//             3,
//             Some(vec![1,2,3]),
//             None,
//         );

//         assert_eq!(stateful_reader.reader_cache().changes().len(), 1);
//         assert!(stateful_reader.reader_cache().changes().contains(&expected_change_1));
//         assert_eq!(writer_proxy.available_changes_max(), 0);

//         // Run ready state without any received message and verify nothing changes
//         let matched_writers = stateful_reader.matched_writers();
//         let writer_proxy = matched_writers.get(&remote_writer_guid).unwrap();
//         ReliableStatefulReaderBehavior::ready_state(writer_proxy, &stateful_reader);
//         assert_eq!(stateful_reader.reader_cache().changes().len(), 1);
//         assert_eq!(writer_proxy.available_changes_max(), 0);
//     }

//     #[test]
//     fn run_reliable_non_final_heartbeat() {
//         let reader_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
//         let mut reader_qos = DataReaderQos::default();
//         reader_qos.reliability.kind = ReliabilityQosPolicyKind::ReliableReliabilityQos;

//         let stateful_reader = StatefulReader::new(
//             reader_guid,
//             TopicKind::WithKey,
//             &reader_qos);

//         let remote_writer_guid_prefix = [1;12];
//         let remote_writer_guid = GUID::new(remote_writer_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
//         let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);
//         stateful_reader.matched_writer_add(writer_proxy);

//         let heartbeat = Heartbeat::new(
//             Endianness::LittleEndian,
//             reader_guid.entity_id(),
//             remote_writer_guid.entity_id(),
//             3,
//             6,
//             1,
//             false,
//             false,
//         );
    
//         stateful_reader.push_receive_message(remote_writer_guid_prefix, RtpsSubmessage::Heartbeat(heartbeat));

//         let matched_writers = stateful_reader.matched_writers();
//         let writer_proxy = matched_writers.get(&remote_writer_guid).unwrap();
//         ReliableStatefulReaderBehavior::run(writer_proxy, &stateful_reader);
//         assert_eq!(writer_proxy.missing_changes(), [3, 4, 5, 6].iter().cloned().collect());
//         assert_eq!(writer_proxy.behavior().must_send_ack(), true);
//     }
    
//     #[test]
//     fn run_reliable_final_heartbeat_with_missing_changes() {
//         let reader_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
//         let mut reader_qos = DataReaderQos::default();
//         reader_qos.reliability.kind = ReliabilityQosPolicyKind::ReliableReliabilityQos;

//         let stateful_reader = StatefulReader::new(
//             reader_guid,
//             TopicKind::WithKey,
//             &reader_qos);

//         let remote_writer_guid_prefix = [1;12];
//         let remote_writer_guid = GUID::new(remote_writer_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
//         let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);
//         stateful_reader.matched_writer_add(writer_proxy);

//         let heartbeat = Heartbeat::new(
//             Endianness::LittleEndian,
//             ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
//             remote_writer_guid.entity_id(),
//             2,
//             3,
//             1,
//             true,
//             false,
//         );
//         stateful_reader.push_receive_message(remote_writer_guid_prefix, RtpsSubmessage::Heartbeat(heartbeat));

//         let matched_writers = stateful_reader.matched_writers();
//         let writer_proxy = matched_writers.get(&remote_writer_guid).unwrap();

//         let heartbeat_response_delay = Duration::from_millis(300);
//         ReliableStatefulReaderBehavior::run(writer_proxy, &stateful_reader);
//         assert_eq!(writer_proxy.missing_changes(), [2, 3].iter().cloned().collect());
//         assert_eq!(writer_proxy.behavior().must_send_ack(), true);

//         std::thread::sleep(heartbeat_response_delay.into());

        
//         ReliableStatefulReaderBehavior::run(writer_proxy, &stateful_reader);
//         assert_eq!(writer_proxy.behavior().must_send_ack(), false);

//         // TODO: Test that AckNack is sent after duration
//     }

//     #[test]
//     fn run_reliable_final_heartbeat_without_missing_changes() {
//         let reader_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
//         let mut reader_qos = DataReaderQos::default();
//         reader_qos.reliability.kind = ReliabilityQosPolicyKind::ReliableReliabilityQos;

//         let stateful_reader = StatefulReader::new(
//             reader_guid,
//             TopicKind::WithKey,
//             &reader_qos);

//         let remote_writer_guid_prefix = [1;12];
//         let remote_writer_guid = GUID::new(remote_writer_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
//         let writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);
//         stateful_reader.matched_writer_add(writer_proxy);

//         let heartbeat = Heartbeat::new(
//             Endianness::LittleEndian,
//             ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
//             remote_writer_guid.entity_id(),
//             1,
//             0,
//             1,
//             true,
//             false,
//         );
//         stateful_reader.push_receive_message(remote_writer_guid_prefix, RtpsSubmessage::Heartbeat(heartbeat));

//         let matched_writers = stateful_reader.matched_writers();
//         let writer_proxy = matched_writers.get(&remote_writer_guid).unwrap();
//         ReliableStatefulReaderBehavior::run(writer_proxy, &stateful_reader);
//         assert_eq!(writer_proxy.missing_changes(), [].iter().cloned().collect());
//         assert_eq!(writer_proxy.behavior().must_send_ack(), false);
//     }
// }