use super::submessage_elements;
use super::SubmessageFlag;
use super::{Submessage, SubmessageHeader};

use crate::messages::types::constants;
#[derive(PartialEq, Debug)]
pub struct NackFrag {
    pub endianness_flag: SubmessageFlag,
    pub reader_id: submessage_elements::EntityId,
    pub writer_id: submessage_elements::EntityId,
    pub writer_sn: submessage_elements::SequenceNumber,
    pub fragment_number_state: submessage_elements::FragmentNumberSet,
    pub count: submessage_elements::Count,
}

impl Submessage for NackFrag {
    fn submessage_header(&self) -> SubmessageHeader {
        const X: SubmessageFlag = false;
        let e = self.endianness_flag;
        let flags = [e, X, X, X, X, X, X, X];

        SubmessageHeader::new(constants::SUBMESSAGE_KIND_NACK_FRAG, flags, 0)
    }

    fn is_valid(&self) -> bool {
        todo!()
        // if self.writer_sn <= 0 ||
        // !self.fragment_number_state.is_valid() {
        //     false
        // } else {
        //     true
        // }
    }
}

impl serde::Serialize for NackFrag {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}
