use crate::types::TimeT;

use super::helpers::{deserialize, endianess};

use super::{Result, ErrorMessage};

#[derive(PartialEq, Debug)]
pub struct InfoTs {
    timestamp: Option<TimeT>, 
}

impl InfoTs {
    pub fn timestamp(&self) -> &Option<TimeT> {
        &self.timestamp
    } 
}

pub fn parse_info_timestamp_submessage(submessage: &[u8], submessage_flags: &u8) -> Result<InfoTs> {
    const MESSAGE_PAYLOAD_FIRST_INDEX: usize = 0;
    const MESSAGE_PAYLOAD_LAST_INDEX: usize = 7;

    if MESSAGE_PAYLOAD_LAST_INDEX >= submessage.len() {
        return Err(ErrorMessage::InvalidSubmessage);
    }

    let submessage_endianess = endianess(submessage_flags)?;

    let timestamp = if *submessage_flags & 0x02 == 0x02 {
        None
    }
    else {
        Some(deserialize::<TimeT>(submessage, &MESSAGE_PAYLOAD_FIRST_INDEX, &MESSAGE_PAYLOAD_LAST_INDEX, &submessage_endianess)?)
    };

    Ok(InfoTs{timestamp: timestamp})
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_parse_info_timestamp_submessage() {
        const BIG_ENDIAN_FLAG: u8 = 0x00;
        const LITTLE_ENDIAN_FLAG: u8 = 0x01;
        const INVALID_FLAG : u8 = 0x02;

        // Unix time: 1565525425=>0x5D5005B1
        // Is equivalent to: 08/11/2019 @ 12:10pm (UTC)
        // Seconds fraction: 0x10112243 => 269558339 => 0.0628
        const TEST_TIME : TimeT = TimeT {
            seconds: 1565525425,
            fraction: 269558339,
        };
        
        let timestamp_message_payload_big_endian = [0x5D,0x50,0x05,0xB1,0x10,0x11,0x22,0x43];
        let info_ts_big_endian = parse_info_timestamp_submessage(&timestamp_message_payload_big_endian, &BIG_ENDIAN_FLAG).unwrap();
        assert_eq!(Some(TEST_TIME),info_ts_big_endian.timestamp);

        let timestamp_message_payload_little_endian = [0xB1,0x05,0x50,0x5D,0x43,0x22,0x11,0x10];
        let info_ts_little_endian = parse_info_timestamp_submessage(&timestamp_message_payload_little_endian, &LITTLE_ENDIAN_FLAG).unwrap();
        assert_eq!(Some(TEST_TIME),info_ts_little_endian.timestamp);

        let info_ts_none_big_endian = parse_info_timestamp_submessage(&timestamp_message_payload_big_endian, &(BIG_ENDIAN_FLAG+INVALID_FLAG)).unwrap();
        assert_eq!(None,info_ts_none_big_endian.timestamp);

        let info_ts_none_little_endian = parse_info_timestamp_submessage(&timestamp_message_payload_little_endian, &(LITTLE_ENDIAN_FLAG+INVALID_FLAG)).unwrap();
        assert_eq!(None,info_ts_none_little_endian.timestamp);
    }

}