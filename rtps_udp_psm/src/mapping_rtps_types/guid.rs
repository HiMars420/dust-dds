use std::io::Write;

use byteorder::ByteOrder;
use rust_rtps_pim::structure::types::{EntityId, EntityKind, Guid};

use crate::{
    deserialize::{self, Deserialize},
    serialize::{self, Serialize},
};

// Table 9.1 - entityKind octet of an EntityId_t
pub const USER_DEFINED_UNKNOWN: u8 = 0x00;
pub const BUILT_IN_UNKNOWN: u8 = 0xc0;
pub const BUILT_IN_PARTICIPANT: u8 = 0xc1;
pub const USER_DEFINED_WRITER_WITH_KEY: u8 = 0x02;
pub const BUILT_IN_WRITER_WITH_KEY: u8 = 0xc2;
pub const USER_DEFINED_WRITER_NO_KEY: u8 = 0x03;
pub const BUILT_IN_WRITER_NO_KEY: u8 = 0xc3;
pub const USER_DEFINED_READER_WITH_KEY: u8 = 0x07;
pub const BUILT_IN_READER_WITH_KEY: u8 = 0xc7;
pub const USER_DEFINED_READER_NO_KEY: u8 = 0x04;
pub const BUILT_IN_READER_NO_KEY: u8 = 0xc4;
pub const USER_DEFINED_WRITER_GROUP: u8 = 0x08;
pub const BUILT_IN_WRITER_GROUP: u8 = 0xc8;
pub const USER_DEFINED_READER_GROUP: u8 = 0x09;
pub const BUILT_IN_READER_GROUP: u8 = 0xc9;

impl Serialize for EntityId {
    fn serialize<W: Write, B: ByteOrder>(&self, mut writer: W) -> serialize::Result {
        self.entity_key().serialize::<_, B>(&mut writer)?;
        match self.entity_kind() {
            EntityKind::UserDefinedUnknown => USER_DEFINED_UNKNOWN,
            EntityKind::BuiltInUnknown => BUILT_IN_UNKNOWN,
            EntityKind::BuiltInParticipant => BUILT_IN_PARTICIPANT,
            EntityKind::UserDefinedWriterWithKey => USER_DEFINED_WRITER_WITH_KEY,
            EntityKind::BuiltInWriterWithKey => BUILT_IN_WRITER_WITH_KEY,
            EntityKind::UserDefinedWriterNoKey => USER_DEFINED_WRITER_NO_KEY,
            EntityKind::BuiltInWriterNoKey => BUILT_IN_WRITER_NO_KEY,
            EntityKind::UserDefinedReaderWithKey => USER_DEFINED_READER_WITH_KEY,
            EntityKind::BuiltInReaderWithKey => BUILT_IN_READER_WITH_KEY,
            EntityKind::UserDefinedReaderNoKey => USER_DEFINED_READER_NO_KEY,
            EntityKind::BuiltInReaderNoKey => BUILT_IN_READER_NO_KEY,
            EntityKind::UserDefinedWriterGroup => USER_DEFINED_WRITER_GROUP,
            EntityKind::BuiltInWriterGroup => BUILT_IN_WRITER_GROUP,
            EntityKind::UserDefinedReaderGroup => USER_DEFINED_READER_GROUP,
            EntityKind::BuiltInReaderGroup => BUILT_IN_READER_GROUP,
        }
        .serialize::<_, B>(&mut writer)
    }
}

impl<'de> Deserialize<'de> for EntityId {
    fn deserialize<B: ByteOrder>(buf: &mut &'de [u8]) -> deserialize::Result<Self> {
        let entity_key = Deserialize::deserialize::<B>(buf)?;
        let entity_kind = match Deserialize::deserialize::<B>(buf)? {
            USER_DEFINED_UNKNOWN => EntityKind::UserDefinedUnknown,
            BUILT_IN_UNKNOWN => EntityKind::BuiltInUnknown,
            BUILT_IN_PARTICIPANT => EntityKind::BuiltInParticipant,
            USER_DEFINED_WRITER_WITH_KEY => EntityKind::UserDefinedWriterWithKey,
            BUILT_IN_WRITER_WITH_KEY => EntityKind::BuiltInWriterWithKey,
            USER_DEFINED_WRITER_NO_KEY => EntityKind::UserDefinedWriterNoKey,
            BUILT_IN_WRITER_NO_KEY => EntityKind::BuiltInWriterNoKey,
            USER_DEFINED_READER_WITH_KEY => EntityKind::UserDefinedReaderWithKey,
            BUILT_IN_READER_WITH_KEY => EntityKind::BuiltInReaderWithKey,
            USER_DEFINED_READER_NO_KEY => EntityKind::UserDefinedReaderNoKey,
            BUILT_IN_READER_NO_KEY => EntityKind::BuiltInReaderNoKey,
            USER_DEFINED_WRITER_GROUP => EntityKind::UserDefinedWriterGroup,
            BUILT_IN_WRITER_GROUP => EntityKind::BuiltInWriterGroup,
            USER_DEFINED_READER_GROUP => EntityKind::UserDefinedReaderGroup,
            BUILT_IN_READER_GROUP => EntityKind::BuiltInReaderGroup,
            _ => {
                return deserialize::Result::Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "EntityKind unknown",
                ))
            }
        };
        Ok(Self {
            entity_key,
            entity_kind,
        })
    }
}

impl Serialize for Guid {
    fn serialize<W: Write, B: ByteOrder>(&self, mut writer: W) -> serialize::Result {
        self.prefix.serialize::<_, B>(&mut writer)?;
        self.entity_id.serialize::<_, B>(&mut writer)
    }
}

impl<'de> Deserialize<'de> for Guid {
    fn deserialize<B: ByteOrder>(buf: &mut &'de [u8]) -> deserialize::Result<Self> {
        Ok(Self {
            prefix: Deserialize::deserialize::<B>(buf)?,
            entity_id: Deserialize::deserialize::<B>(buf)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_rtps_pim::structure::types::{EntityId, EntityKind};

    use crate::deserialize::from_bytes_le;
    use crate::serialize::to_bytes_le;

    #[test]
    fn serialize_entity_id() {
        let entity_id = EntityId::new([1, 2, 3], EntityKind::BuiltInParticipant);
        assert_eq!(to_bytes_le(&entity_id).unwrap(), [1, 2, 3, 0xc1]);
    }

    #[test]
    fn deserialize_locator() {
        let expected = EntityId::new([1, 2, 3], EntityKind::BuiltInParticipant);
        let result = from_bytes_le(&[1, 2, 3, 0xc1]).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_guid() {
        let guid = Guid {
            prefix: [3; 12],
            entity_id: EntityId {
                entity_key: [1, 2, 3],
                entity_kind: EntityKind::UserDefinedReaderNoKey,
            },
        };

        assert_eq!(
            to_bytes_le(&guid).unwrap(),
            [
                3, 3, 3, 3, // GuidPrefix
                3, 3, 3, 3, // GuidPrefix
                3, 3, 3, 3, // GuidPrefix
                1, 2, 3, 0x04 // EntityId
            ]
        );
    }

    #[test]
    fn deserialize_guid() {
        let expected = Guid {
            prefix: [3; 12],
            entity_id: EntityId {
                entity_key: [1, 2, 3],
                entity_kind: EntityKind::UserDefinedReaderNoKey,
            },
        };

        assert_eq!(
            expected,
            from_bytes_le(&[
                3, 3, 3, 3, // GuidPrefix
                3, 3, 3, 3, // GuidPrefix
                3, 3, 3, 3, // GuidPrefix
                1, 2, 3, 0x04 // EntityId
            ])
            .unwrap()
        );
    }
}
