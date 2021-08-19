use std::io::Write;

use byteorder::ByteOrder;
use rust_rtps_pim::{
    messages::submessage_elements::SequenceNumberSubmessageElement,
    structure::types::SequenceNumber,
};

use crate::{
    deserialize::{self, Deserialize},
    serialize::{self, Serialize},
};

impl Serialize for SequenceNumber {
    fn serialize<W: Write, B: ByteOrder>(&self, mut writer: W) -> serialize::Result {
        let high = (self >> 32) as i32;
        let low = *self as i32;
        high.serialize::<_, B>(&mut writer)?;
        low.serialize::<_, B>(&mut writer)
    }
}

impl<'de> Deserialize<'de> for SequenceNumber {
    fn deserialize<B: ByteOrder>(buf: &mut &'de [u8]) -> deserialize::Result<Self> {
        let high: i32 = Deserialize::deserialize::<B>(buf)?;
        let low: i32 = Deserialize::deserialize::<B>(buf)?;
        Ok(((high as i64) << 32) + low as i64)
    }
}

impl Serialize for SequenceNumberSubmessageElement {
    fn serialize<W: Write, B: ByteOrder>(&self, mut writer: W) -> serialize::Result {
        self.value.serialize::<_, B>(&mut writer)
    }
}

impl<'de> Deserialize<'de> for SequenceNumberSubmessageElement {
    fn deserialize<B: ByteOrder>(buf: &mut &'de [u8]) -> deserialize::Result<Self> {
        Ok(Self {
            value: Deserialize::deserialize::<B>(buf)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deserialize::from_bytes_le;
    use crate::serialize::to_bytes_le;

    #[test]
    fn serialize_sequence_number() {
        let data = SequenceNumberSubmessageElement { value: 7 };
        assert_eq!(
            to_bytes_le(&data).unwrap(),
            vec![
                0, 0, 0, 0, // high (long)
                7, 0, 0, 0, // low (unsigned long)
            ]
        );
    }

    #[test]
    fn deserialize_sequence_number() {
        let expected = SequenceNumberSubmessageElement { value: 7 };
        assert_eq!(
            expected,
            from_bytes_le(&[
                0, 0, 0, 0, // high (long)
                7, 0, 0, 0, // low (unsigned long)
            ])
            .unwrap()
        );
    }
}
