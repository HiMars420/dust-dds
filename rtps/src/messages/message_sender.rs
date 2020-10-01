use std::collections::VecDeque;

use crate::types::{GUID, GuidPrefix, Locator };
use crate::types::constants::{PROTOCOL_VERSION_2_4, VENDOR_ID};
use crate::transport::Transport;

use super::{RtpsSubmessage, Endianness};
use super::submessages::{InfoTs};
use super::message::{RtpsMessage};
use super::types::Time;

pub trait Sender {
    fn push_send_message(&self, dst_locator: &Locator, dst_guid: &GUID, submessage: RtpsSubmessage);

    fn pop_send_message(&self) -> Option<(Vec<Locator>, VecDeque<RtpsSubmessage>)>;
}

pub struct RtpsMessageSender;

impl RtpsMessageSender {
    pub fn send(participant_guid_prefix: GuidPrefix, transport: &dyn Transport,  sender_list: &[&dyn Sender]) {
        for sender in sender_list {
            while let Some((dst_locators, submessage_list)) = sender.pop_send_message() {
                let mut rtps_submessages = Vec::new();
                for submessage in submessage_list {
                    rtps_submessages.push(RtpsSubmessage::InfoTs(InfoTs::new(Endianness::LittleEndian, Some(Time::now()))));
                    rtps_submessages.push(submessage);
                }

                if !rtps_submessages.is_empty() {
                    let rtps_message = RtpsMessage::new(PROTOCOL_VERSION_2_4, VENDOR_ID, participant_guid_prefix, rtps_submessages);
                    transport.write(rtps_message, &dst_locators);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::memory::MemoryTransport;
    use crate::types::{TopicKind, GUID, ChangeKind, Locator, ReliabilityKind};
    use crate::types::constants::{
        ENTITYID_UNKNOWN,
        ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
        ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
        ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER, };
    use crate::behavior::types::Duration;
    use crate::structure::{StatelessWriter, StatefulWriter, ReaderProxy};

    #[test]
    fn stateless_writer_single_reader_locator() {
        let transport = MemoryTransport::new(Locator::new(0,0,[0;16]), vec![]).unwrap();
        let participant_guid_prefix = [1,2,3,4,5,5,4,3,2,1,1,2];

        let stateless_writer_1 = StatelessWriter::new(GUID::new(participant_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER), TopicKind::WithKey);
        let reader_locator_1 = Locator::new(-2, 10000, [1;16]);
        stateless_writer_1.reader_locator_add(reader_locator_1);

        // Check that nothing is sent to start with
        RtpsMessageSender::send(participant_guid_prefix, &transport, &[&stateless_writer_1]);
        
        assert_eq!(transport.pop_write(), None);

        // Add a change to the stateless writer history cache and run the writer
        let change_1 = stateless_writer_1.new_change(ChangeKind::Alive, Some(vec![1,2,3,4]), None, [5;16]);
        stateless_writer_1.writer_cache().add_change(change_1);
        stateless_writer_1.run();

        RtpsMessageSender::send(participant_guid_prefix, &transport, &[&stateless_writer_1]);
        let (message, dst_locator) = transport.pop_write().unwrap();
        assert_eq!(dst_locator, vec![reader_locator_1]);
        assert_eq!(message.submessages().len(), 2);
        match &message.submessages()[0] {
            RtpsSubmessage::InfoTs(info_ts) => {
                assert_eq!(info_ts.time().is_some(), true);
            },
            _ => panic!("Unexpected submessage type received"),
        };
        match &message.submessages()[1] {
            RtpsSubmessage::Data(data) => {
                assert_eq!(data.reader_id(), ENTITYID_UNKNOWN);
                assert_eq!(data.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
                assert_eq!(data.writer_sn(), 1);
            },
            _ => panic!("Unexpected submessage type received"),
        };

        // Add two changes to the stateless writer history cache and run the writer
        let change_2 = stateless_writer_1.new_change(ChangeKind::Alive, Some(vec![1,2,3,4]), None, [5;16]);
        let change_3 = stateless_writer_1.new_change(ChangeKind::Alive, Some(vec![1,2,3,4]), None, [5;16]);
        stateless_writer_1.writer_cache().add_change(change_2);
        stateless_writer_1.writer_cache().add_change(change_3);
        stateless_writer_1.run();

        RtpsMessageSender::send(participant_guid_prefix, &transport, &[&stateless_writer_1]);
        let (message, dst_locator) = transport.pop_write().unwrap();
        assert_eq!(dst_locator, vec![reader_locator_1]);
        assert_eq!(message.submessages().len(), 4);
        match &message.submessages()[0] {
            RtpsSubmessage::InfoTs(info_ts) => {
                assert_eq!(info_ts.time().is_some(), true);
            },
            _ => panic!("Unexpected submessage type received"),
        };
        match &message.submessages()[1] {
            RtpsSubmessage::Data(data) => {
                assert_eq!(data.reader_id(), ENTITYID_UNKNOWN);
                assert_eq!(data.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
                assert_eq!(data.writer_sn(), 2);
            },
            _ => panic!("Unexpected submessage type received"),
        };
        match &message.submessages()[2] {
            RtpsSubmessage::InfoTs(info_ts) => {
                assert_eq!(info_ts.time().is_some(), true);
            },
            _ => panic!("Unexpected submessage type received"),
        };
        match &message.submessages()[3] {
            RtpsSubmessage::Data(data) => {
                assert_eq!(data.reader_id(), ENTITYID_UNKNOWN);
                assert_eq!(data.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
                assert_eq!(data.writer_sn(), 3);
            },
            _ => panic!("Unexpected submessage type received"),
        };

        // Add two new changes but only one to the stateless writer history cache and run the writer
        let _change_4 = stateless_writer_1.new_change(ChangeKind::Alive, Some(vec![1,2,3,4]), None, [5;16]);
        let change_5 = stateless_writer_1.new_change(ChangeKind::Alive, Some(vec![1,2,3,4]), None, [5;16]);
        stateless_writer_1.writer_cache().add_change(change_5);
        stateless_writer_1.run();

        RtpsMessageSender::send(participant_guid_prefix, &transport, &[&stateless_writer_1]);
        let (message, dst_locator) = transport.pop_write().unwrap();
        assert_eq!(dst_locator, vec![reader_locator_1]);
        assert_eq!(message.submessages().len(), 4);
        match &message.submessages()[0] {
            RtpsSubmessage::InfoTs(info_ts) => {
                assert_eq!(info_ts.time().is_some(), true);
            },
            _ => panic!("Unexpected submessage type received"),
        };
        match &message.submessages()[1] {
            RtpsSubmessage::Gap(gap) => {
                assert_eq!(gap.reader_id(), ENTITYID_UNKNOWN);
                assert_eq!(gap.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
            },
            _ => panic!("Unexpected submessage type received"),
        };
        match &message.submessages()[2] {
            RtpsSubmessage::InfoTs(info_ts) => {
                assert_eq!(info_ts.time().is_some(), true);
            },
            _ => panic!("Unexpected submessage type received"),
        };
        match &message.submessages()[3] {
            RtpsSubmessage::Data(data) => {
                assert_eq!(data.reader_id(), ENTITYID_UNKNOWN);
                assert_eq!(data.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
                assert_eq!(data.writer_sn(), 5);
            },
            _ => panic!("Unexpected submessage type received"),
        };
    }

    #[test]
    fn multiple_stateless_writers_multiple_reader_locators() {
        let transport = MemoryTransport::new(Locator::new(0,0,[0;16]), vec![]).unwrap();
        let participant_guid_prefix = [1,2,3,4,5,5,4,3,2,1,1,2];

        let reader_locator_1 = Locator::new(-2, 10000, [1;16]);
        let reader_locator_2 = Locator::new(-2, 2000, [2;16]);
        let reader_locator_3 = Locator::new(-2, 300, [3;16]);

        let stateless_writer_1 = StatelessWriter::new(GUID::new(participant_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER), TopicKind::WithKey);
        let stateless_writer_2 = StatelessWriter::new(GUID::new(participant_guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER), TopicKind::WithKey);

        RtpsMessageSender::send(participant_guid_prefix, &transport, &[&stateless_writer_1, &stateless_writer_2]);
        
        stateless_writer_1.reader_locator_add(reader_locator_1);

        stateless_writer_2.reader_locator_add(reader_locator_1);
        stateless_writer_2.reader_locator_add(reader_locator_2);
        stateless_writer_2.reader_locator_add(reader_locator_3);

        let change_1_1 = stateless_writer_1.new_change(ChangeKind::Alive, Some(vec![1,2,3,4]), None, [5;16]);
        let change_1_2 = stateless_writer_1.new_change(ChangeKind::Alive, Some(vec![1,2,3,4]), None, [5;16]);
        stateless_writer_1.writer_cache().add_change(change_1_1);
        stateless_writer_1.writer_cache().add_change(change_1_2);

        let _change_2_1 = stateless_writer_2.new_change(ChangeKind::Alive, Some(vec![1,2,4]), None, [12;16]);
        let change_2_2 = stateless_writer_2.new_change(ChangeKind::Alive, Some(vec![1,2,3,4]), None, [12;16]);
        stateless_writer_2.writer_cache().add_change(change_2_2);

        stateless_writer_1.run();
        stateless_writer_2.run();

        RtpsMessageSender::send(participant_guid_prefix, &transport, &[&stateless_writer_1, &stateless_writer_2]);

        // The order at which the messages are sent is not predictable so collect eveything in a vector and test from there onwards
        let mut sent_messages = Vec::new();
        sent_messages.push(transport.pop_write().unwrap());
        sent_messages.push(transport.pop_write().unwrap());
        sent_messages.push(transport.pop_write().unwrap());
        sent_messages.push(transport.pop_write().unwrap());

        assert_eq!(sent_messages.iter().filter(|&(_, locator)| locator==&vec![reader_locator_1]).count(), 2);
        assert_eq!(sent_messages.iter().filter(|&(_, locator)| locator==&vec![reader_locator_2]).count(), 1);
        assert_eq!(sent_messages.iter().filter(|&(_, locator)| locator==&vec![reader_locator_3]).count(), 1);

        assert!(transport.pop_write().is_none());
    }

    #[test]
    fn stateful_writer_multiple_reader_locators() {
        let transport = MemoryTransport::new(Locator::new(0,0,[0;16]), vec![]).unwrap();
        let participant_guid_prefix = [1,2,3,4,5,5,4,3,2,1,1,2];

        let stateful_writer_1 = StatefulWriter::new(
            GUID::new(participant_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER), 
            TopicKind::WithKey, 
            ReliabilityKind::BestEffort,
            true,
            Duration::from_secs(10),
            Duration::from_millis(500),
            Duration::from_millis(0)
        );

        let reader_guid_prefix = [5;12];
        let unicast_locator_1 = Locator::new(-2, 10000, [1;16]);
        let unicast_locator_2 = Locator::new(-2, 200, [2;16]);
        let multicast_locator = Locator::new(-2, 5, [10;16]);
        let reader_proxy_1 = ReaderProxy::new(
            GUID::new(reader_guid_prefix,ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR),
            vec![unicast_locator_1, unicast_locator_2],
            vec![multicast_locator],
            false,
            true);

        stateful_writer_1.matched_reader_add(reader_proxy_1);

        // Check that nothing is sent to start with
        RtpsMessageSender::send(participant_guid_prefix, &transport, &[&stateful_writer_1]);
        assert_eq!(transport.pop_write(), None);

        // Add a change to the stateless writer history cache and run the writer
        let change_1 = stateful_writer_1.new_change(ChangeKind::Alive, Some(vec![1,2,3,4]), None, [5;16]);
        stateful_writer_1.writer_cache().add_change(change_1);
        stateful_writer_1.run();

        RtpsMessageSender::send(participant_guid_prefix, &transport, &[&stateful_writer_1]);
        let (message, dst_locator) = transport.pop_write().unwrap();
        assert_eq!(dst_locator, vec![unicast_locator_1, unicast_locator_2,  multicast_locator ]);
        assert_eq!(message.submessages().len(), 2);
        match &message.submessages()[0] {
            RtpsSubmessage::InfoTs(info_ts) => {
                assert_eq!(info_ts.time().is_some(), true);
            },
            _ => panic!("Unexpected submessage type received"),
        };
        match &message.submessages()[1] {
            RtpsSubmessage::Data(data) => {
                assert_eq!(data.reader_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
                assert_eq!(data.writer_id(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
                assert_eq!(data.writer_sn(), 1);
            },
            _ => panic!("Unexpected submessage type received"),
        };
    }


}