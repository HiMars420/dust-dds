use super::submessage_elements;
use super::{Submessage, SubmessageFlag, SubmessageHeader, SubmessageKind};

#[derive(PartialEq, Debug)]
pub struct InfoTs {
    pub endianness_flag: SubmessageFlag,
    pub invalidate_flag: SubmessageFlag,
    pub timestamp: submessage_elements::Timestamp,
}

impl InfoTs {
    pub const INVALID_TIME_FLAG_MASK: u8 = 0x02;
}

impl Submessage for InfoTs {
    fn submessage_header(&self, octets_to_next_header: u16) -> SubmessageHeader {
        let submessage_id = SubmessageKind::InfoTimestamp;

        let x = false;
        let e = self.endianness_flag; // Indicates endianness.
        let i = self.invalidate_flag; // Indicates whether subsequent Submessages should be considered as having a timestamp or not.
                                      // X|X|X|X|X|X|I|E
        let flags = [e, i, x, x, x, x, x, x];

        SubmessageHeader::new(submessage_id, flags, octets_to_next_header)
    }

    fn is_valid(&self) -> bool {
        true
    }
}
