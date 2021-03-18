use super::submessage_elements;
use super::{Submessage, SubmessageHeader};
use super::{SubmessageFlag, SubmessageKind};

#[derive(PartialEq, Debug)]
pub struct InfoSource {
    pub endianness_flag: SubmessageFlag,
    pub protocol_version: submessage_elements::ProtocolVersion,
    pub vendor_id: submessage_elements::VendorId,
    pub guid_prefix: submessage_elements::GuidPrefix,
}

impl Submessage for InfoSource {
    fn submessage_header(&self, octets_to_next_header: u16) -> SubmessageHeader {
        let submessage_id = SubmessageKind::InfoSource;

        const X: SubmessageFlag = false;
        let e = self.endianness_flag;
        let flags = [e, X, X, X, X, X, X, X];

        SubmessageHeader::new(submessage_id, flags, octets_to_next_header)
    }

    fn is_valid(&self) -> bool {
        true
    }
}

impl serde::Serialize for InfoSource {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}
