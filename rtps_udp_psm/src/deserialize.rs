use byteorder::{BigEndian, ByteOrder, LittleEndian};
use rust_rtps_pim::messages::overall_structure::RtpsSubmessageHeader;

pub type Result<T> = std::result::Result<T, std::io::Error>;

pub trait MappingRead<'de>: Sized {
    fn mapping_read(buf: &mut &'de [u8]) -> Result<Self>;
}

pub trait MappingReadSubmessage<'de>: Sized {
    fn mapping_read_submessage<B: ByteOrder>(
        buf: &mut &'de [u8],
        header: RtpsSubmessageHeader,
    ) -> Result<Self>;
}

impl<'a, 'de: 'a, T> MappingRead<'de> for T
where
    T: MappingReadSubmessage<'de>,
{
    fn mapping_read(buf: &mut &'de [u8]) -> Result<Self> {
        let header: RtpsSubmessageHeader = MappingRead::mapping_read(buf)?;
        if header.flags[0] {
            Self::mapping_read_submessage::<LittleEndian>(buf, header)
        } else {
            Self::mapping_read_submessage::<BigEndian>(buf, header)
        }
    }
}

pub trait MappingReadByteOrdered<'de>: Sized {
    fn mapping_read_byte_ordered<B>(buf: &mut &'de [u8]) -> Result<Self>
    where
        B: ByteOrder;
}

pub fn from_bytes_le<'de, D: MappingReadByteOrdered<'de>>(mut buf: &'de [u8]) -> Result<D> {
    D::mapping_read_byte_ordered::<LittleEndian>(&mut buf)
}

pub fn from_bytes<'de, D: MappingRead<'de>>(mut buf: &'de [u8]) -> Result<D> {
    D::mapping_read(&mut buf)
}
