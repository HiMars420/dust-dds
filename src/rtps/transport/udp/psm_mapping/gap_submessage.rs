

use crate::rtps::messages::submessages::Gap;
use crate::rtps::messages::submessages::SubmessageHeader;

use super::{UdpPsmMappingResult, };
use super::submessage_elements::{serialize_entity_id, deserialize_entity_id, serialize_sequence_number, deserialize_sequence_number, serialize_sequence_number_set, deserialize_sequence_number_set};

pub fn serialize_gap(gap: &Gap, writer: &mut impl std::io::Write) -> UdpPsmMappingResult<()> {
    let endianness = gap.endianness_flag().into();
    serialize_entity_id(&gap.reader_id(), writer)?;
    serialize_entity_id(&gap.writer_id(), writer)?;
    serialize_sequence_number(&gap.gap_start(), writer, endianness)?;
    serialize_sequence_number_set(gap.gap_list(), writer, endianness)?;
    Ok(())
}

pub fn deserialize_gap(bytes: &[u8], header: SubmessageHeader) -> UdpPsmMappingResult<Gap> { 
    
    let flags = header.flags();
    // X|X|X|X|X|X|X|E
    /*E*/ let endianness_flag = flags[0];

    let endianness = endianness_flag.into();

    let reader_id = deserialize_entity_id(&bytes[0..4])?;
    let writer_id = deserialize_entity_id(&bytes[4..8])?;
    let gap_start = deserialize_sequence_number(&bytes[8..16], endianness)?;
    let gap_list = deserialize_sequence_number_set(&bytes[16..], endianness)?;
        

    Ok(Gap::from_raw_parts(endianness_flag, reader_id, writer_id, gap_start, gap_list))
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtps::types::{EntityKind, EntityId};
    use crate::rtps::messages::types::Endianness;
    use crate::rtps::messages::submessages::Submessage;
    
    #[test]
    fn serialize_gap_submessage_big_endian() {
        let expected = vec![
            // 0x08, 0b00000000, 0, 32, // Header 
            0x10, 0x12, 0x14, 0x04, // readerId
            0x26, 0x24, 0x22, 0x02, // writerId
            0x00, 0x00, 0x00, 0x00, // gapStart
            0x00, 0x00, 0x04, 0xB0, // gapStart
            0x00, 0x00, 0x00, 0x00, // gapList base
            0x00, 0x00, 0x04, 0xD2, // gapList base
            0x00, 0x00, 0x00,    2, // gapList numBits
            0b11000000, 0x00, 0x00, 0x00, // gapList bitmap
        ];

        let gap = Gap::new(
            Endianness::BigEndian, 
            EntityId::new([0x10, 0x12, 0x14], EntityKind::UserDefinedReaderWithKey),
            EntityId::new([0x26, 0x24, 0x22], EntityKind::UserDefinedWriterWithKey),
            1200,
            [1234, 1235,].iter().cloned().collect(),
        );

        let mut writer = Vec::new();
        serialize_gap(&gap, &mut writer).unwrap();
        assert_eq!(expected, writer);        

        let deserialized_gap = deserialize_gap(&writer, gap.submessage_header(expected.len() as u16)).unwrap();
        assert_eq!(gap, deserialized_gap);
    }

    #[test]
    fn serialize_gap_submessage_little_endian() {
        let expected = vec![
            // 0x08, 0b00000001, 32, 0, // Header 
            0x10, 0x12, 0x14, 0x04, // readerId
            0x26, 0x24, 0x22, 0x02, // writerId
            0x00, 0x00, 0x00, 0x00, // gapStart
            0xB0, 0x04, 0x00, 0x00, // gapStart
            0x00, 0x00, 0x00, 0x00, // gapList base
            0xD2, 0x04, 0x00, 0x00, // gapList base
               2, 0x00, 0x00, 0x00, // gapList numBits
            0x00, 0x00, 0x00, 0b11000000, // gapList bitmap
        ];

        let gap = Gap::new(
            Endianness::LittleEndian, 
            EntityId::new([0x10, 0x12, 0x14], EntityKind::UserDefinedReaderWithKey),
            EntityId::new([0x26, 0x24, 0x22], EntityKind::UserDefinedWriterWithKey),
            1200,
            [1234, 1235,].iter().cloned().collect(),
        );

        let mut writer = Vec::new();
        serialize_gap(&gap, &mut writer).unwrap();
        assert_eq!(expected, writer);

        let deserialized_gap = deserialize_gap(&writer, gap.submessage_header(expected.len() as u16)).unwrap();
        assert_eq!(gap, deserialized_gap);
    }
}