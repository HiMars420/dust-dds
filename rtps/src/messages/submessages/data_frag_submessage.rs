use super::{SubmessageKind, SubmessageFlag,};
use super::{Submessage, SubmessageHeader, };
use super::submessage_elements;

pub struct DataFrag<'a> {
    pub endianness_flag: SubmessageFlag,
    pub inline_qos_flag: SubmessageFlag,
    pub non_standard_payload_flag: SubmessageFlag,
    pub key_flag: SubmessageFlag,
    pub reader_id: submessage_elements::EntityId,
    pub writer_id: submessage_elements::EntityId,
    pub writer_sn: submessage_elements::SequenceNumber,
    pub fragment_starting_num: submessage_elements::FragmentNumber,
    pub fragments_in_submessage: submessage_elements::UShort,
    pub data_size: submessage_elements::ULong,
    pub fragment_size: submessage_elements::UShort,
    pub inline_qos: submessage_elements::ParameterList,
    pub serialized_payload: submessage_elements::SerializedDataFragment<'a>,
}


impl<'a> Submessage for DataFrag<'a> {
    fn submessage_header(&self, octets_to_next_header: u16) -> SubmessageHeader {
        let submessage_id = SubmessageKind::DataFrag;

        const X: SubmessageFlag = false;
        let e = self.endianness_flag;
        let q = self.inline_qos_flag;
        let k = self.key_flag;
        let n = self.non_standard_payload_flag;
        let flags = [e, q, k, n, X, X, X, X];

        SubmessageHeader::new(submessage_id, flags, octets_to_next_header)
    }

    fn is_valid(&self) -> bool {
        let serialized_data_size = self.serialized_payload.len();

        if (self.writer_sn < 1 || self.writer_sn == crate::types::constants::SEQUENCE_NUMBER_UNKNOWN) ||
           (self.fragment_starting_num < 1) ||
           (self.fragment_size as u32 > self.data_size) ||
           (serialized_data_size > self.fragments_in_submessage as usize * self.fragment_size as usize)
        {
            // TODO: Check total number of fragments
            // TODO: Check validity of inline_qos
            false
        } else {
            false
        }
    }
}
