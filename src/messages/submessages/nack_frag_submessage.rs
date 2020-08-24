use super::{SubmessageKind, SubmessageFlag, };
use super::{Submessage, SubmessageHeader, };
use super::submessage_elements;


#[derive(PartialEq, Debug)]
pub struct NackFrag {
    endianness_flag: SubmessageFlag,
    reader_id: submessage_elements::EntityId,
    writer_id: submessage_elements::EntityId,
    writer_sn: submessage_elements::SequenceNumber,
    fragment_number_state: submessage_elements::FragmentNumberSet,
    count: submessage_elements::Count,
}


impl Submessage for NackFrag {
    fn submessage_header(&self, octets_to_next_header: u16) -> SubmessageHeader {
        let submessage_id = SubmessageKind::NackFrag;

        const X: SubmessageFlag = false;
        let e = self.endianness_flag; 
        let flags = [e, X, X, X, X, X, X, X];

        SubmessageHeader::new(submessage_id, flags, octets_to_next_header)
    }

    fn is_valid(&self) -> bool {
        if self.writer_sn.0 <= 0 ||
        !self.fragment_number_state.is_valid() {
            false
        } else {
            true
        }
    }
}