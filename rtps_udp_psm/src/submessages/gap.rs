use std::io::Write;

use byteorder::ByteOrder;
use rust_rtps_pim::messages::{types::SubmessageFlag, RtpsSubmessageHeader};

use crate::{
    submessage_elements::{
        flags_to_byte, is_bit_set, EntityIdUdp, SequenceNumberSetUdp, SequenceNumberUdp,
    },
    submessage_header::{SubmessageHeaderUdp, GAP},
};

#[derive(Debug, PartialEq)]
pub struct GapSubmessageUdp {
    pub header: SubmessageHeaderUdp,
    reader_id: EntityIdUdp,
    writer_id: EntityIdUdp,
    gap_start: SequenceNumberUdp,
    gap_list: SequenceNumberSetUdp,
}

impl crate::serialize::Serialize for GapSubmessageUdp {
    fn serialize<W: Write, B: ByteOrder>(&self, mut writer: W) -> crate::serialize::Result {
        todo!()
    }
}
impl<'de> crate::deserialize::Deserialize<'de> for GapSubmessageUdp {
    fn deserialize<B>(buf: &mut &'de[u8]) -> crate::deserialize::Result<Self> where B: ByteOrder {
        todo!()
    }
}

impl rust_rtps_pim::messages::submessages::GapSubmessage for GapSubmessageUdp {
    type EntityIdSubmessageElementType = EntityIdUdp;
    type SequenceNumberSubmessageElementType = SequenceNumberUdp;
    type SequenceNumberSetSubmessageElementType = SequenceNumberSetUdp;

    fn new(
        endianness_flag: SubmessageFlag,
        reader_id: EntityIdUdp,
        writer_id: EntityIdUdp,
        gap_start: SequenceNumberUdp,
        gap_list: SequenceNumberSetUdp,
    ) -> Self {
        let flags = flags_to_byte([endianness_flag]);

        let submessage_length = 16 + gap_list.len();

        let header = SubmessageHeaderUdp {
            submessage_id: GAP,
            flags,
            submessage_length,
        };
        Self {
            header,
            reader_id,
            writer_id,
            gap_start,
            gap_list,
        }
    }

    fn endianness_flag(&self) -> SubmessageFlag {
        is_bit_set(self.header.flags, 0)
    }

    fn reader_id(&self) -> &EntityIdUdp {
        &self.reader_id
    }

    fn writer_id(&self) -> &EntityIdUdp {
        &self.writer_id
    }

    fn gap_start(&self) -> &SequenceNumberUdp {
        &self.gap_start
    }

    fn gap_list(&self) -> &SequenceNumberSetUdp {
        &self.gap_list
    }
}

impl rust_rtps_pim::messages::Submessage for GapSubmessageUdp {
    fn submessage_header(&self) -> RtpsSubmessageHeader {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{deserialize::from_bytes_le, serialize::to_bytes_le};

    use super::*;
    use rust_rtps_pim::messages::submessage_elements::{
        SequenceNumberSetSubmessageElementType, SequenceNumberSubmessageElementType,
    };
    use rust_serde_cdr::{
        deserializer::RtpsMessageDeserializer, serializer::RtpsMessageSerializer,
    };

    fn serialize<T: serde::Serialize>(value: T) -> Vec<u8> {
        let mut serializer = RtpsMessageSerializer {
            writer: Vec::<u8>::new(),
        };
        value.serialize(&mut serializer).unwrap();
        serializer.writer
    }

    fn deserialize<'de, T: serde::Deserialize<'de>>(buffer: &'de [u8]) -> T {
        let mut de = RtpsMessageDeserializer { reader: buffer };
        serde::de::Deserialize::deserialize(&mut de).unwrap()
    }

    #[test]
    fn serialize_gap() {
        let endianness_flag = true;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let gap_start = SequenceNumberUdp::new(&5);
        let gap_list = SequenceNumberSetUdp::new(&10, &[]);
        let submessage: GapSubmessageUdp = rust_rtps_pim::messages::submessages::GapSubmessage::new(
            endianness_flag,
            reader_id,
            writer_id,
            gap_start,
            gap_list,
        );
        #[rustfmt::skip]
        assert_eq!(to_bytes_le(&submessage).unwrap(), vec![
                0x08_u8, 0b_0000_0001, 28, 0, // Submessage header
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // gapStart: SequenceNumber: high
                5, 0, 0, 0, // gapStart: SequenceNumber: low
                0, 0, 0, 0, // gapList: SequenceNumberSet: bitmapBase: high
               10, 0, 0, 0, // gapList: SequenceNumberSet: bitmapBase: low
                0, 0, 0, 0, // gapList: SequenceNumberSet: numBits (ULong)
            ]
        );
    }

    #[test]
    fn deserialize_gap() {
        let endianness_flag = true;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let gap_start = SequenceNumberUdp::new(&5);
        let gap_list = SequenceNumberSetUdp::new(&10, &[]);
        let expected: GapSubmessageUdp = rust_rtps_pim::messages::submessages::GapSubmessage::new(
            endianness_flag,
            reader_id,
            writer_id,
            gap_start,
            gap_list,
        );
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x08, 0b_0000_0001, 28, 0, // Submessage header
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // gapStart: SequenceNumber: high
            5, 0, 0, 0, // gapStart: SequenceNumber: low
            0, 0, 0, 0, // gapList: SequenceNumberSet: bitmapBase: high
           10, 0, 0, 0, // gapList: SequenceNumberSet: bitmapBase: low
            0, 0, 0, 0, // gapList: SequenceNumberSet: numBits (ULong)
        ]).unwrap();
        assert_eq!(expected, result);
    }
}
