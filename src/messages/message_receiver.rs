use crate::types::{GUID, GuidPrefix, Locator,};
use crate::transport::Transport;

use super::submessages::RtpsSubmessage;

// ////////////////// RTPS Message Receiver

pub trait Receiver {
    fn push_receive_message(&self, src_guid_prefix: GuidPrefix, submessage: RtpsSubmessage);
    
    fn pop_receive_message(&self, guid: &GUID) -> Option<(GuidPrefix, RtpsSubmessage)>;

    fn is_submessage_destination(&self, src_locator: &Locator, src_guid_prefix: &GuidPrefix, submessage: &RtpsSubmessage) -> bool;
}

pub struct RtpsMessageReceiver;

impl RtpsMessageReceiver {
    pub fn receive(participant_guid_prefix: GuidPrefix, transport: &impl Transport, receiver_list: &[&dyn Receiver]) {
        if let Some((message, src_locator)) = transport.read().unwrap() {
            let _source_version = message.header().version();
            let _source_vendor_id = message.header().vendor_id();
            let source_guid_prefix = *message.header().guid_prefix();
            let _dest_guid_prefix = participant_guid_prefix;
            let _unicast_reply_locator_list = vec![Locator::new(0,0,[0;16])];
            let _multicast_reply_locator_list = vec![Locator::new(0,0,[0;16])];
            let mut _timestamp = None;
            let _message_length = 0;
    
            for submessage in message.take_submessages() {
                if submessage.is_entity_submessage() {
                    for &receiver in receiver_list {
                        if receiver.is_submessage_destination(&src_locator, &source_guid_prefix, &submessage) {
                            receiver.push_receive_message(source_guid_prefix, submessage);
                            break;
                        }
                    }  
                } else if submessage.is_interpreter_submessage(){
                    match submessage {
                        RtpsSubmessage::InfoTs(info_ts) => _timestamp = info_ts.time(),
                        _ => panic!("Unexpected interpreter submessage"),
                    };
                }
            }
        }    
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TopicKind, ReliabilityKind};
    use crate::types::constants::{ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR, ENTITYID_UNKNOWN};
    use crate::transport::memory_transport::MemoryTransport;
    use crate::messages::{Endianness, RtpsMessage,};
    use crate::messages::submessages::{Data};
    use crate::messages::submessages::data_submessage::Payload;
    use crate::behavior::types::Duration;
    use crate::structure::stateful_reader::{StatefulReader, WriterProxy};
    use crate::structure::stateless_reader::StatelessReader;

    #[test]
    fn stateless_reader_message_receive() {
        let transport = MemoryTransport::new(Locator::new(0,0,[0;16]), None).unwrap();
        let guid_prefix = [1,2,3,4,5,6,8,1,2,3,4,5];

        let src_locator = Locator::new_udpv4(7500, [127,0,0,1]);

        let stateless_reader_guid = GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR);
        let stateless_reader = StatelessReader::new(
            stateless_reader_guid,
            TopicKind::WithKey,
            vec![src_locator],
            vec![],
            false);


        // Run the empty transport and check that nothing happends
        RtpsMessageReceiver::receive(guid_prefix, &transport, &[&stateless_reader]);
        assert!(stateless_reader.pop_receive_message(&stateless_reader_guid).is_none());

        // Send a message to the stateless reader
        let src_guid_prefix = [5,2,3,4,5,6,8,1,2,3,4,5];
        let data = Data::new(Endianness::LittleEndian, ENTITYID_UNKNOWN, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, 1, None, Payload::None);
        let message = RtpsMessage::new(src_guid_prefix, vec![RtpsSubmessage::Data(data)]);
        transport.push_read(message, src_locator);

        RtpsMessageReceiver::receive(guid_prefix, &transport, &[&stateless_reader]);

        let expected_data = Data::new(Endianness::LittleEndian, ENTITYID_UNKNOWN, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, 1, None, Payload::None);
        match stateless_reader.pop_receive_message(&stateless_reader_guid) {
            Some((received_guid_prefix, RtpsSubmessage::Data(data_received))) => {
                assert_eq!(received_guid_prefix, src_guid_prefix);
                assert_eq!(data_received, expected_data);
            },
            _ => panic!("Unexpected message received"),
        };
    }

    #[test]
    fn stateless_reader_message_receive_other_locator() {
        let transport = MemoryTransport::new(Locator::new(0,0,[0;16]), None).unwrap();
        let guid_prefix = [1,2,3,4,5,6,8,1,2,3,4,5];

        let src_locator = Locator::new_udpv4(7500, [127,0,0,1]);

        let stateless_reader_guid = GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR);
        let stateless_reader = StatelessReader::new(
            stateless_reader_guid,
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

        RtpsMessageReceiver::receive(guid_prefix, &transport, &[&stateless_reader]);
        assert!(stateless_reader.pop_receive_message(&stateless_reader_guid).is_none());
    }

    #[test]
    fn stateful_reader_message_receive() {
        let transport = MemoryTransport::new(Locator::new(0,0,[0;16]), None).unwrap();
        let guid_prefix = [1,2,3,4,5,6,8,1,2,3,4,5];

        let stateful_reader = StatefulReader::new(
            GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR),
            TopicKind::WithKey,
            ReliabilityKind::BestEffort,
            false,
            Duration::from_millis(500));

        RtpsMessageReceiver::receive(guid_prefix, &transport, &[&stateful_reader]);

        let remote_guid_prefix = [1;12];
        let remote_guid = GUID::new(remote_guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER);
        let src_locator = Locator::new_udpv4(7500, [127,0,0,1]);

        let proxy = WriterProxy::new(remote_guid,vec![src_locator], vec![]);
        stateful_reader.matched_writer_add(proxy);
        
        // Run the empty transport and check that nothing happends
        RtpsMessageReceiver::receive(guid_prefix, &transport, &[&stateful_reader]);
        assert!(stateful_reader.pop_receive_message(&remote_guid).is_none());

        // Send a message from the matched writer to the reader
        let data = Data::new(Endianness::LittleEndian, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, 1, None, Payload::None);
        let message = RtpsMessage::new(remote_guid_prefix, vec![RtpsSubmessage::Data(data)]);
        transport.push_read(message, src_locator);

        RtpsMessageReceiver::receive(guid_prefix, &transport, &[&stateful_reader]);

        let expected_data = Data::new(Endianness::LittleEndian, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, 1, None, Payload::None);
        match stateful_reader.pop_receive_message(&remote_guid) {
            Some((_, RtpsSubmessage::Data(data_received))) => {
                assert_eq!(data_received, expected_data);
            },
            _ => panic!("Unexpected message received"),
        };

        // Send a message from an unmatched writer to the reader
        let other_remote_guid_prefix = [10;12];
        let data = Data::new(Endianness::LittleEndian, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, 1, None, Payload::None);
        let message = RtpsMessage::new(other_remote_guid_prefix, vec![RtpsSubmessage::Data(data)]);
        transport.push_read(message, src_locator);

        RtpsMessageReceiver::receive(guid_prefix, &transport, &[&stateful_reader]);
        assert!(stateful_reader.pop_receive_message(&remote_guid).is_none());
    }    

}