use super::serdes::{SubmessageElement, Endianness, RtpsSerdesResult, };
use super::{SubmessageKind, SubmessageFlag, UdpPsmMapping, };
use super::submessage::{Submessage, SubmessageHeader, };
use super::submessage_elements;
use crate::messages;

#[derive(PartialEq, Debug)]
pub struct InfoTs {
    endianness_flag: SubmessageFlag,
    invalidate_flag: SubmessageFlag,
    timestamp: Option<submessage_elements::Timestamp>,
}

impl InfoTs {
    const INVALID_TIME_FLAG_MASK: u8 = 0x02;

    pub fn new(time: Option<messages::types::Time>, endianness: Endianness) -> InfoTs {
        let endianness_flag = endianness.into();
        let invalidate_flag = !time.is_some();
        let timestamp = match time {
            Some(time) => Some(submessage_elements::Timestamp(time)),
            None => None,
        };
        InfoTs {
            endianness_flag,
            invalidate_flag,
            timestamp,
        }
    }

    pub fn time(&self) -> Option<messages::types::Time> {
        match self.invalidate_flag {
            true => None,
            false => Some((&self.timestamp).as_ref().unwrap().0),
        }
    }
}

impl Submessage for InfoTs {
    fn submessage_header(&self) -> SubmessageHeader {
        let x = false;
        let e = self.endianness_flag; // Indicates endianness.
        let i = self.invalidate_flag; // Indicates whether subsequent Submessages should be considered as having a timestamp or not.
        // X|X|X|X|X|X|I|E
        let flags = [e, i, x, x, x, x, x, x];

        let octets_to_next_header = if self.invalidate_flag {
            0
        } else {
            self.timestamp.octets()
        };
            
        SubmessageHeader::new( 
            SubmessageKind::InfoTimestamp,
            flags,
            octets_to_next_header)
    }
    
    fn is_valid(&self) -> bool {
        true
    }
}

impl UdpPsmMapping for InfoTs {
    fn compose(&self, writer: &mut impl std::io::Write) -> RtpsSerdesResult<()> {
        let endianness = Endianness::from(self.endianness_flag);
        self.submessage_header().compose(writer)?;
        match &self.timestamp {
            Some(timestamp) => timestamp.serialize(writer, endianness)?,
            None => (),
        };

        Ok(())
    }

    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        let header = SubmessageHeader::parse(bytes)?;
        let flags = header.flags();
        // X|X|X|X|X|X|I|E
        /*E*/ let endianness_flag = flags[0];
        /*I*/ let invalidate_flag = flags[1];

        let endianness = endianness_flag.into();
        if invalidate_flag {
            Ok(InfoTs{ invalidate_flag, endianness_flag, timestamp: None})
        } else {            
            let timestamp = Some(submessage_elements::Timestamp::deserialize(&bytes[4..12], endianness)?);
            Ok(InfoTs{invalidate_flag, endianness_flag, timestamp})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_infots() {
        // let mut writer_le = Vec::new();
        let mut writer = Vec::new();
        let info_timestamp_message_little_endian =
            [0x09, 0x01, 0x08, 0x00, 0xB1, 0x05, 0x50, 0x5D, 0x43, 0x22, 0x11, 0x10];
        let info_timestamp_message_big_endian = 
            [0x09, 0x00, 0x00, 0x08, 0x5D, 0x50, 0x05, 0xB1, 0x10, 0x11, 0x22, 0x43];

        let test_time = super::super::types::Time::new(1565525425, 269558339);

        let infots_big_endian = InfoTs::new(Some(test_time), Endianness::BigEndian);
        // infots.compose(&mut writer_le, Endianness::LittleEndian).unwrap();
        infots_big_endian.compose(&mut writer).unwrap();
        assert_eq!(writer, info_timestamp_message_big_endian);
        assert_eq!(InfoTs::parse(&writer).unwrap(), infots_big_endian);

        writer.clear();

        let infots_little_endian = InfoTs::new(Some(test_time), Endianness::LittleEndian);
        infots_little_endian.compose(&mut writer).unwrap();
        assert_eq!(writer, info_timestamp_message_little_endian);
        assert_eq!(InfoTs::parse(&writer).unwrap(), infots_little_endian);
    }
}
