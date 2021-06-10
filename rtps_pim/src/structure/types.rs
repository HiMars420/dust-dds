use crate::messages::{submessage_elements, types::ParameterIdPIM};

///
/// This files shall only contain the types as listed in the DDSI-RTPS Version 2.3
/// Table 8.2 - Types of the attributes that appear in the RTPS Entities and Classes
///
pub trait GuidPrefixPIM {
    type GuidPrefixType: Into<[u8; 12]> + From<[u8; 12]>;
    const GUIDPREFIX_UNKNOWN: Self::GuidPrefixType;
}

pub trait EntityIdPIM {
    type EntityIdType: Into<[u8; 4]> + From<[u8; 4]>;
    const ENTITYID_UNKNOWN: Self::EntityIdType;
    const ENTITYID_PARTICIPANT: Self::EntityIdType;
}

pub trait SequenceNumberPIM {
    type SequenceNumberType: Into<i64> + From<i64>;
    const SEQUENCE_NUMBER_UNKNOWN: Self::SequenceNumberType;
}

pub trait LocatorPIM {
    type LocatorType;

    const LOCATOR_INVALID: Self::LocatorType;
}

pub trait Locator {
    type LocatorKind: Into<[u8; 4]> + From<[u8; 4]>;

    const LOCATOR_KIND_INVALID: Self::LocatorKind;
    const LOCATOR_KIND_RESERVED: Self::LocatorKind;
    #[allow(non_upper_case_globals)]
    const LOCATOR_KIND_UDPv4: Self::LocatorKind;
    #[allow(non_upper_case_globals)]
    const LOCATOR_KIND_UDPv6: Self::LocatorKind;

    type LocatorPort: Into<[u8; 4]> + From<[u8; 4]>;
    const LOCATOR_PORT_INVALID: Self::LocatorPort;

    type LocatorAddress: Into<[u8; 16]> + From<[u8; 16]>;
    const LOCATOR_ADDRESS_INVALID: Self::LocatorAddress;

    fn kind(&self) -> &Self::LocatorKind;
    fn port(&self) -> &Self::LocatorPort;
    fn address(&self) -> &Self::LocatorAddress;
}

pub trait InstanceHandlePIM {
    type InstanceHandleType;
}

pub trait ProtocolVersionPIM {
    type ProtocolVersionType;
    const PROTOCOLVERSION: Self::ProtocolVersionType;
    const PROTOCOLVERSION_1_0: Self::ProtocolVersionType;
    const PROTOCOLVERSION_1_1: Self::ProtocolVersionType;
    const PROTOCOLVERSION_2_0: Self::ProtocolVersionType;
    const PROTOCOLVERSION_2_1: Self::ProtocolVersionType;
    const PROTOCOLVERSION_2_2: Self::ProtocolVersionType;
    const PROTOCOLVERSION_2_3: Self::ProtocolVersionType;
    const PROTOCOLVERSION_2_4: Self::ProtocolVersionType;
}

pub trait VendorIdPIM {
    type VendorIdType;
    const VENDOR_ID_UNKNOWN: Self::VendorIdType;
}

pub trait DataPIM {
    type DataType;
}

pub trait ParameterListPIM<PSM: ParameterIdPIM> {
    type ParameterListType: submessage_elements::ParameterList<PSM>;
}

pub trait GUIDPIM<PSM: GuidPrefixPIM + EntityIdPIM> {
    type GUIDType: GUIDType<PSM>;
    const GUID_UNKNOWN: Self::GUIDType;
}

/// Define the GUID as described in 8.2.4.1 Identifying RTPS entities: The GUID
pub trait GUIDType<PSM: GuidPrefixPIM + EntityIdPIM> {
    fn new(prefix: PSM::GuidPrefixType, entity_id: PSM::EntityIdType) -> Self;
    fn prefix(&self) -> PSM::GuidPrefixType;
    fn entity_id(&self) -> PSM::EntityIdType;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TopicKind {
    NoKey,
    WithKey,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Alive,
    AliveFiltered,
    NotAliveDisposed,
    NotAliveUnregistered,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReliabilityKind {
    BestEffort,
    Reliable,
}
