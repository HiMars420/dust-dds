use std::time::Instant;
use std::convert::{TryFrom, TryInto};

use crate::types::{GUID, SequenceNumber};
use crate::behavior::types::Duration;
use crate::messages::{RtpsMessage, RtpsSubmessage, AckNack};
use crate::messages::submessage_elements::SequenceNumberSet;
use crate::messages::types::Count;
use crate::cache::{HistoryCache};
use crate::stateful_reader::WriterProxy;
use crate::inline_qos_types::{KeyHash, StatusInfo, };
use crate::serdes::Endianness;
use super::cache_change_from_data;

pub struct StatefulReaderBehaviour {
    must_send_ack: bool,
    time_heartbeat_received: Instant,
    ackanck_count: Count,
    highest_received_heartbeat_count: Count,
}

impl StatefulReaderBehaviour {
    pub fn new() -> Self {
        Self {
            must_send_ack: false,
            time_heartbeat_received: Instant::now(),
            ackanck_count: Count(0),
            highest_received_heartbeat_count: Count(0),
        }
    }

    pub fn run_best_effort(&mut self, writer_proxy: &mut WriterProxy, _reader_guid: &GUID, history_cache: &mut HistoryCache, received_message: Option<&RtpsMessage>) -> Option<Vec<RtpsSubmessage>> {
        StatefulReaderBehaviour::run_waiting_state(writer_proxy, history_cache, received_message);
        None
    }

    pub fn run_reliable(&mut self, writer_proxy: &mut WriterProxy, reader_guid: &GUID, history_cache: &mut HistoryCache, heartbeat_response_delay: Duration, received_message: Option<&RtpsMessage>) -> Option<Vec<RtpsSubmessage>>{
        let must_send_ack = false;
        StatefulReaderBehaviour::run_ready_state(writer_proxy, history_cache, received_message);
        if must_send_ack {
            // This is the only case in which a message is sent by the stateful reader
            self.run_must_send_ack_state(writer_proxy, reader_guid, heartbeat_response_delay)
        } else {
            self.run_waiting_heartbeat_state(writer_proxy, received_message);
            None
        }
    }

    fn run_waiting_state(writer_proxy: &mut WriterProxy, history_cache: &mut HistoryCache, received_message: Option<&RtpsMessage>) {
        if let Some(received_message) = received_message {
            let guid_prefix = received_message.header().guid_prefix();
            for submessage in received_message.submessages().iter() {                
                if let RtpsSubmessage::Data(data) = submessage {
                    let expected_seq_number = writer_proxy.available_changes_max() + 1;
                    if data.writer_sn() >= &expected_seq_number {
                        let cache_change = cache_change_from_data(data, guid_prefix);
                        history_cache.add_change(cache_change);
                        writer_proxy.received_change_set(*data.writer_sn());
                        writer_proxy.lost_changes_update(*data.writer_sn());
                    }
                } else if let RtpsSubmessage::Gap(_gap) = submessage {
                    let _expected_seq_number = writer_proxy.available_changes_max() + 1;
                    todo!()
                }
            }
        }
    }

    fn run_ready_state(writer_proxy: &mut WriterProxy, history_cache: &mut HistoryCache, received_message: Option<&RtpsMessage>) {
        if let Some(received_message) = received_message {
            let guid_prefix = received_message.header().guid_prefix();
            for submessage in received_message.submessages().iter() {                
                if let RtpsSubmessage::Data(data) = submessage {
                    let expected_seq_number = writer_proxy.available_changes_max() + 1;
                    if data.writer_sn() >= &expected_seq_number {
                        let cache_change = cache_change_from_data(data, guid_prefix);
                        history_cache.add_change(cache_change);
                        writer_proxy.received_change_set(*data.writer_sn());
                    }
                } else if let RtpsSubmessage::Gap(_gap) = submessage {
                    let _expected_seq_number = writer_proxy.available_changes_max() + 1;
                    todo!()
                } 
                // The heartbeat reception is moved to the waiting state since it has to be read there anyway
            }
        }
    }

    fn run_waiting_heartbeat_state(&mut self, writer_proxy: &mut WriterProxy, received_message: Option<&RtpsMessage>) {
        if let Some(received_message) = received_message {
            let guid_prefix = received_message.header().guid_prefix();
            for submessage in received_message.submessages().iter() {                
                if let RtpsSubmessage::Heartbeat(heartbeat) = submessage {
                    writer_proxy.missing_changes_update(*heartbeat.last_sn());
                    writer_proxy.lost_changes_update(*heartbeat.first_sn());
                    if !heartbeat.is_final() || 
                        (heartbeat.is_final() && !writer_proxy.missing_changes().is_empty()) {
                        self.must_send_ack = true;
                        self.time_heartbeat_received = Instant::now();
                    } 
                }
            }
        }
    }

    fn run_must_send_ack_state(&mut self, writer_proxy: &mut WriterProxy, reader_guid: &GUID, heartbeat_response_delay: Duration) -> Option<Vec<RtpsSubmessage>> {
        if Duration::try_from(self.time_heartbeat_received.elapsed()).unwrap() >  heartbeat_response_delay {
            self.must_send_ack = false;
            let reader_sn_state = SequenceNumberSet::new(
                writer_proxy.available_changes_max(),
                writer_proxy.missing_changes().clone()
            );
            self.ackanck_count += 1;
            let acknack = AckNack::new(
                *reader_guid.entity_id(), 
                *writer_proxy.remote_writer_guid().entity_id(),
                reader_sn_state,
                self.ackanck_count,
                true,
                Endianness::LittleEndian);

            Some(vec![RtpsSubmessage::AckNack(acknack)])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SequenceNumber, ChangeKind, GuidPrefix, TopicKind, ReliabilityKind, Locator};
    use crate::behavior::types::constants::DURATION_ZERO;
    use crate::messages::types::Count;
    use crate::types::constants::{
        ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER, 
        ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR, };
    use crate::cache::CacheChange;
    use crate::messages::{Data, Payload, Heartbeat};
    use crate::messages::submessage_elements::{SequenceNumberSet, Parameter, ParameterList};
    use crate::serdes::Endianness;
    use crate::stateful_writer::StatefulWriter;
    use crate::serialized_payload::SerializedPayload;
    use std::thread::sleep;

    #[test]
    fn run_waiting_state_data_only() {
        let mut history_cache = HistoryCache::new();
        let remote_writer_guid = GUID::new(GuidPrefix([1;12]), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let mut writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

        let mut submessages = Vec::new();
        let inline_qos_parameters = vec![
            Parameter::new(StatusInfo::from(ChangeKind::Alive), Endianness::LittleEndian),
            Parameter::new(KeyHash([1;16]), Endianness::LittleEndian)];

        let data1 = Data::new(
            Endianness::LittleEndian, 
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, 
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, 
            SequenceNumber(3),
            Some(ParameterList::new(inline_qos_parameters)),
            Payload::Data(SerializedPayload(vec![1,2,3])));
        submessages.push(RtpsSubmessage::Data(data1));

        let received_message = RtpsMessage::new(*remote_writer_guid.prefix(), submessages);

        StatefulReaderBehaviour::run_waiting_state(&mut writer_proxy, &mut history_cache, Some(&received_message));

        let expected_change_1 = CacheChange::new(
            ChangeKind::Alive,
            remote_writer_guid,
            [1;16],
            SequenceNumber(3),
            None,
            Some(vec![1,2,3]),
        );

        assert_eq!(history_cache.get_changes().len(), 1);
        assert!(history_cache.get_changes().contains(&expected_change_1));
        assert_eq!(writer_proxy.available_changes_max(), SequenceNumber(3));

        // Run waiting state without any received message and verify nothing changes
        StatefulReaderBehaviour::run_waiting_state(&mut writer_proxy, &mut history_cache, None);
        assert_eq!(history_cache.get_changes().len(), 1);
        assert_eq!(writer_proxy.available_changes_max(), SequenceNumber(3));
    }

    #[test]
    fn run_ready_state_data_only() {
        let mut history_cache = HistoryCache::new();
        let remote_writer_guid = GUID::new(GuidPrefix([1;12]), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let mut writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

        let mut submessages = Vec::new();
        let inline_qos_parameters = vec![
            Parameter::new(StatusInfo::from(ChangeKind::Alive), Endianness::LittleEndian),
            Parameter::new(KeyHash([1;16]), Endianness::LittleEndian)];

        let data1 = Data::new(
            Endianness::LittleEndian, 
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, 
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER, 
            SequenceNumber(3),
            Some(ParameterList::new(inline_qos_parameters)),
            Payload::Data(SerializedPayload(vec![1,2,3])));
        submessages.push(RtpsSubmessage::Data(data1));

        let received_message = RtpsMessage::new(*remote_writer_guid.prefix(), submessages);

        StatefulReaderBehaviour::run_ready_state(&mut writer_proxy, &mut history_cache, Some(&received_message));

        let expected_change_1 = CacheChange::new(
            ChangeKind::Alive,
            remote_writer_guid,
            [1;16],
            SequenceNumber(3),
            None,
            Some(vec![1,2,3]),
        );

        assert_eq!(history_cache.get_changes().len(), 1);
        assert!(history_cache.get_changes().contains(&expected_change_1));
        assert_eq!(writer_proxy.available_changes_max(), SequenceNumber(0));

        // Run waiting state without any received message and verify nothing changes
        StatefulReaderBehaviour::run_waiting_state(&mut writer_proxy, &mut history_cache, None);
        assert_eq!(history_cache.get_changes().len(), 1);
        assert_eq!(writer_proxy.available_changes_max(), SequenceNumber(0));
    }

    #[test]
    fn run_waiting_heartbeat_state_non_final_heartbeat() {
        let mut stateful_reader_behaviour = StatefulReaderBehaviour::new();

        let remote_writer_guid = GUID::new(GuidPrefix([1;12]), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let mut writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

        let mut submessages = Vec::new();
        let heartbeat = Heartbeat::new(
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
            *remote_writer_guid.entity_id(),
            SequenceNumber(3),
            SequenceNumber(6),
            Count(1),
            false,
            false,
            Endianness::LittleEndian,
        );
        submessages.push(RtpsSubmessage::Heartbeat(heartbeat));
        let received_message = RtpsMessage::new(*remote_writer_guid.prefix(), submessages);       

        stateful_reader_behaviour.run_waiting_heartbeat_state(&mut writer_proxy, Some(&received_message));
        assert_eq!(writer_proxy.missing_changes(), &[SequenceNumber(3), SequenceNumber(4), SequenceNumber(5), SequenceNumber(6)].iter().cloned().collect());
        assert_eq!(stateful_reader_behaviour.must_send_ack, true);
    }
    
    #[test]
    fn run_waiting_heartbeat_state_final_heartbeat_with_missing_changes() {
        let mut stateful_reader_behaviour = StatefulReaderBehaviour::new();

        let remote_writer_guid = GUID::new(GuidPrefix([1;12]), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let mut writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

        let mut submessages = Vec::new();
        let heartbeat = Heartbeat::new(
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
            *remote_writer_guid.entity_id(),
            SequenceNumber(2),
            SequenceNumber(3),
            Count(1),
            true,
            false,
            Endianness::LittleEndian,
        );
        submessages.push(RtpsSubmessage::Heartbeat(heartbeat));
        let received_message = RtpsMessage::new(*remote_writer_guid.prefix(), submessages);       

        stateful_reader_behaviour.run_waiting_heartbeat_state(&mut writer_proxy, Some(&received_message));
        assert_eq!(writer_proxy.missing_changes(), &[SequenceNumber(2), SequenceNumber(3)].iter().cloned().collect());
        assert_eq!(stateful_reader_behaviour.must_send_ack, true);
    }

    #[test]
    fn run_waiting_heartbeat_state_final_heartbeat_without_missing_changes() {
        let mut stateful_reader_behaviour = StatefulReaderBehaviour::new();

        let remote_writer_guid = GUID::new(GuidPrefix([1;12]), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let mut writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

        let mut submessages = Vec::new();
        let heartbeat = Heartbeat::new(
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
            *remote_writer_guid.entity_id(),
            SequenceNumber(1),
            SequenceNumber(0),
            Count(1),
            true,
            false,
            Endianness::LittleEndian,
        );
        submessages.push(RtpsSubmessage::Heartbeat(heartbeat));
        let received_message = RtpsMessage::new(*remote_writer_guid.prefix(), submessages);       

        stateful_reader_behaviour.run_waiting_heartbeat_state(&mut writer_proxy, Some(&received_message));
        assert_eq!(writer_proxy.missing_changes(), &[].iter().cloned().collect());
        assert_eq!(stateful_reader_behaviour.must_send_ack, false);
    }

    #[test]
    fn run_waiting_heartbeat_state_and_must_send_ack_state() {
        let mut stateful_reader_behaviour = StatefulReaderBehaviour::new();

        let reader_guid = GUID::new(GuidPrefix([2;12]), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
        let remote_writer_guid = GUID::new(GuidPrefix([1;12]), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let mut writer_proxy = WriterProxy::new(remote_writer_guid, vec![], vec![]);

        let mut submessages = Vec::new();
        let heartbeat = Heartbeat::new(
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
            *remote_writer_guid.entity_id(),
            SequenceNumber(3),
            SequenceNumber(6),
            Count(1),
            false,
            false,
            Endianness::LittleEndian,
        );
        submessages.push(RtpsSubmessage::Heartbeat(heartbeat));
        let received_message = RtpsMessage::new(*remote_writer_guid.prefix(), submessages);       

        stateful_reader_behaviour.run_waiting_heartbeat_state(&mut writer_proxy, Some(&received_message));
        assert_eq!(writer_proxy.missing_changes(), &[SequenceNumber(3), SequenceNumber(4), SequenceNumber(5), SequenceNumber(6)].iter().cloned().collect());
        assert_eq!(stateful_reader_behaviour.must_send_ack, true);

        let heartbeat_response_delay = Duration::from_millis(300);
        let message = stateful_reader_behaviour.run_must_send_ack_state(&mut writer_proxy, &reader_guid, heartbeat_response_delay);
        assert!(message.is_none());

        std::thread::sleep(heartbeat_response_delay.into());

        let message = stateful_reader_behaviour.run_must_send_ack_state(&mut writer_proxy, &reader_guid, heartbeat_response_delay).unwrap();
        assert_eq!(message.len(), 1);
        if let RtpsSubmessage::AckNack(acknack) = &message[0] {
            assert_eq!(acknack.writer_id(), remote_writer_guid.entity_id());
            assert_eq!(acknack.reader_id(), reader_guid.entity_id());
            assert_eq!(acknack.count(), &Count(1));
            assert_eq!(acknack.reader_sn_state().base(), &SequenceNumber(2));
            assert_eq!(acknack.reader_sn_state().set(), writer_proxy.missing_changes());
        } else {
            panic!("Wrong message type");
        }
    }
}