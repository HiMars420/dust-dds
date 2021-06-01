use crate::messages::{submessage_elements::ParameterList, types::ParameterIdType};

///
/// This files shall only contain the types as listed in the DDSI-RTPS Version 2.3
/// Table 8.2 - Types of the attributes that appear in the RTPS Entities and Classes
///
pub trait GuidPrefixPIM {
    type GuidPrefixType: Into<[u8; 12]> + From<[u8; 12]> + Copy + PartialEq + Send + Sync;
    const GUIDPREFIX_UNKNOWN: Self::GuidPrefixType;
}

pub trait EntityIdPIM {
    type EntityIdType: Into<[u8; 4]> + From<[u8; 4]> + Copy + PartialEq + Send + Sync;
    const ENTITYID_UNKNOWN: Self::EntityIdType;
    const ENTITYID_PARTICIPANT: Self::EntityIdType;
}

pub trait SequenceNumberType {
    type SequenceNumber: Into<i64> + From<i64> + Ord + Copy + Send + Sync;
    const SEQUENCE_NUMBER_UNKNOWN: Self::SequenceNumber;
}

pub trait LocatorType {
    type Locator: LocatorSubTypes;
}

pub trait LocatorSubTypes: PartialEq + Copy + Send + Sync {
    type LocatorKind: Into<[u8; 4]> + From<[u8; 4]> + PartialEq + Copy + Send + Sync;
    const LOCATOR_KIND_INVALID: Self::LocatorKind;
    const LOCATOR_KIND_RESERVED: Self::LocatorKind;
    #[allow(non_upper_case_globals)]
    const LOCATOR_KIND_UDPv4: Self::LocatorKind;
    #[allow(non_upper_case_globals)]
    const LOCATOR_KIND_UDPv6: Self::LocatorKind;

    type LocatorPort: Into<[u8; 4]> + From<[u8; 4]> + PartialEq + Copy + Send + Sync;
    const LOCATOR_PORT_INVALID: Self::LocatorPort;

    type LocatorAddress: Into<[u8; 16]> + From<[u8; 16]> + PartialEq + Copy + Send + Sync;
    const LOCATOR_ADDRESS_INVALID: Self::LocatorAddress;

    const LOCATOR_INVALID: Self;

    fn kind(&self) -> &Self::LocatorKind;
    fn port(&self) -> &Self::LocatorPort;
    fn address(&self) -> &Self::LocatorAddress;
}

pub trait InstanceHandleType {
    type InstanceHandle: Copy + Send + Sync;
}

pub trait ProtocolVersionType {
    type ProtocolVersion: Copy + Send + Sync;
    const PROTOCOLVERSION: Self::ProtocolVersion;
    const PROTOCOLVERSION_1_0: Self::ProtocolVersion;
    const PROTOCOLVERSION_1_1: Self::ProtocolVersion;
    const PROTOCOLVERSION_2_0: Self::ProtocolVersion;
    const PROTOCOLVERSION_2_1: Self::ProtocolVersion;
    const PROTOCOLVERSION_2_2: Self::ProtocolVersion;
    const PROTOCOLVERSION_2_3: Self::ProtocolVersion;
    const PROTOCOLVERSION_2_4: Self::ProtocolVersion;
}

pub trait VendorIdType {
    type VendorId: Copy + Send + Sync;
    const VENDOR_ID_UNKNOWN: Self::VendorId;
}

pub trait DataType {
    type Data: Send + Sync;
}

pub trait ParameterListType<PSM: ParameterIdType> {
    type ParameterList: ParameterList<PSM> + Send + Sync;
}

pub trait GUIDType<PSM: GuidPrefixPIM + EntityIdPIM> {
    type GUID: GUID<PSM> + Copy + PartialEq + Send + Sync;
    const GUID_UNKNOWN: Self::GUID;
}

/// Define the GUID as described in 8.2.4.1 Identifying RTPS entities: The GUID
pub trait GUID<PSM: GuidPrefixPIM + EntityIdPIM> {
    fn new(prefix: PSM::GuidPrefixType, entity_id: PSM::EntityIdType) -> Self;
    fn prefix(&self) -> &PSM::GuidPrefixType;
    fn entity_id(&self) -> &PSM::EntityIdType;
}

// impl<PSM: GuidPrefixPIM + EntityIdPIM> GUID<PSM> {
//     pub const GUID_UNKNOWN: Self = Self {
//         prefix: PSM::GUIDPREFIX_UNKNOWN,
//         entity_id: PSM::ENTITYID_UNKNOWN,
//     };

//     pub fn new(prefix: PSM::GuidPrefix, entity_id: PSM::EntityId) -> Self {
//         Self { prefix, entity_id }
//     }

//     pub fn prefix(&self) -> &PSM::GuidPrefix {
//         &self.prefix
//     }

//     pub fn entity_id(&self) -> &PSM::EntityId {
//         &self.entity_id
//     }
// }

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TopicKind {
    NoKey,
    WithKey,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeKind {
    Alive,
    AliveFiltered,
    NotAliveDisposed,
    NotAliveUnregistered,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReliabilityKind {
    BestEffort,
    Reliable,
}
