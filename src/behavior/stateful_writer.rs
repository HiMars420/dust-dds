use std::time::Instant;
use std::convert::TryInto;

use crate::types::{SequenceNumber, };
use crate::messages::{Gap, Heartbeat, Endianness, AckNack, RtpsSubmessage};
use crate::structure::stateful_writer::{StatefulWriter, ReaderProxy};
use crate::messages::types::{Count, };
use crate::messages::message_sender::Sender;
use crate::messages::message_receiver::Receiver;

use super::types::Duration;
use super::data_from_cache_change;

pub struct StatefulWriterBehavior {
    heartbeat_count: Count,
    time_last_sent_data: Instant,
    time_nack_received: Instant,
    highest_nack_count_received: Count,
}

impl StatefulWriterBehavior {
    pub fn new() -> Self {
        Self {
            heartbeat_count: 0,
            time_last_sent_data: Instant::now(),
            time_nack_received: Instant::now(),
            highest_nack_count_received: 0,
        }
    }

    fn heartbeat_count(&self) -> &Count {
        &self.heartbeat_count
    }

    fn nack_received(&self) -> &Count {
        &self.highest_nack_count_received
    }

    fn nack_received_set(&mut self, highest_nack_count_received: Count) {
        self.highest_nack_count_received = highest_nack_count_received;
    }

    fn time_last_sent_data_reset(&mut self) {
        self.time_last_sent_data = Instant::now();
    }

    fn time_nack_received_reset(&mut self) {
        self.time_nack_received = Instant::now();
    }

    fn duration_since_last_sent_data(&self) -> Duration {
        self.time_last_sent_data.elapsed().try_into().unwrap()
    }

    fn duration_since_nack_received(&self) -> Duration {
        self.time_nack_received.elapsed().try_into().unwrap()
    }

    fn increment_heartbeat_count(&mut self) {
        self.heartbeat_count += 1;
    }
}

pub struct BestEffortStatefulWriterBehavior {}

impl BestEffortStatefulWriterBehavior {
    pub fn run(reader_proxy: &ReaderProxy, stateful_writer: &StatefulWriter) {
        if !reader_proxy.unsent_changes(stateful_writer.last_change_sequence_number()).is_empty() {
            Self::pushing_state(reader_proxy, stateful_writer);
        }
    }

    fn pushing_state(reader_proxy: &ReaderProxy, stateful_writer: &StatefulWriter) {
        // This state is only valid if there are unsent changes
        debug_assert!(!reader_proxy.unsent_changes(stateful_writer.last_change_sequence_number()).is_empty());
    
        while let Some(next_unsent_seq_num) = reader_proxy.next_unsent_change(stateful_writer.last_change_sequence_number()) {
            Self::transition_t4(reader_proxy, stateful_writer, next_unsent_seq_num);
        }
        reader_proxy.behavior().time_last_sent_data_reset();
    }

    fn transition_t4(reader_proxy: &ReaderProxy, stateful_writer: &StatefulWriter, next_unsent_seq_num: SequenceNumber) {
        let endianness = Endianness::LittleEndian;

        if let Some(cache_change) = stateful_writer.writer_cache()
            .changes().iter().find(|cc| cc.sequence_number() == next_unsent_seq_num)
        {
            let reader_id = reader_proxy.remote_reader_guid().entity_id();
            let data = data_from_cache_change(cache_change, endianness, reader_id);
            reader_proxy.push_send_message(RtpsSubmessage::Data(data));
        } else {
            let gap = Gap::new(
                reader_proxy.remote_reader_guid().entity_id(), 
                stateful_writer.guid().entity_id(),
                next_unsent_seq_num,
                Endianness::LittleEndian);

            reader_proxy.push_send_message(RtpsSubmessage::Gap(gap));
        }
    }
}

pub struct ReliableStatefulWriterBehavior{}

impl ReliableStatefulWriterBehavior {
    pub fn run(reader_proxy: &ReaderProxy, stateful_writer: &StatefulWriter) {
        if reader_proxy.unacked_changes(stateful_writer.last_change_sequence_number()).is_empty() {
            // Idle
        } else if !reader_proxy.unsent_changes(stateful_writer.last_change_sequence_number()).is_empty() {
            BestEffortStatefulWriterBehavior::pushing_state(reader_proxy, stateful_writer);
        } else if !reader_proxy.unacked_changes(stateful_writer.last_change_sequence_number()).is_empty() {
            Self::announcing_state(reader_proxy, stateful_writer);
        }
    
        if reader_proxy.requested_changes().is_empty() {
            Self::waiting_state(reader_proxy, stateful_writer);
        } else {
            Self::must_repair_state(reader_proxy, stateful_writer);
            if reader_proxy.behavior().duration_since_nack_received() > stateful_writer.nack_response_delay() {
                Self::repairing_state(reader_proxy, stateful_writer);
            }
        }
    }

    fn announcing_state(reader_proxy: &ReaderProxy, stateful_writer: &StatefulWriter){
        if reader_proxy.behavior().duration_since_last_sent_data() > stateful_writer.heartbeat_period() {
            Self::transition_t7(reader_proxy, stateful_writer);
            reader_proxy.behavior().time_last_sent_data_reset();
        }
    }

    fn transition_t7(reader_proxy: &ReaderProxy, stateful_writer: &StatefulWriter) {
        let first_sn = if let Some(seq_num) = stateful_writer.writer_cache().get_seq_num_min() {
            seq_num
        } else {
            stateful_writer.last_change_sequence_number() + 1
        };
        reader_proxy.behavior().increment_heartbeat_count();

        let heartbeat = Heartbeat::new(
            reader_proxy.remote_reader_guid().entity_id(),
            stateful_writer.guid().entity_id(),
            first_sn,
            stateful_writer.last_change_sequence_number(),
            *reader_proxy.behavior().heartbeat_count(),
            false,
            false,
            Endianness::LittleEndian,
        );

        reader_proxy.push_send_message(RtpsSubmessage::Heartbeat(heartbeat));
    }
    
    fn waiting_state(reader_proxy: &ReaderProxy, stateful_writer: &StatefulWriter) {
        if let Some((_, RtpsSubmessage::AckNack(acknack))) = stateful_writer.pop_receive_message(reader_proxy.remote_reader_guid()) {
            Self::transition_t8(reader_proxy, acknack);
            reader_proxy.behavior().time_nack_received_reset();
        }
    }
    
    fn transition_t8(reader_proxy: &ReaderProxy, acknack: AckNack) {
        reader_proxy.acked_changes_set(acknack.reader_sn_state().base() - 1);
        reader_proxy.requested_changes_set(acknack.reader_sn_state().set().clone());
    }
    
    fn must_repair_state(reader_proxy: &ReaderProxy, stateful_writer: &StatefulWriter) {
        if let Some((_, RtpsSubmessage::AckNack(acknack))) = stateful_writer.pop_receive_message(reader_proxy.remote_reader_guid()) {
            Self::transition_t8(reader_proxy, acknack);
        }
    }
    
    fn repairing_state(reader_proxy: &ReaderProxy, stateful_writer: &StatefulWriter) {
        // This state is only valid if there are requested changes
        debug_assert!(!reader_proxy.requested_changes().is_empty());
    
        while let Some(next_requested_seq_num) = reader_proxy.next_requested_change() {
            Self::transition_t12(reader_proxy, stateful_writer, next_requested_seq_num);
        }
    }

    fn transition_t12(reader_proxy: &ReaderProxy, stateful_writer: &StatefulWriter, next_requested_seq_num: SequenceNumber) {
        if let Some(cache_change) = stateful_writer.writer_cache()
        .changes().iter().find(|cc| cc.sequence_number() == next_requested_seq_num)
        {
            let endianness = Endianness::LittleEndian;
            let data = data_from_cache_change(cache_change, endianness, reader_proxy.remote_reader_guid().entity_id());
            reader_proxy.push_send_message(RtpsSubmessage::Data(data));
        } else {
            let gap = Gap::new(
                reader_proxy.remote_reader_guid().entity_id(), 
                stateful_writer.guid().entity_id(),
                next_requested_seq_num,
                Endianness::LittleEndian);

            reader_proxy.push_send_message(RtpsSubmessage::Gap(gap));
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::types::{ChangeKind, TopicKind, ReliabilityKind, GUID};
    // use crate::behavior::types::constants::DURATION_ZERO;
    // use crate::types::constants::{
    //     ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER, 
    //     ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR, };
    // use crate::messages::{AckNack};
    // use crate::messages::receiver::WriterReceiveMessage;
    // use crate::stateful_writer::StatefulWriter;

    // use std::collections::BTreeSet;
    // use std::thread::sleep;

    // #[test]
    // fn run_announcing_state_multiple_data_combinations() {
    //     let writer_guid = GUID::new([2;12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //     let history_cache = HistoryCache::new();

    //     let remote_reader_guid = GUID::new([1,2,3,4,5,6,7,8,9,10,11,12], ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //     let mut reader_proxy = ReaderProxy::new(remote_reader_guid, vec![], vec![], false, true);

    //     let heartbeat_period = DURATION_ZERO;
        
    //     // Test no data in the history cache and no changes written
    //     let no_change_sequence_number = 0;
    //     let submessages = StatefulWriterBehavior::run_announcing_state(&mut reader_proxy, &writer_guid, &history_cache, no_change_sequence_number, heartbeat_period).unwrap();
    //     if let RtpsSubmessage::Heartbeat(heartbeat) = &submessages[1] {
    //         assert_eq!(heartbeat.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(heartbeat.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(heartbeat.first_sn(), 1);
    //         assert_eq!(heartbeat.last_sn(), 0);
    //         assert_eq!(heartbeat.count(), 1);
    //         assert_eq!(heartbeat.is_final(), false);
    //     } else {
    //         assert!(false);
    //     }

    //     // Test no data in the history cache and two changes written
    //     let two_changes_sequence_number = 2;
    //     let submessages = StatefulWriterBehavior::run_announcing_state(&mut reader_proxy, &writer_guid, &history_cache, two_changes_sequence_number, heartbeat_period).unwrap();
    //     if let RtpsSubmessage::Heartbeat(heartbeat) = &submessages[1] {
    //         assert_eq!(heartbeat.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(heartbeat.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(heartbeat.first_sn(), 3);
    //         assert_eq!(heartbeat.last_sn(), 2);
    //         assert_eq!(heartbeat.count(), 2);
    //         assert_eq!(heartbeat.is_final(), false);
    //     } else {
    //         assert!(false);
    //     }

    //     // Test two changes in the history cache and two changes written
    //     let instance_handle = [1;16];
    //     let cache_change_seq1 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 1, Some(vec![1,2,3]), None);
    //     let cache_change_seq2 = CacheChange::new(ChangeKind::Alive, writer_guid, instance_handle, 2, Some(vec![2,3,4]), None);
    //     history_cache.add_change(cache_change_seq1);
    //     history_cache.add_change(cache_change_seq2);

    //     let submessages = StatefulWriterBehavior::run_announcing_state(&mut reader_proxy, &writer_guid, &history_cache, two_changes_sequence_number, heartbeat_period).unwrap();
    //     if let RtpsSubmessage::Heartbeat(heartbeat) = &submessages[1] {
    //         assert_eq!(heartbeat.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
    //         assert_eq!(heartbeat.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
    //         assert_eq!(heartbeat.first_sn(), 1);
    //         assert_eq!(heartbeat.last_sn(), 2);
    //         assert_eq!(heartbeat.count(), 3);
    //         assert_eq!(heartbeat.is_final(), false);
    //     } else {
    //         assert!(false);
    //     }
    // }

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