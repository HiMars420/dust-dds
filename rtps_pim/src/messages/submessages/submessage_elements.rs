///
/// This files shall only contain the types as listed in the DDSI-RTPS Version 2.3
/// 8.3.5 RTPS SubmessageElements
///
use crate::{messages, types};

pub trait SubmessageElement {}

pub trait GuidPrefix: SubmessageElement {
    type GuidPrefix: types::GuidPrefix;
    fn value(&self) -> &Self::GuidPrefix;
}

pub trait EntityId: SubmessageElement {
    type EntityId: types::EntityId;
    fn value(&self) -> &Self::EntityId;
}

pub trait VendorId: SubmessageElement {
    type VendorId: types::VendorId;
    fn value(&self) -> &Self::VendorId;

    const VENDORID_UNKNOWN: Self;
}

pub trait ProtocolVersion: SubmessageElement {
    type ProtocolVersion: types::ProtocolVersion;
    fn value(&self) -> &Self::ProtocolVersion;

    const PROTOCOLVERSION_1_0: Self;
    const PROTOCOLVERSION_1_1: Self;
    const PROTOCOLVERSION_2_0: Self;
    const PROTOCOLVERSION_2_1: Self;
    const PROTOCOLVERSION_2_2: Self;
    const PROTOCOLVERSION_2_3: Self;
    const PROTOCOLVERSION_2_4: Self;
}

pub trait SequenceNumber: SubmessageElement {
    type SequenceNumber: types::SequenceNumber;
    fn value(&self) -> &Self::SequenceNumber;

    const SEQUENCENUMBER_UNKNOWN: Self;
}

pub trait SequenceNumberSet: SubmessageElement {
    type SequenceNumber: types::SequenceNumber;

    fn base(&self) -> &Self::SequenceNumber;
    fn set(&self) -> &[Self::SequenceNumber];
}

pub trait FragmentNumber: SubmessageElement {
    type FragmentNumber: messages::types::FragmentNumber;
    fn value(&self) -> &Self::FragmentNumber;
}

pub trait FragmentNumberSet: SubmessageElement {
    type FragmentNumber: messages::types::FragmentNumber;

    fn base(&self) -> &Self::FragmentNumber;
    fn set(&self) -> &[Self::FragmentNumber];
}

pub trait Timestamp: SubmessageElement {
    type Time: messages::types::Time;
    fn value(&self) -> &Self::Time;

    const TIME_ZERO: Self;
    const TIME_INVALID: Self;
    const TIME_INFINITE: Self;
}

pub trait Parameter {
    type ParameterId: messages::types::ParameterId;
    fn parameter_id(&self) -> &Self::ParameterId;
    fn length(&self) -> i16;
    fn value(&self) -> &[u8];
}

pub trait ParameterList: SubmessageElement {
    type Parameter: Parameter + ?Sized;
    type Item: core::ops::Deref<Target = Self::Parameter>;
    type ParameterList: IntoIterator<Item = Self::Item>;

    fn parameter(&self) -> &Self::ParameterList;
}

pub trait Count: SubmessageElement {
    type Count: messages::types::Count;
    fn value(&self) -> &Self::Count;
}

pub trait LocatorList: SubmessageElement {
    type Locator: types::Locator;
    fn value(&self) -> &[Self::Locator];
}

pub trait SerializedData: SubmessageElement {
    fn value(&self) -> &[u8];
}

pub trait SerializedDataFragment: SubmessageElement {
    fn value(&self) -> &[u8];
}

pub trait GroupDigest: SubmessageElement {
    type GroupDigest: GroupDigest;
    fn value(&self) -> &Self::GroupDigest;
}
