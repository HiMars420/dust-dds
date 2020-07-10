use self::types::constants::PROTOCOL_RTPS;
use self::types::{ProtocolId, SubmessageFlag, SubmessageKind};
use crate::serdes::{
    Endianness, RtpsCompose, RtpsDeserialize, RtpsParse, RtpsSerdesResult, RtpsSerialize,
};
use crate::types::constants::{PROTOCOL_VERSION_2_4, VENDOR_ID};
use crate::types::{GuidPrefix, ProtocolVersion, VendorId};
use std::convert::TryInto;

mod ack_nack_submessage;
mod data_frag_submessage;
mod data_submessage;
mod gap_submessage;
mod heartbeat_frag_submessage;
mod heartbeat_submessage;
mod info_destination_submessage;
mod info_reply_submessage;
mod info_source_submessage;
mod info_timestamp_submessage;
mod nack_frag_submessage;
pub mod submessage_elements;
pub mod types;

pub use ack_nack_submessage::AckNack;
// pub use data_frag_submessage::DataFrag;
pub use data_submessage::Data;
pub use data_submessage::Payload;
pub use gap_submessage::Gap;
// pub use heartbeat_frag_submessage::HeartbeatFrag;
pub use heartbeat_submessage::Heartbeat;
// pub use info_destination_submessage::InfoDst;
// pub use info_reply_submessage::InfoReply;
// pub use info_source_submessage::InfoSrc;
pub use info_timestamp_submessage::InfoTs;
// pub use nack_frag_submessage::NackFrag;

pub const RTPS_MAJOR_VERSION: u8 = 2;
pub const RTPS_MINOR_VERSION: u8 = 4;

#[derive(Debug, PartialEq)]
pub enum RtpsSubmessage {
    AckNack(AckNack),
    Data(Data),
    // DataFrag(DataFrag),
    Gap(Gap),
    Heartbeat(Heartbeat),
    // HeartbeatFrag(HeartbeatFrag),
    // InfoDst(InfoDst),
    // InfoReply(InfoReply),
    // InfoSrc(InfoSrc),
    InfoTs(InfoTs),
    // NackFrag(NackFrag),
}

impl RtpsCompose for RtpsSubmessage {
    fn compose(&self, writer: &mut impl std::io::Write) -> RtpsSerdesResult<()> {
        match self {
            RtpsSubmessage::AckNack(acknack) => acknack.compose(writer),
            RtpsSubmessage::Data(data) => data.compose(writer),
            RtpsSubmessage::Gap(gap) => gap.compose(writer),
            RtpsSubmessage::Heartbeat(heartbeat) => heartbeat.compose(writer),
            RtpsSubmessage::InfoTs(infots) => infots.compose(writer),
        }
    }
}

impl RtpsParse for RtpsSubmessage {
    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        let submessage_id =
            SubmessageKind::deserialize(&[bytes[0]], Endianness::LittleEndian /*irrelevant*/)?;
        match submessage_id {
            SubmessageKind::Data => Ok(RtpsSubmessage::Data(Data::parse(bytes)?)),
            SubmessageKind::Pad => todo!(),
            SubmessageKind::AckNack => Ok(RtpsSubmessage::AckNack(AckNack::parse(bytes)?)),
            SubmessageKind::Heartbeat => Ok(RtpsSubmessage::Heartbeat(Heartbeat::parse(bytes)?)),
            SubmessageKind::Gap => Ok(RtpsSubmessage::Gap(Gap::parse(bytes)?)),
            SubmessageKind::InfoTimestamp => Ok(RtpsSubmessage::InfoTs(InfoTs::parse(bytes)?)),
            SubmessageKind::InfoSource => todo!(),
            SubmessageKind::InfoReplyIP4 => todo!(),
            SubmessageKind::InfoDestination => todo!(),
            SubmessageKind::InfoReply => todo!(),
            SubmessageKind::NackFrag => todo!(),
            SubmessageKind::HeartbeatFrag => todo!(),
            SubmessageKind::DataFrag => todo!(),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct SubmessageHeader {
    submessage_id: SubmessageKind,
    flags: [SubmessageFlag; 8],
    submessage_length: u16,
}

impl SubmessageHeader {
    pub fn submessage_id(&self) -> SubmessageKind {
        self.submessage_id
    }
    pub fn flags(&self) -> &[SubmessageFlag; 8] {
        &self.flags
    }
    pub fn submessage_length(&self) -> u16 {
        self.submessage_length
    }
}

impl RtpsCompose for SubmessageHeader {
    fn compose(&self, writer: &mut impl std::io::Write) -> RtpsSerdesResult<()> {
        let endianness = Endianness::from(self.flags[0]);
        self.submessage_id.serialize(writer, endianness)?;
        self.flags.serialize(writer, endianness)?;
        match endianness {
            Endianness::LittleEndian => writer.write(&self.submessage_length.to_le_bytes())?,
            Endianness::BigEndian => writer.write(&self.submessage_length.to_be_bytes())?,
        };
        Ok(())
    }
}

impl RtpsParse for SubmessageHeader {
    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        let submessage_id = SubmessageKind::deserialize(
            &bytes[0..1],
            Endianness::LittleEndian, /*irrelevant*/
        )?;
        let flags = <[SubmessageFlag; 8]>::deserialize(
            &bytes[1..2],
            Endianness::LittleEndian, /*irrelevant*/
        )?;
        let endianness = Endianness::from(flags[0]);
        let submessage_length = match endianness {
            Endianness::LittleEndian => u16::from_le_bytes(bytes[2..4].try_into()?),
            Endianness::BigEndian => u16::from_be_bytes(bytes[2..4].try_into()?),
        };
        Ok(SubmessageHeader {
            submessage_id,
            flags,
            submessage_length,
        })
    }
}

pub trait Submessage {
    fn submessage_header(&self) -> SubmessageHeader;

    fn is_valid(&self) -> bool;
}

#[derive(PartialEq, Debug)]
pub struct Header {
    protocol: ProtocolId,
    version: ProtocolVersion,
    vendor_id: VendorId,
    guid_prefix: GuidPrefix,
}

impl Header {
    pub fn new(guid_prefix: GuidPrefix) -> Self {
        Self {
            protocol: PROTOCOL_RTPS,
            version: PROTOCOL_VERSION_2_4,
            vendor_id: VENDOR_ID,
            guid_prefix,
        }
    }
    pub fn guid_prefix(&self) -> &GuidPrefix {
        &self.guid_prefix
    }
}

impl RtpsCompose for Header {
    fn compose(&self, writer: &mut impl std::io::Write) -> RtpsSerdesResult<()> {
        writer.write(&self.protocol)?;
        writer.write(&[self.version.major])?;
        writer.write(&[self.version.minor])?;
        writer.write(&self.vendor_id)?;
        writer.write(&self.guid_prefix)?;
        Ok(())
    }
}

impl RtpsParse for Header {
    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        let protocol = bytes[0..4].try_into()?;
        let version = ProtocolVersion {
            major: bytes[4],
            minor: bytes[5],
        };
        let vendor_id = bytes[6..8].try_into()?;
        let guid_prefix = bytes[8..20].try_into()?;
        Ok(Header {
            protocol,
            version,
            vendor_id,
            guid_prefix,
        })
    }
}

#[derive(PartialEq, Debug)]
pub struct RtpsMessage {
    header: Header,
    submessages: Vec<RtpsSubmessage>,
}

impl RtpsMessage {
    pub fn new(guid_prefix: GuidPrefix, submessages: Vec<RtpsSubmessage>) -> Self {
        if submessages.is_empty() {
            panic!("At least one submessage is required");
        };

        RtpsMessage {
            header: Header::new(guid_prefix),
            submessages,
        }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn submessages(&self) -> &Vec<RtpsSubmessage> {
        &self.submessages
    }
}

impl RtpsCompose for RtpsMessage {
    fn compose(&self, writer: &mut impl std::io::Write) -> RtpsSerdesResult<()> {
        &self.header.compose(writer)?;
        for submessage in &self.submessages {
            submessage.compose(writer)?;
        }
        Ok(())
    }
}

impl RtpsParse for RtpsMessage {
    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        let size = bytes.len();
        let header = Header::parse(bytes)?;

        let mut submessage_start_index: usize = header.octets();
        let mut submessages = Vec::<RtpsSubmessage>::new();

        while submessage_start_index < size {
            let submessage = RtpsSubmessage::parse(&bytes[submessage_start_index..])?;

            submessage_start_index += submessage.octets();
            submessages.push(submessage);
        }
        Ok(RtpsMessage {
            header,
            submessages,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::types::Time;
    use super::*;
    use crate::types::constants;
    use crate::types::constants::{ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER, ENTITYID_UNKNOWN};
    use crate::serdes::RtpsSerdesError;

    #[test]
    fn test_parse_message_header() {
        let expected = Header {
            protocol: [b'R', b'T', b'P', b'S'],
            version: ProtocolVersion { major: 2, minor: 1 },
            vendor_id: [1, 2],
            guid_prefix: [127, 32, 247, 215, 0, 0, 1, 187, 0, 0, 0, 1],
        };

        let bytes = [
            0x52, 0x54, 0x50, 0x53, //000 protocol: ProtocolId_t => 'R', 'T', 'P', 'S',
            0x02, 0x01, 0x01,
            0x02, //004 version: ProtocolVersion_t => 2.1 | vendorId: VendorId_t => 1,2
            0x7f, 0x20, 0xf7, 0xd7, //008 guidPrefix: GuidPrefix_t => 127, 32, 247, 215
            0x00, 0x00, 0x01, 0xbb, //012 guidPrefix: GuidPrefix_t => 0, 0, 1, 187
            0x00, 0x00, 0x00, 0x01, //016 guidPrefix: GuidPrefix_t => 0, 0, 0, 1
        ];
        let result = Header::parse(&bytes).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_submessage_header() {
        let bytes = [0x15_u8, 0b00000001, 20, 0x0];
        let f = false;
        let flags: [SubmessageFlag; 8] = [true, f, f, f, f, f, f, f];
        let expected = SubmessageHeader {
            submessage_id: SubmessageKind::Data,
            flags,
            submessage_length: 20,
        };
        let result = SubmessageHeader::parse(&bytes);

        assert_eq!(expected, result.unwrap());
    }

    #[test]
    fn test_compose_submessage_header() {
        let mut result = Vec::new();

        let f = false;
        let t = true;
        let header = SubmessageHeader {
            submessage_id: SubmessageKind::Data,
            flags: [t, t, f, f, f, f, f, f],
            submessage_length: 16,
        };
        let expected = vec![0x15, 0b00000011, 16, 0x0];
        header.compose(&mut result).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compose_submessage() {
        let submessage = RtpsSubmessage::Data(Data::new(
            Endianness::LittleEndian,
            submessage_elements::EntityId(ENTITYID_UNKNOWN),
            submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            submessage_elements::SequenceNumber(1),
            None,
            Payload::None,
        ));

        let expected = vec![
            0x15_u8, 0b00000001, 20, 0x0, // Submessgae Header
            0x00, 0x00, 16, 0x0, // ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
        ];

        let mut writer = Vec::new();
        submessage.compose(&mut writer).unwrap();
        assert_eq!(expected, writer);
    }

    #[test]
    fn test_parse_submessage() {
        let bytes = vec![
            0x15_u8, 0b00000001, 20, 0x0, // Submessgae Header
            0x00, 0x00, 16, 0x0, // ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
        ];
        let expected = RtpsSubmessage::Data(Data::new(
            Endianness::LittleEndian,
            submessage_elements::EntityId(ENTITYID_UNKNOWN),
            submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            submessage_elements::SequenceNumber(1),
            None,
            Payload::None,
        ));
        let result = RtpsSubmessage::parse(&bytes).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_compose_message() {
        let submessage = RtpsSubmessage::Data(Data::new(
            Endianness::LittleEndian,
            submessage_elements::EntityId(ENTITYID_UNKNOWN),
            submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            submessage_elements::SequenceNumber(1),
            None,
            Payload::None,
        ));
        let message = RtpsMessage {
            header: Header {
                protocol: [b'R', b'T', b'P', b'S'],
                version: ProtocolVersion { major: 2, minor: 1 },
                vendor_id: [1, 2],
                guid_prefix: [127, 32, 247, 215, 0, 0, 1, 187, 0, 0, 0, 1],
            },
            submessages: vec![submessage],
        };

        let expected = vec![
            0x52, 0x54, 0x50, 0x53, //000 protocol: ProtocolId_t => 'R', 'T', 'P', 'S',
            0x02, 0x01, 0x01,
            0x02, //004 version: ProtocolVersion_t => 2.1 | vendorId: VendorId_t => 1,2
            0x7f, 0x20, 0xf7, 0xd7, //008 guidPrefix: GuidPrefix_t => 127, 32, 247, 215
            0x00, 0x00, 0x01, 0xbb, //012 guidPrefix: GuidPrefix_t => 0, 0, 1, 187
            0x00, 0x00, 0x00, 0x01, //016 guidPrefix: GuidPrefix_t => 0, 0, 0, 1
            0x15_u8, 0b00000001, 20, 0x0, // Submessgae Header
            0x00, 0x00, 16, 0x0, // ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
        ];
        let mut writer = Vec::new();
        message.compose(&mut writer).unwrap();
        assert_eq!(expected, writer);
    }

    #[test]
    fn test_compose_message_three_data_submessages() {
        let test_time = submessage_elements::Timestamp(Time::new(1565525425, 269558339));
        let submessage1 =
            RtpsSubmessage::InfoTs(InfoTs::new(Some(test_time), Endianness::LittleEndian));
        let submessage2 = RtpsSubmessage::Data(Data::new(
            Endianness::LittleEndian,
            submessage_elements::EntityId(ENTITYID_UNKNOWN),
            submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            submessage_elements::SequenceNumber(1),
            None,
            Payload::None,
        ));
        let submessage3 = RtpsSubmessage::Data(Data::new(
            Endianness::LittleEndian,
            submessage_elements::EntityId(ENTITYID_UNKNOWN),
            submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            submessage_elements::SequenceNumber(2),
            None,
            Payload::None,
        ));
        let message = RtpsMessage {
            header: Header {
                protocol: [b'R', b'T', b'P', b'S'],
                version: ProtocolVersion { major: 2, minor: 1 },
                vendor_id: [1, 2],
                guid_prefix: [127, 32, 247, 215, 0, 0, 1, 187, 0, 0, 0, 1],
            },
            submessages: vec![submessage1, submessage2, submessage3],
        };

        let expected = vec![
            0x52, 0x54, 0x50, 0x53, //000 protocol: ProtocolId_t => 'R', 'T', 'P', 'S',
            0x02, 0x01, 0x01,
            0x02, //004 version: ProtocolVersion_t => 2.1 | vendorId: VendorId_t => 1,2
            0x7f, 0x20, 0xf7, 0xd7, //008 guidPrefix: GuidPrefix_t => 127, 32, 247, 215
            0x00, 0x00, 0x01, 0xbb, //012 guidPrefix: GuidPrefix_t => 0, 0, 1, 187
            0x00, 0x00, 0x00, 0x01, //016 guidPrefix: GuidPrefix_t => 0, 0, 0, 1
            0x09, 0x01, 0x08, 0x00, // [Info Timestamp] Submessgae Header
            0xB1, 0x05, 0x50, 0x5D, // [Info Timestamp]
            0x43, 0x22, 0x11, 0x10, // [Info Timestamp]
            0x15_u8, 0b00000001, 20, 0x0, // Submessgae Header
            0x00, 0x00, 16, 0x0, // ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
            0x15_u8, 0b00000001, 20, 0x0, // Submessgae Header
            0x00, 0x00, 16, 0x0, // ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x02, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 2
        ];
        let mut writer = Vec::new();
        message.compose(&mut writer).unwrap();
        assert_eq!(expected, writer);
    }

    #[test]
    fn test_parse_message() {
        let submessage = RtpsSubmessage::Data(Data::new(
            Endianness::LittleEndian,
            submessage_elements::EntityId(ENTITYID_UNKNOWN),
            submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            submessage_elements::SequenceNumber(1),
            None,
            Payload::None,
        ));
        let expected = RtpsMessage {
            header: Header {
                protocol: [b'R', b'T', b'P', b'S'],
                version: ProtocolVersion { major: 2, minor: 1 },
                vendor_id: [1, 2],
                guid_prefix: [127, 32, 247, 215, 0, 0, 1, 187, 0, 0, 0, 1],
            },
            submessages: vec![submessage],
        };

        let bytes = [
            0x52, 0x54, 0x50, 0x53, //000 protocol: ProtocolId_t => 'R', 'T', 'P', 'S',
            0x02, 0x01, 0x01,
            0x02, //004 version: ProtocolVersion_t => 2.1 | vendorId: VendorId_t => 1,2
            0x7f, 0x20, 0xf7, 0xd7, //008 guidPrefix: GuidPrefix_t => 127, 32, 247, 215
            0x00, 0x00, 0x01, 0xbb, //012 guidPrefix: GuidPrefix_t => 0, 0, 1, 187
            0x00, 0x00, 0x00, 0x01, //016 guidPrefix: GuidPrefix_t => 0, 0, 0, 1
            0x15, 0b00000001, 20, 0x0, // Submessgae Header
            0x00, 0x00, 16,
            0x0, // [Data Submessage] ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
        ];
        let result = RtpsMessage::parse(&bytes).unwrap();
        assert_eq!(expected, result);

        let submessage1 = RtpsSubmessage::Data(Data::new(
            Endianness::LittleEndian,
            submessage_elements::EntityId(ENTITYID_UNKNOWN),
            submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            submessage_elements::SequenceNumber(1),
            None,
            Payload::None,
        ));
        let reader_id = submessage_elements::EntityId(crate::types::EntityId::new(
            [0x10, 0x12, 0x14],
            constants::USER_DEFINED_READER_WITH_KEY,
        ));
        let writer_id = submessage_elements::EntityId(crate::types::EntityId::new(
            [0x26, 0x24, 0x22],
            constants::USER_DEFINED_WRITER_WITH_KEY,
        ));
        let first_sn = submessage_elements::SequenceNumber(1233);
        let last_sn = submessage_elements::SequenceNumber(1237);
        let count = submessage_elements::Count(8);
        let final_flag = true;
        let liveliness_flag = false;

        let submessage2 = RtpsSubmessage::Heartbeat(Heartbeat::new(
            reader_id,
            writer_id,
            first_sn,
            last_sn,
            count,
            final_flag,
            liveliness_flag,
            Endianness::BigEndian,
        ));
        let expected = RtpsMessage {
            header: Header {
                protocol: [b'R', b'T', b'P', b'S'],
                version: ProtocolVersion { major: 2, minor: 1 },
                vendor_id: [1, 2],
                guid_prefix: [127, 32, 247, 215, 0, 0, 1, 187, 0, 0, 0, 1],
            },
            submessages: vec![submessage1, submessage2],
        };
        let bytes = [
            0x52, 0x54, 0x50, 0x53, // protocol: ProtocolId_t => 'R', 'T', 'P', 'S',
            0x02, 0x01, 0x01,
            0x02, // version: ProtocolVersion_t => 2.1 | vendorId: VendorId_t => 1,2
            0x7f, 0x20, 0xf7, 0xd7, // guidPrefix: GuidPrefix_t => 127, 32, 247, 215
            0x00, 0x00, 0x01, 0xbb, // guidPrefix: GuidPrefix_t => 0, 0, 1, 187
            0x00, 0x00, 0x00, 0x01, // guidPrefix: GuidPrefix_t => 0, 0, 0, 1
            0x15, 0b00000001, 20, 0x0, // Submessgae Header => Data
            0x00, 0x00, 16,
            0x0, // [Data Submessage] ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
            0x07, 0b00000010, 0x00, 28, // Submessage Header => Heartbeat
            0x10, 0x12, 0x14, 0x04, // [Heartbeat Submessage] Reader ID
            0x26, 0x24, 0x22, 0x02, // [Heartbeat Submessage] Writer ID
            0x00, 0x00, 0x00, 0x00, // [Heartbeat Submessage] First Sequence Number
            0x00, 0x00, 0x04, 0xD1, // [Heartbeat Submessage] First Sequence Number
            0x00, 0x00, 0x00, 0x00, // [Heartbeat Submessage] Last Sequence Number
            0x00, 0x00, 0x04, 0xD5, // [Heartbeat Submessage] Last Sequence Number
            0x00, 0x00, 0x00, 0x08, // [Heartbeat Submessage] Count
        ];
        let result = RtpsMessage::parse(&bytes).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_message_two_data_submessages() {
        let test_time = submessage_elements::Timestamp(Time::new(1565525425, 269558339));
        let submessage1 =
            RtpsSubmessage::InfoTs(InfoTs::new(Some(test_time), Endianness::LittleEndian));
        let submessage2 = RtpsSubmessage::Data(Data::new(
            Endianness::LittleEndian,
            submessage_elements::EntityId(ENTITYID_UNKNOWN),
            submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            submessage_elements::SequenceNumber(1),
            None,
            Payload::None,
        ));
        let submessage3 = RtpsSubmessage::Data(Data::new(
            Endianness::LittleEndian,
            submessage_elements::EntityId(ENTITYID_UNKNOWN),
            submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            submessage_elements::SequenceNumber(2),
            None,
            Payload::None,
        ));
        let expected = RtpsMessage {
            header: Header {
                protocol: [b'R', b'T', b'P', b'S'],
                version: ProtocolVersion { major: 2, minor: 1 },
                vendor_id: [1, 2],
                guid_prefix: [127, 32, 247, 215, 0, 0, 1, 187, 0, 0, 0, 1],
            },
            submessages: vec![submessage1, submessage2, submessage3],
        };

        let bytes = [
            0x52, 0x54, 0x50, 0x53, // protocol: ProtocolId_t => 'R', 'T', 'P', 'S',
            0x02, 0x01, 0x01,
            0x02, // version: ProtocolVersion_t => 2.1 | vendorId: VendorId_t => 1,2
            0x7f, 0x20, 0xf7, 0xd7, // guidPrefix: GuidPrefix_t => 127, 32, 247, 215
            0x00, 0x00, 0x01, 0xbb, // guidPrefix: GuidPrefix_t => 0, 0, 1, 187
            0x00, 0x00, 0x00, 0x01, // guidPrefix: GuidPrefix_t => 0, 0, 0, 1
            0x09, 0x01, 0x08, 0x00, // [Info Timestamp] Submessgae Header
            0xB1, 0x05, 0x50, 0x5D, // [Info Timestamp]
            0x43, 0x22, 0x11, 0x10, // [Info Timestamp]
            0x15, 0b00000001, 20, 0x0, // [Data Submessage] Submessgae Header
            0x00, 0x00, 16,
            0x0, // [Data Submessage] ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
            0x15, 0b00000001, 20, 0x0, // [Data Submessage] Submessgae Header
            0x00, 0x00, 16,
            0x0, // [Data Submessage] ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x02, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
        ];
        let result = RtpsMessage::parse(&bytes).unwrap();
        assert_eq!(expected, result);
    }

    // #[test]
    // fn serialize_deserialize_ushort(){
    //     let mut buf = Vec::new();

    //     let val: UShort = 123;

    //     serialize_ushort(val, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [123, 0]);
    //     assert_eq!(deserialize_ushort(&buf, Endianness::LittleEndian).unwrap(), val);
    //     buf.clear();

    //     serialize_ushort(val, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0, 123]);
    //     assert_eq!(deserialize_ushort(&buf, Endianness::BigEndian).unwrap(), val);
    //     buf.clear();


    //     let max: UShort = UShort::MAX;

    //     serialize_ushort(max, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [0xFF, 0xFF]);
    //     assert_eq!(deserialize_ushort(&buf, Endianness::LittleEndian).unwrap(), max);
    //     buf.clear();

    //     serialize_ushort(max, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0xFF, 0xFF]);
    //     assert_eq!(deserialize_ushort(&buf, Endianness::BigEndian).unwrap(), max);
    //     buf.clear();

    //     let min: UShort = UShort::MIN;

    //     serialize_ushort(min, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [0x00, 0x00]);
    //     assert_eq!(deserialize_ushort(&buf, Endianness::LittleEndian).unwrap(), min);
    //     buf.clear();

    //     serialize_ushort(min, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0x00, 0x00]);
    //     assert_eq!(deserialize_ushort(&buf, Endianness::BigEndian).unwrap(), min);
    //     buf.clear();
    // }

    // #[test]
    // fn invalid_ushort_deserialize() {
    //     let buf: [u8; 1] = [1];
    //     let result = deserialize_ushort(&buf, Endianness::BigEndian);
    //     match result {
    //         Err(RtpsSerdesError::WrongSize) => assert!(true),
    //         _ => assert!(false),
    //     }
    // }

    // #[test]
    // fn serialize_deserialize_short(){
    //     let mut buf = Vec::new();

    //     let val: Short = 123;

    //     serialize_short(val, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [123, 0]);
    //     assert_eq!(deserialize_short(&buf, Endianness::LittleEndian).unwrap(), val);
    //     buf.clear();

    //     serialize_short(val, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0, 123]);
    //     assert_eq!(deserialize_short(&buf, Endianness::BigEndian).unwrap(), val);
    //     buf.clear();


    //     let max: Short = Short::MAX;

    //     serialize_short(max, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [0xFF, 0x7F]);
    //     assert_eq!(deserialize_short(&buf, Endianness::LittleEndian).unwrap(), max);
    //     buf.clear();

    //     serialize_short(max, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0x7F, 0xFF]);
    //     assert_eq!(deserialize_short(&buf, Endianness::BigEndian).unwrap(), max);
    //     buf.clear();

    //     let min: Short = Short::MIN;

    //     serialize_short(min, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [0x00, 0x80]);
    //     assert_eq!(deserialize_short(&buf, Endianness::LittleEndian).unwrap(), min);
    //     buf.clear();

    //     serialize_short(min, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0x80, 0x00]);
    //     assert_eq!(deserialize_short(&buf, Endianness::BigEndian).unwrap(), min);
    //     buf.clear();
    // }

    // #[test]
    // fn invalid_short_deserialize() {
    //     let buf: [u8; 1] = [1];
    //     let result = deserialize_short(&buf, Endianness::BigEndian);
    //     match result {
    //         Err(RtpsSerdesError::WrongSize) => assert!(true),
    //         _ => assert!(false),
    //     }
    // }

    // #[test]
    // fn serialize_deserialize_long(){
    //     let mut buf = Vec::new();

    //     let val: Long = 1230;

    //     serialize_long(val, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [0xCE, 0x04, 0, 0]);
    //     assert_eq!(deserialize_long(&buf, Endianness::LittleEndian).unwrap(), val);
    //     buf.clear();

    //     serialize_long(val, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0, 0, 0x04, 0xCE]);
    //     assert_eq!(deserialize_long(&buf, Endianness::BigEndian).unwrap(), val);
    //     buf.clear();


    //     let max: Long = Long::MAX;

    //     serialize_long(max, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [0xFF, 0xFF, 0xFF, 0x7F]);
    //     assert_eq!(deserialize_long(&buf, Endianness::LittleEndian).unwrap(), max);
    //     buf.clear();

    //     serialize_long(max, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0x7F, 0xFF, 0xFF, 0xFF]);
    //     assert_eq!(deserialize_long(&buf, Endianness::BigEndian).unwrap(), max);
    //     buf.clear();

    //     let min: Long = Long::MIN;

    //     serialize_long(min, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [0x00, 0x00, 0x00, 0x80]);
    //     assert_eq!(deserialize_long(&buf, Endianness::LittleEndian).unwrap(), min);
    //     buf.clear();

    //     serialize_long(min, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0x80, 0x00, 0x00, 0x00]);
    //     assert_eq!(deserialize_long(&buf, Endianness::BigEndian).unwrap(), min);
    //     buf.clear();
    // }

    // #[test]
    // fn invalid_long_deserialize() {
    //     let buf: [u8; 3] = [1, 2, 3];
    //     let result = deserialize_long(&buf, Endianness::BigEndian);
    //     match result {
    //         Err(RtpsSerdesError::WrongSize) => assert!(true),
    //         _ => assert!(false),
    //     }
    // }

    // #[test]
    // fn serialize_deserialize_ulong(){
    //     let mut buf = Vec::new();

    //     let val: ULong = 1230;

    //     serialize_ulong(val, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [0xCE, 0x04, 0, 0]);
    //     assert_eq!(deserialize_ulong(&buf, Endianness::LittleEndian).unwrap(), val);
    //     buf.clear();

    //     serialize_ulong(val, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0, 0, 0x04, 0xCE]);
    //     assert_eq!(deserialize_ulong(&buf, Endianness::BigEndian).unwrap(), val);
    //     buf.clear();


    //     let max: ULong = ULong::MAX;

    //     serialize_ulong(max, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [0xFF, 0xFF, 0xFF, 0xFF]);
    //     assert_eq!(deserialize_ulong(&buf, Endianness::LittleEndian).unwrap(), max);
    //     buf.clear();

    //     serialize_ulong(max, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0xFF, 0xFF, 0xFF, 0xFF]);
    //     assert_eq!(deserialize_ulong(&buf, Endianness::BigEndian).unwrap(), max);
    //     buf.clear();

    //     let min: ULong = ULong::MIN;

    //     serialize_ulong(min, &mut buf, Endianness::LittleEndian).unwrap();
    //     assert_eq!(buf, [0x00, 0x00, 0x00, 0x00]);
    //     assert_eq!(deserialize_ulong(&buf, Endianness::LittleEndian).unwrap(), min);
    //     buf.clear();

    //     serialize_ulong(min, &mut buf, Endianness::BigEndian).unwrap();
    //     assert_eq!(buf, [0x00, 0x00, 0x00, 0x00]);
    //     assert_eq!(deserialize_ulong(&buf, Endianness::BigEndian).unwrap(), min);
    //     buf.clear();
    // }

    // #[test]
    // fn invalid_ulong_deserialize() {
    //     let buf: [u8; 3] = [1, 2, 3];
    //     let result = deserialize_ulong(&buf, Endianness::BigEndian);
    //     match result {
    //         Err(RtpsSerdesError::WrongSize) => assert!(true),
    //         _ => assert!(false),
    //     }
    // }
}
