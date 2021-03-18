use std::ops::{Add, AddAssign, Sub};

///
/// This files shall only contain the types as listed in the DDSI-RTPS Version 2.3
/// Table 8.2 - Types of the attributes that appear in the RTPS Entities and Classes
///

pub mod constants {
    use super::{EntityId, GuidPrefix, Locator, ProtocolVersion, SequenceNumber, VendorId, GUID};

    pub const VENDOR_ID: VendorId = [99, 99];

    pub const PROTOCOL_VERSION_2_1: ProtocolVersion = ProtocolVersion { major: 2, minor: 1 };
    pub const PROTOCOL_VERSION_2_2: ProtocolVersion = ProtocolVersion { major: 2, minor: 2 };
    pub const PROTOCOL_VERSION_2_4: ProtocolVersion = ProtocolVersion { major: 2, minor: 4 };

    pub const LOCATOR_KIND_INVALID: i32 = -1;
    pub const LOCATOR_KIND_RESERVED: i32 = 0;
    #[allow(non_upper_case_globals)]
    pub const LOCATOR_KIND_UDPv4: i32 = 1;
    #[allow(non_upper_case_globals)]
    pub const LOCATOR_KIND_UDPv6: i32 = 2;
    pub const LOCATOR_PORT_INVALID: u32 = 0;
    pub const LOCATOR_ADDRESS_INVALID: [u8; 16] = [0; 16];
    pub const LOCATOR_INVALID: Locator = Locator {
        kind: LOCATOR_KIND_INVALID,
        port: LOCATOR_PORT_INVALID,
        address: LOCATOR_ADDRESS_INVALID,
    };

    pub const GUID_PREFIX_UNKNOWN: GuidPrefix = [0; 12];

    pub const ENTITY_KIND_USER_DEFINED_UNKNOWN: u8 = 0x00;
    pub const ENTITY_KIND_USER_DEFINED_WRITER_WITH_KEY: u8 = 0x02;
    pub const ENTITY_KIND_USER_DEFINED_WRITER_NO_KEY: u8 = 0x03;
    pub const ENTITY_KIND_USER_DEFINED_READER_WITH_KEY: u8 = 0x04;
    pub const ENTITY_KIND_USER_DEFINED_READER_NO_KEY: u8 = 0x07;
    pub const ENTITY_KIND_USER_DEFINED_WRITER_GROUP: u8 = 0x08;
    pub const ENTITY_KIND_USER_DEFINED_READER_GROUP: u8 = 0x09;
    pub const ENTITY_KIND_BUILT_IN_UNKNOWN: u8 = 0xc0;
    pub const ENTITY_KIND_BUILT_IN_PARTICIPANT: u8 = 0xc1;
    pub const ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY: u8 = 0xc2;
    pub const ENTITY_KIND_BUILT_IN_WRITER_NO_KEY: u8 = 0xc3;
    pub const ENTITY_KIND_BUILT_IN_READER_WITH_KEY: u8 = 0xc4;
    pub const ENTITY_KIND_BUILT_IN_READER_NO_KEY: u8 = 0xc7;
    pub const ENTITY_KIND_BUILT_IN_WRITER_GROUP: u8 = 0xc8;
    pub const ENTITY_KIND_BUILT_IN_READER_GROUP: u8 = 0xc9;

    pub const ENTITYID_UNKNOWN: EntityId = EntityId {
        entity_key: [0, 0, 0x00],
        entity_kind: ENTITY_KIND_USER_DEFINED_UNKNOWN,
    };

    pub const ENTITYID_PARTICIPANT: EntityId = EntityId {
        entity_key: [0, 0, 0x01],
        entity_kind: ENTITY_KIND_BUILT_IN_PARTICIPANT,
    };

    pub const ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER: EntityId = EntityId {
        entity_key: [0, 0, 0x02],
        entity_kind: ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY,
    };
    pub const ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR: EntityId = EntityId {
        entity_key: [0, 0, 0x02],
        entity_kind: ENTITY_KIND_BUILT_IN_READER_WITH_KEY,
    };

    pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER: EntityId = EntityId {
        entity_key: [0, 0, 0x03],
        entity_kind: ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY,
    };
    pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR: EntityId = EntityId {
        entity_key: [0, 0, 0x03],
        entity_kind: ENTITY_KIND_BUILT_IN_READER_WITH_KEY,
    };

    pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER: EntityId = EntityId {
        entity_key: [0, 0, 0x04],
        entity_kind: ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY,
    };
    pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR: EntityId = EntityId {
        entity_key: [0, 0, 0x04],
        entity_kind: ENTITY_KIND_BUILT_IN_READER_WITH_KEY,
    };

    pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER: EntityId = EntityId {
        entity_key: [0, 0x01, 0x00],
        entity_kind: ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY,
    };

    pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_DETECTOR: EntityId = EntityId {
        entity_key: [0, 0x01, 0x00],
        entity_kind: ENTITY_KIND_BUILT_IN_READER_WITH_KEY,
    };

    pub const ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER: EntityId = EntityId {
        entity_key: [0, 0x02, 0x00],
        entity_kind: ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY,
    };
    pub const ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER: EntityId = EntityId {
        entity_key: [0, 0x02, 0x00],
        entity_kind: ENTITY_KIND_BUILT_IN_READER_WITH_KEY,
    };

    pub const GUID_UNKNOWN: GUID = GUID {
        prefix: GUID_PREFIX_UNKNOWN,
        entity_id: ENTITYID_UNKNOWN,
    };

    pub const SEQUENCE_NUMBER_UNKNOWN: SequenceNumber = SequenceNumber {
        high: core::i32::MIN,
        low: core::u32::MAX,
    };
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct GUID {
    prefix: GuidPrefix,
    entity_id: EntityId,
}

impl GUID {
    pub fn new(prefix: GuidPrefix, entity_id: EntityId) -> GUID {
        GUID { prefix, entity_id }
    }

    pub fn prefix(&self) -> GuidPrefix {
        self.prefix
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
}

pub type GuidPrefix = [u8; 12];

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct EntityId {
    entity_key: [u8; 3],
    entity_kind: u8,
}

impl EntityId {
    pub fn new(entity_key: [u8; 3], entity_kind: u8) -> Self {
        Self {
            entity_key,
            entity_kind,
        }
    }

    pub fn entity_key(&self) -> [u8; 3] {
        self.entity_key
    }

    pub fn entity_kind(&self) -> u8 {
        self.entity_kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceNumber {
    high: i32,
    low: u32,
}

impl SequenceNumber {
    pub fn new(high: i32, low: u32) -> Self {
        Self { high, low }
    }

    pub fn high(&self) -> i32 {
        self.high
    }

    pub fn low(&self) -> u32 {
        self.low
    }
}

impl From<SequenceNumber> for i64 {
    fn from(_: SequenceNumber) -> Self {
        todo!()
    }
}

impl From<i64> for SequenceNumber {
    fn from(value: i64) -> Self {
        Self {
            high: (value >> 32) as i32,
            low: (value & 2 ^ 32) as u32,
        }
    }
}

impl PartialOrd for SequenceNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        todo!()
    }
}

impl Ord for SequenceNumber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        todo!()
    }
}

impl PartialEq<i64> for SequenceNumber {
    fn eq(&self, other: &i64) -> bool {
        todo!()
    }
}

impl PartialOrd<i64> for SequenceNumber {
    fn partial_cmp(&self, other: &i64) -> Option<std::cmp::Ordering> {
        todo!()
    }
}

impl Add<i64> for SequenceNumber {
    type Output = SequenceNumber;

    fn add(self, rhs: i64) -> Self::Output {
        todo!()
    }
}

impl AddAssign<i64> for SequenceNumber {
    fn add_assign(&mut self, rhs: i64) {
        todo!()
    }
}

impl Sub<i64> for SequenceNumber {
    type Output = SequenceNumber;

    fn sub(self, rhs: i64) -> Self::Output {
        todo!()
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct Locator {
    kind: i32,
    port: u32,
    address: [u8; 16],
}

impl Locator {
    pub fn new(kind: i32, port: u32, address: [u8; 16]) -> Self {
        Self {
            kind,
            port,
            address,
        }
    }

    pub const fn new_udpv4(port: u16, address: [u8; 4]) -> Locator {
        let address: [u8; 16] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, address[0], address[1], address[2], address[3],
        ];
        Locator {
            kind: constants::LOCATOR_KIND_UDPv4,
            port: port as u32,
            address,
        }
    }
    pub fn kind(&self) -> i32 {
        self.kind
    }
    pub fn port(&self) -> u32 {
        self.port
    }
    pub fn address(&self) -> &[u8; 16] {
        &self.address
    }
}

#[derive(Copy, Clone)]
pub enum TopicKind {
    NoKey,
    WithKey,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ChangeKind {
    Alive,
    AliveFiltered,
    NotAliveDisposed,
    NotAliveUnregistered,
}

#[derive(PartialEq, Copy, Clone)]
pub enum ReliabilityKind {
    BestEffort,
    Reliable,
}

#[derive(PartialEq, Debug, Clone, Copy, Eq)]
pub struct ProtocolVersion {
    pub major: u8,
    pub minor: u8,
}

pub type VendorId = [u8; 2];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_number_from_i64() {
        let sn: SequenceNumber = 1234.into();
        assert_eq!(sn, SequenceNumber { high: 0, low: 1234 });
    }
}
