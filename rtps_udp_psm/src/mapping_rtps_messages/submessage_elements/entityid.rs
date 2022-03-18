use std::io::{Error, Write};

use byteorder::ByteOrder;
use rtps_pim::messages::submessage_elements::EntityIdSubmessageElement;

use crate::mapping_traits::{MappingReadByteOrdered, MappingWriteByteOrdered, NumberOfBytes};

impl MappingWriteByteOrdered for EntityIdSubmessageElement {
    fn mapping_write_byte_ordered<W: Write, B: ByteOrder>(
        &self,
        mut writer: W,
    ) -> Result<(), Error> {
        self.value.mapping_write_byte_ordered::<_, B>(&mut writer)
    }
}
impl<'de> MappingReadByteOrdered<'de> for EntityIdSubmessageElement {
    fn mapping_read_byte_ordered<B: ByteOrder>(buf: &mut &'de [u8]) -> Result<Self, Error> {
        Ok(Self {
            value: MappingReadByteOrdered::mapping_read_byte_ordered::<B>(buf)?,
        })
    }
}
impl NumberOfBytes for EntityIdSubmessageElement {
    fn number_of_bytes(&self) -> usize {
        4
    }
}
