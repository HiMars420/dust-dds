use crate::types::{GUID, GuidPrefix, Locator,};
use crate::stateless_reader::StatelessReader;
use crate::stateless_writer::StatelessWriter;
use crate::stateful_reader::{StatefulReader, WriterProxy};
use crate::stateful_writer::{StatefulWriter, ReaderProxy};
use crate::transport::Transport;


use super::submessage::RtpsSubmessage;
use super::{Data, Gap, Heartbeat, AckNack, InfoTs, Endianness};
use super::message::{RtpsMessage};
use super::types::Time;
use super::message_sender::WriterReceiveMessage;

// Messages received by the reader. Which are the same as the ones sent by the writer
#[derive(Debug)]
pub enum ReaderReceiveMessage {
    Data(Data),
    Gap(Gap),
    Heartbeat(Heartbeat),
}

pub type ReaderSendMessage = WriterReceiveMessage;


// ////////////////// RTPS Message Receiver

pub fn rtps_message_receiver(transport: &impl Transport, participant_guid_prefix: GuidPrefix, stateless_reader_list: &[&StatelessReader]) {
    if let Some((message, src_locator)) = transport.read().unwrap() {
        let _source_version = message.header().version();
        let _source_vendor_id = message.header().vendor_id();
        let source_guid_prefix = *message.header().guid_prefix();
        let _dest_guid_prefix = participant_guid_prefix;
        let _unicast_reply_locator_list = vec![Locator::new(0,0,[0;16])];
        let _multicast_reply_locator_list = vec![Locator::new(0,0,[0;16])];
        let mut _timestamp = None;
        let _message_length = 0;
        
        let source_locator = Locator::new(0,0, [0;16]);

        for submessage in message.take_submessages() {
            match submessage {
                // Writer to reader messages
                RtpsSubmessage::Data(data) => receive_reader_submessage(&src_locator, source_guid_prefix, ReaderReceiveMessage::Data(data), stateless_reader_list),
                RtpsSubmessage::Gap(gap) => receive_reader_submessage(&src_locator, source_guid_prefix, ReaderReceiveMessage::Gap(gap), stateless_reader_list),
                RtpsSubmessage::Heartbeat(heartbeat) => receive_reader_submessage(&source_locator, source_guid_prefix, ReaderReceiveMessage::Heartbeat(heartbeat), stateless_reader_list),
                // Reader to writer messages
                RtpsSubmessage::AckNack(ack_nack) => receive_writer_submessage(source_guid_prefix, WriterReceiveMessage::AckNack(ack_nack)),
                // Receiver status messages
                RtpsSubmessage::InfoTs(info_ts) => _timestamp = info_ts.time(),
            }
        }
    }    
}

fn receive_reader_submessage(source_locator: &Locator, source_guid_prefix: GuidPrefix, message: ReaderReceiveMessage, stateless_reader_list: &[&StatelessReader]) {
    let writer_guid = match &message {
        ReaderReceiveMessage::Data(data) => GUID::new(source_guid_prefix, data.writer_id()),
        ReaderReceiveMessage::Gap(gap) => GUID::new(source_guid_prefix, gap.writer_id()),
        ReaderReceiveMessage::Heartbeat(heartbeat) => GUID::new(source_guid_prefix, heartbeat.writer_id()),
    };

    for stateless_reader in stateless_reader_list {
        if stateless_reader.unicast_locator_list().iter().find(|&loc| loc == source_locator).is_some() ||
            stateless_reader.multicast_locator_list().iter().find(|&loc| loc == source_locator).is_some() {
                stateless_reader.push_receive_message(source_guid_prefix, message);
                break;
        }
    }

    // if let Some(writer_proxy) = stateful_reader.matched_writers().get(&writer_guid) {
    //     writer_proxy_received_message(writer_proxy, message);
    //     break;
    
}

fn receive_writer_submessage(source_guid_prefix: GuidPrefix, message: WriterReceiveMessage) {
    todo!()
    // let reader_guid = match &message {
    //     WriterReceiveMessage::AckNack(ack_nack) =>  GUID::new(source_guid_prefix, ack_nack.reader_id()),
    // };

    // for writer in &self.writer_list {
    //     match writer {
    //         Writer::StatelessWriter(_stateless_writer) => {
    //             // Stateless writers do not receive any message because they are only best effort
    //         },
    //         Writer::StatefulWriter(stateful_writer) => {
    //             if let Some(reader_proxy) = stateful_writer.matched_readers().get(&reader_guid) {
    //                 reader_proxy_received_message(reader_proxy, message);
    //                 break;
    //             }
    //         },
    //     }
    // }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TopicKind;
    use crate::types::constants::{ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR, ENTITYID_UNKNOWN};
    use crate::transport_stub::StubTransport;
    use crate::messages::{Endianness, Data, RtpsMessage, Payload};
    // use crate::messages::{InfoTs, Data};
    // use crate::messages::Endianness;
    // use crate::messages::types::Time;
    // use crate::messages::data_submessage::Payload;
    // use crate::types::{GUID, EntityId, EntityKind, TopicKind, ReliabilityKind};
    // use crate::stateful_reader::WriterProxy;
    // use crate::behavior::types::Duration;

    #[test]
    fn stateless_reader_message_receive() {
        let transport = StubTransport::new();
        let guid_prefix = [1,2,3,4,5,6,8,1,2,3,4,5];

        let src_locator = Locator::new_udpv4(7500, [127,0,0,1]);

        let stateless_reader = StatelessReader::new(
            GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR),
            TopicKind::WithKey,
            vec![src_locator],
            vec![],
            false);

        // Run the empty transport and check that nothing happends
        rtps_message_receiver(&transport, guid_prefix, &[&stateless_reader]);
        assert!(stateless_reader.pop_receive_message().is_none());

        // Send a message to the stateless reader
        let src_guid_prefix = [5,2,3,4,5,6,8,1,2,3,4,5];
        let data = Data::new(Endianness::LittleEndian, ENTITYID_UNKNOWN, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, 1, None, Payload::None);
        let message = RtpsMessage::new(src_guid_prefix, vec![RtpsSubmessage::Data(data)]);
        transport.push_read(message, src_locator);

        rtps_message_receiver(&transport, guid_prefix, &[&stateless_reader]);

        let expected_data = Data::new(Endianness::LittleEndian, ENTITYID_UNKNOWN, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, 1, None, Payload::None);
        match stateless_reader.pop_receive_message() {
            Some((received_guid_prefix, ReaderReceiveMessage::Data(data_received))) => {
                assert_eq!(received_guid_prefix, src_guid_prefix);
                assert_eq!(data_received, expected_data);
            },
            _ => panic!("Unexpected message received"),
        };
    }

    #[test]
    fn stateless_reader_message_receive_other_locator() {
        let transport = StubTransport::new();
        let guid_prefix = [1,2,3,4,5,6,8,1,2,3,4,5];

        let src_locator = Locator::new_udpv4(7500, [127,0,0,1]);

        let stateless_reader = StatelessReader::new(
            GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR),
            TopicKind::WithKey,
            vec![src_locator],
            vec![],
            false);

        // Send a message to the stateless reader
        let other_locator = Locator::new_udpv4(7600, [1,1,1,1]);
        let src_guid_prefix = [5,2,3,4,5,6,8,1,2,3,4,5];
        let data = Data::new(Endianness::LittleEndian, ENTITYID_UNKNOWN, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, 1, None, Payload::None);
        let message = RtpsMessage::new(src_guid_prefix, vec![RtpsSubmessage::Data(data)]);
        transport.push_read(message, other_locator);

        rtps_message_receiver(&transport, guid_prefix, &[&stateless_reader]);
        assert!(stateless_reader.pop_receive_message().is_none());
    }
    
    // fn receive_infots_data_message() {
    //     let guid_prefix = [1;12];
    //     let guid = GUID::new(guid_prefix, EntityId::new([1,2,3], EntityKind::UserDefinedReaderWithKey));
    //     let mut reader1 = StatefulReader::new(
    //         guid,
    //         TopicKind::WithKey, 
    //     ReliabilityKind::BestEffort,
    //     false,
    //     Duration::from_millis(500));

    //     let proxy1 = WriterProxy::new(
    //         GUID::new([2;12], EntityId::new([1,2,3], EntityKind::UserDefinedWriterWithKey)),
    //         vec![],
    //         vec![]);

    //     reader1.matched_writer_add(proxy1);

    //     let receiver = RtpsMessageReceiver::new(guid_prefix, Locator::new(0,0, [0;16]), vec![Reader::StatefulReader(&reader1)], vec![]);

    //     let info_ts = InfoTs::new(Some(Time::new(100,100)), Endianness::LittleEndian);
    //     let data = Data::new(Endianness::LittleEndian,
    //         EntityId::new([1,2,3], EntityKind::UserDefinedReaderWithKey),
    //         EntityId::new([1,2,3], EntityKind::UserDefinedWriterWithKey),
    //         1, None, Payload::Data(vec![1,2,3,4]));
    //     let message = RtpsMessage::new([2;12],vec![RtpsSubmessage::InfoTs(info_ts), RtpsSubmessage::Data(data)]);

    //     receiver.receive(message);
    // }
}