use super::{SubmessageKind, SubmessageFlag, };
use super::{Submessage, SubmessageHeader, };
use super::submessage_elements;

#[derive(PartialEq, Debug)]
pub struct InfoReply {
    endianness_flag: SubmessageFlag,
    multicast_flag: SubmessageFlag,
    unicast_locator_list: submessage_elements::LocatorList,
    multicast_locator_list: submessage_elements::LocatorList,
}

impl Submessage for InfoReply {
    fn submessage_header(&self, octets_to_next_header: u16) -> SubmessageHeader {
        let submessage_id = SubmessageKind::InfoReply;

        const X : SubmessageFlag = false;
        let e = self.endianness_flag; 
        let m = self.multicast_flag; 
        let flags = [e, m, X, X, X, X, X, X];

        SubmessageHeader::new(submessage_id, flags, octets_to_next_header)
    }

    fn is_valid(&self) -> bool {
        true
    }   
}