use rust_rtps_pim::messages::{types::SubmessageFlag, RtpsSubmessageHeader};
use std::io::Write;
use byteorder::ByteOrder;

use crate::{submessage_elements::{
        EntityIdUdp, FragmentNumberUdp, SequenceNumberUdp, SerializedDataUdp, ULongUdp, UShortUdp,
    }};

#[derive(Debug, PartialEq)]
pub struct DataFragUdp<'a> {
    pub serialized_data: SerializedDataUdp<'a>,
}

impl<'a> crate::serialize::Serialize for DataFragUdp<'a> {
    fn serialize<W: Write, B: ByteOrder>(&self, mut _writer: W) -> crate::serialize::Result {
        todo!()
    }
}
impl<'a:'de, 'de> crate::deserialize::Deserialize<'de> for DataFragUdp<'a> {
    fn deserialize<B>(_buf: &mut &'de[u8]) -> crate::deserialize::Result<Self> where B: ByteOrder {
        todo!()
    }
}

impl<'a> rust_rtps_pim::messages::submessages::DataFragSubmessageTrait for DataFragUdp<'a> {
    type EntityIdSubmessageElementType = EntityIdUdp;
    type SequenceNumberSubmessageElementType = SequenceNumberUdp;
    type FragmentNumberSubmessageElementType = FragmentNumberUdp;
    type UShortSubmessageElementType = UShortUdp;
    type ULongSubmessageElementType = ULongUdp;
    type ParameterListSubmessageElementType = ();
    type SerializedDataFragmentSubmessageElementType = SerializedDataUdp<'a>;

    fn new(
        _endianness_flag: SubmessageFlag,
        _inline_qos_flag: SubmessageFlag,
        _non_standard_payload_flag: SubmessageFlag,
        _key_flag: SubmessageFlag,
        _reader_id: Self::EntityIdSubmessageElementType,
        _writer_id: Self::EntityIdSubmessageElementType,
        _writer_sn: Self::SequenceNumberSubmessageElementType,
        _fragment_starting_num: Self::FragmentNumberSubmessageElementType,
        _fragments_in_submessage: Self::UShortSubmessageElementType,
        _data_size: Self::ULongSubmessageElementType,
        _fragment_size: Self::UShortSubmessageElementType,
        _inline_qos: Self::ParameterListSubmessageElementType,
        _serialized_payload: Self::SerializedDataFragmentSubmessageElementType,
    ) -> Self {
        todo!()
    }

    fn endianness_flag(&self) -> SubmessageFlag {
        todo!()
    }

    fn inline_qos_flag(&self) -> SubmessageFlag {
        todo!()
    }

    fn non_standard_payload_flag(&self) -> SubmessageFlag {
        todo!()
    }

    fn key_flag(&self) -> SubmessageFlag {
        todo!()
    }

    fn reader_id(&self) -> &EntityIdUdp {
        todo!()
    }

    fn writer_id(&self) -> &EntityIdUdp {
        todo!()
    }

    fn writer_sn(&self) -> &SequenceNumberUdp {
        todo!()
    }

    fn fragment_starting_num(&self) -> &FragmentNumberUdp {
        todo!()
    }

    fn fragments_in_submessage(&self) -> &UShortUdp {
        todo!()
    }

    fn data_size(&self) -> &ULongUdp {
        todo!()
    }

    fn fragment_size(&self) -> &UShortUdp {
        todo!()
    }

    fn inline_qos(&self) -> &Self::ParameterListSubmessageElementType {
        todo!()
    }

    fn serialized_payload(&self) -> &SerializedDataUdp<'a> {
        todo!()
    }
}

impl<'a> rust_rtps_pim::messages::Submessage for DataFragUdp<'a> {
    fn submessage_header(&self) -> RtpsSubmessageHeader {
        todo!()
    }
}
