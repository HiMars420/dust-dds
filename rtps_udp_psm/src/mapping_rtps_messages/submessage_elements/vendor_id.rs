use std::io::Write;

use byteorder::ByteOrder;
use rust_rtps_pim::messages::submessage_elements::VendorIdSubmessageElement;

use crate::{
    deserialize::{self, Deserialize},
    serialize::{self, Serialize},
};

impl Serialize for VendorIdSubmessageElement {
    fn serialize<W: Write, B: ByteOrder>(&self, mut writer: W) -> serialize::Result {
        self.value.serialize::<_, B>(&mut writer)
    }
}

impl<'de> Deserialize<'de> for VendorIdSubmessageElement {
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
    fn serialize_vendor_id() {
        let data = VendorIdSubmessageElement { value: [1, 2] };
        assert_eq!(to_bytes_le(&data).unwrap(), vec![1, 2,]);
    }

    #[test]
    fn deserialize_vendor_id() {
        let expected = VendorIdSubmessageElement { value: [1, 2] };
        assert_eq!(expected, from_bytes_le(&[1, 2,]).unwrap());
    }
}
