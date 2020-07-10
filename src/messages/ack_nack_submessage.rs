use std::collections::BTreeSet;

use super::serdes::{SubmessageElement, Endianness, RtpsSerdesResult, };
use super::{SubmessageKind, SubmessageFlag, UdpPsmMapping, };
use super::submessage::{Submessage, SubmessageHeader, };
use super::submessage_elements;
use crate::types;
use crate::messages;

#[derive(PartialEq, Debug)]
pub struct AckNack {
    endianness_flag: SubmessageFlag,
    final_flag: SubmessageFlag,
    reader_id: submessage_elements::EntityId,
    writer_id: submessage_elements::EntityId,
    reader_sn_state: submessage_elements::SequenceNumberSet,
    count: submessage_elements::Count,
}

impl AckNack {
    pub fn new(
        reader_id: types::EntityId,
        writer_id: types::EntityId,
        available_changes_max: types::SequenceNumber,
        missing_changes: BTreeSet<types::SequenceNumber>,
        count: messages::types::Count,
        final_flag: bool,
        endianness_flag: Endianness) -> Self {
            AckNack {
                reader_id: submessage_elements::EntityId(reader_id),
                writer_id: submessage_elements::EntityId(writer_id),
                reader_sn_state: submessage_elements::SequenceNumberSet::new(available_changes_max, missing_changes),
                count: submessage_elements::Count(count),
                final_flag,
                endianness_flag: endianness_flag.into(),
            }
        }

        pub fn reader_id(&self) -> &submessage_elements::EntityId {
            &self.reader_id
        }

        pub fn writer_id(&self) -> &submessage_elements::EntityId {
            &self.writer_id
        }

        pub fn reader_sn_state(&self) -> &submessage_elements::SequenceNumberSet {
            &self.reader_sn_state
        }

        pub fn count(&self) -> &submessage_elements::Count {
            &self.count
        }
}

impl Submessage for AckNack {
    fn submessage_header(&self) -> SubmessageHeader {
        const X : SubmessageFlag = false;
        let e = self.endianness_flag; 
        let f = self.final_flag; 
        let flags = [e, f, X, X, X, X, X, X];     
        let submessage_length = self.reader_id.octets() + self.writer_id.octets() + self.reader_sn_state.octets() + self.count.octets();
        SubmessageHeader::new( 
            SubmessageKind::InfoReply,
            flags,
            submessage_length)
    }

    fn is_valid(&self) -> bool {
        self.reader_sn_state.is_valid()
    }
}

impl UdpPsmMapping for AckNack {
    fn compose(&self, writer: &mut impl std::io::Write) -> RtpsSerdesResult<()> {
        let endianness = Endianness::from(self.endianness_flag);       
        self.submessage_header().compose(writer)?;
        self.reader_id.serialize(writer, endianness)?;
        self.writer_id.serialize(writer, endianness)?;
        self.reader_sn_state.serialize(writer, endianness)?;
        self.count.serialize(writer, endianness)?;        
        Ok(())
    }

    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        let header = SubmessageHeader::parse(bytes)?;
        let endianness_flag = header.flags()[0];
        let final_flag = header.flags()[1];
        let endianness = endianness_flag.into();
        let end_of_message = usize::from(header.submessage_length()) + header.octets();
        let index_count = end_of_message - 4;
        let reader_id = submessage_elements::EntityId::deserialize(&bytes[4..8], endianness)?;
        let writer_id = submessage_elements::EntityId::deserialize(&bytes[8..12], endianness)?;
        let reader_sn_state = submessage_elements::SequenceNumberSet::deserialize(&bytes[12..index_count], endianness)?;
        let count = submessage_elements::Count::deserialize(&bytes[index_count..end_of_message], endianness)?;
        
        Ok(Self{endianness_flag, final_flag, reader_id, writer_id, reader_sn_state, count})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::constants::{ENTITYID_UNKNOWN, ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER, };
    
    #[test]
    fn test_parse_ack_nack_submessage() {
        let bytes = [
            0x0f, 0b00000011, 28, 0, 
            0x00, 0x00, 0x00, 0x00, // readerId 
            0x00, 0x01, 0x00, 0xc2, // writerId
            0, 0, 0, 0, // reader_sn_state: base
            2, 0, 0, 0, // reader_sn_state: base
            2, 0, 0, 0, // reader_sn_state: num bits
            0b_00000000, 0b_00000000, 0b_00000000, 0b_11000000, // reader_sn_state: bitmap
            2, 0, 0, 0, // Count
        ];
        
        let expected = AckNack {
            endianness_flag: Endianness::LittleEndian.into(),
            final_flag: true,
            reader_id: submessage_elements::EntityId(ENTITYID_UNKNOWN),
            writer_id: submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            reader_sn_state: submessage_elements::SequenceNumberSet::from_set([2,3].iter().cloned().collect()),
            count: submessage_elements::Count(2),
        };
        let result = AckNack::parse(&bytes).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn compose_gap_submessage() {
        let expected = vec![
            0x0f, 0b00000011, 28, 0, 
            0x00, 0x00, 0x00, 0x00, // readerId 
            0x00, 0x01, 0x00, 0xc2, // writerId
            0, 0, 0, 0, // reader_sn_state: base
            2, 0, 0, 0, // reader_sn_state: base
            2, 0, 0, 0, // reader_sn_state: num bits
            0b_00000000, 0b_00000000, 0b_00000000, 0b_11000000, // reader_sn_state: bitmap
            2, 0, 0, 0, // Count
        ];

        let message = AckNack {
            endianness_flag: Endianness::LittleEndian.into(),
            final_flag: true,
            reader_id: submessage_elements::EntityId(ENTITYID_UNKNOWN),
            writer_id: submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            reader_sn_state: submessage_elements::SequenceNumberSet::from_set([2, 3].iter().cloned().collect()),
            count: submessage_elements::Count(2),
        };

        let mut writer = Vec::new();
        message.compose(&mut writer).unwrap();
        assert_eq!(expected, writer);        
    }
}
