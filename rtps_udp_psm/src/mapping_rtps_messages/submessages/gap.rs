use std::io::Write;

use byteorder::ByteOrder;
use rust_rtps_pim::messages::{overall_structure::RtpsSubmessageHeader, types::SubmessageKind};
use rust_rtps_psm::messages::submessages::{GapSubmessageRead, GapSubmessageWrite};

use crate::{
    deserialize::{self, MappingReadByteOrdered, DeserializeSubmessage},
    serialize::{self, NumberOfBytes, MappingWriteByteOrdered, SerializeSubmessage},
};

impl SerializeSubmessage for GapSubmessageWrite {
    fn submessage_header(&self) -> RtpsSubmessageHeader {
        let submessage_length = 16 + self.gap_list.number_of_bytes();
        RtpsSubmessageHeader {
            submessage_id: SubmessageKind::GAP,
            flags: [
                self.endianness_flag,
                false,
                false,
                false,
                false,
                false,
                false,
                false,
            ],
            submessage_length: submessage_length as u16,
        }
    }

    fn serialize_submessage_elements<W: Write, B: ByteOrder>(
        &self,
        mut writer: W,
    ) -> serialize::Result {
        self.reader_id.write_byte_ordered::<_, B>(&mut writer)?;
        self.writer_id.write_byte_ordered::<_, B>(&mut writer)?;
        self.gap_start.write_byte_ordered::<_, B>(&mut writer)?;
        self.gap_list.write_byte_ordered::<_, B>(&mut writer)
    }
}

impl<'de> DeserializeSubmessage<'de> for GapSubmessageRead {
    fn deserialize_submessage<B: ByteOrder>(
        buf: &mut &'de [u8],
        header: RtpsSubmessageHeader,
    ) -> deserialize::Result<Self> {
        let reader_id = MappingReadByteOrdered::read_byte_ordered::<B>(buf)?;
        let writer_id = MappingReadByteOrdered::read_byte_ordered::<B>(buf)?;
        let gap_start = MappingReadByteOrdered::read_byte_ordered::<B>(buf)?;
        let gap_list = MappingReadByteOrdered::read_byte_ordered::<B>(buf)?;
        Ok(Self::new(
            header.flags[0],
            reader_id,
            writer_id,
            gap_start,
            gap_list,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{deserialize::from_bytes, serialize::to_bytes};

    use super::*;
    use rust_rtps_pim::{
        messages::submessage_elements::{
            EntityIdSubmessageElement, SequenceNumberSetSubmessageElement,
            SequenceNumberSubmessageElement,
        },
        structure::types::{EntityId, USER_DEFINED_READER_GROUP, USER_DEFINED_READER_NO_KEY},
    };
    #[test]
    fn serialize_gap() {
        let endianness_flag = true;
        let reader_id = EntityIdSubmessageElement {
            value: EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY),
        };
        let writer_id = EntityIdSubmessageElement {
            value: EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP),
        };
        let gap_start = SequenceNumberSubmessageElement { value: 5 };
        let gap_list = SequenceNumberSetSubmessageElement {
            base: 10,
            set: vec![],
        };
        let submessage =
            GapSubmessageWrite::new(endianness_flag, reader_id, writer_id, gap_start, gap_list);
        #[rustfmt::skip]
        assert_eq!(to_bytes(&submessage).unwrap(), vec![
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
        let reader_id = EntityIdSubmessageElement {
            value: EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY),
        };
        let writer_id = EntityIdSubmessageElement {
            value: EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP),
        };
        let gap_start = SequenceNumberSubmessageElement { value: 5 };
        let gap_list = SequenceNumberSetSubmessageElement {
            base: 10,
            set: vec![],
        };
        let expected =
            GapSubmessageRead::new(endianness_flag, reader_id, writer_id, gap_start, gap_list);
        #[rustfmt::skip]
        let result = from_bytes(&[
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
