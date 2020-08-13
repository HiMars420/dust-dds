use std::collections::VecDeque;
use std::sync::Mutex;

use crate::types::Locator;
use crate::messages::RtpsMessage;
use super::{Transport, TransportResult};

pub struct StubTransport {
    read: Mutex<VecDeque<(RtpsMessage, Locator)>>,
    write: Mutex<VecDeque<(RtpsMessage, Vec<Locator>)>>,
    unicast_locator: Locator,
    multicast_locator: Option<Locator>,
}

impl StubTransport {
    pub fn push_read(&self, message: RtpsMessage, locator: Locator) {
        self.read.lock().unwrap().push_back((message, locator));
    }

    pub fn pop_write(&self) -> Option<(RtpsMessage, Vec<Locator>)> {
        self.write.lock().unwrap().pop_front()
    }

    pub fn receive_from(&self, transport: &StubTransport) {
        while let Some((message, dst_locator_list)) = transport.pop_write() {
            // If the message destination is the multicast, then its source has to be the same multicast as well
            // otherwise the source is the
            let dst_locator = dst_locator_list[0];
            if transport.multicast_locator_list().contains(&dst_locator) {
                self.push_read(message, dst_locator);
            } else {
                self.push_read(message, transport.unicast_locator_list()[0]);
            }
        }
    }
}

impl Transport for StubTransport {
    fn new(unicast_locator: Locator, multicast_locator: Option<Locator>) -> TransportResult<Self> {
        Ok(Self {
            read: Mutex::new(VecDeque::new()),
            write: Mutex::new(VecDeque::new()),
            unicast_locator,
            multicast_locator,
        })
    }

    fn read(&self) -> TransportResult<Option<(RtpsMessage, Locator)>> {
        match self.read.lock().unwrap().pop_front() {
            Some((message, locator)) => Ok(Some((message, locator))),
            None => Ok(None),
        }
    }

    fn write(&self, message: RtpsMessage, unicast_locator_list: &[Locator], multicast_locator_list: &[Locator]) {
        let mut locator_list = Vec::new();
        locator_list.extend_from_slice(unicast_locator_list);
        locator_list.extend_from_slice(multicast_locator_list);
        
        self.write.lock().unwrap().push_back((message, locator_list));
    }

    fn unicast_locator_list(&self) -> Vec<Locator> {
        vec![self.unicast_locator]
    }
    
    fn multicast_locator_list(&self) -> Vec<Locator> {
        match self.multicast_locator {
            Some(multicast_locator) => vec![multicast_locator],
            None => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{RtpsSubmessage, InfoTs, Endianness };

    #[test]
    fn receive_from_transport_unicast_and_multicast() {
        let unicast_locator1 = Locator::new_udpv4(7400, [192,168,0,5]);
        let multicast_locator1 = Locator::new_udpv4(7400, [239,255,0,1]);
        let transport1 = StubTransport::new(unicast_locator1, Some(multicast_locator1)).unwrap();

        let unicast_locator2 = Locator::new_udpv4(7400, [192,168,0,25]);
        let multicast_locator2 = Locator::new_udpv4(7400, [239,255,0,1]);
        let transport2 = StubTransport::new(unicast_locator2, Some(multicast_locator2)).unwrap();


        // Write to the unicast locator
        let message = RtpsMessage::new([1;12], vec![RtpsSubmessage::InfoTs(InfoTs::new(None, Endianness::BigEndian))]);
        transport2.write(message, &[unicast_locator1], &[]);

        transport1.receive_from(&transport2);
        let (message_received, src_message) = transport1.read().unwrap().unwrap();
        assert_eq!(src_message, unicast_locator2);
        assert_eq!(message_received.submessages().len(), 1);
        assert!(transport2.pop_write().is_none());

        // Write to the multicast locator
        let message = RtpsMessage::new([1;12], vec![RtpsSubmessage::InfoTs(InfoTs::new(None, Endianness::BigEndian))]);
        transport2.write(message, &[], &[multicast_locator1]);

        transport1.receive_from(&transport2);
        let (message_received, src_message) = transport1.read().unwrap().unwrap();
        assert_eq!(src_message, multicast_locator2);
        assert_eq!(message_received.submessages().len(), 1);
        assert!(transport2.pop_write().is_none());
    }
}