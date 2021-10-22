use rust_rtps_psm::messages::submessages::{
    InfoTimestampSubmessageRead, InfoTimestampSubmessageWrite,
};

use crate::{
    deserialize::{self, MappingRead},
    serialize::{self, MappingWrite},
};

use std::io::Write;

impl MappingWrite for InfoTimestampSubmessageWrite {
    fn write<W: Write>(&self, mut _writer: W) -> serialize::Result {
        todo!()
    }
}
impl<'de> MappingRead<'de> for InfoTimestampSubmessageRead {
    fn read(_buf: &mut &'de [u8]) -> deserialize::Result<Self> {
        todo!()
    }
}
