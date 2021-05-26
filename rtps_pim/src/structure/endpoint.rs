use super::{
    types::{EntityIdType, GuidPrefixType, LocatorType, ReliabilityKind, TopicKind},
    RTPSEntity,
};

pub trait RTPSEndpoint<PSM: GuidPrefixType + EntityIdType + LocatorType>: RTPSEntity<PSM> {
    fn topic_kind(&self) -> TopicKind;
    fn reliability_level(&self) -> ReliabilityKind;
    fn unicast_locator_list(&self) -> &[PSM::Locator];
    fn multicast_locator_list(&self) -> &[PSM::Locator];
}
