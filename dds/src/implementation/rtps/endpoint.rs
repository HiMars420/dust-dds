use dds_transport::types::Locator;

use super::{
    entity::RtpsEntity,
    types::{Guid, TopicKind},
};

pub struct RtpsEndpoint {
    entity: RtpsEntity,
    topic_kind: TopicKind,
    unicast_locator_list: Vec<Locator>,
    multicast_locator_list: Vec<Locator>,
}

impl RtpsEndpoint {
    pub fn new(
        guid: Guid,
        topic_kind: TopicKind,
        unicast_locator_list: &[Locator],
        multicast_locator_list: &[Locator],
    ) -> Self {
        Self {
            entity: RtpsEntity::new(guid),
            topic_kind,
            unicast_locator_list: unicast_locator_list.to_vec(),
            multicast_locator_list: multicast_locator_list.to_vec(),
        }
    }
}

impl RtpsEndpoint {
    pub fn guid(&self) -> Guid {
        self.entity.guid()
    }
}

impl RtpsEndpoint {
    pub fn topic_kind(&self) -> TopicKind {
        self.topic_kind
    }

    pub fn unicast_locator_list(&self) -> &[Locator] {
        &self.unicast_locator_list
    }

    pub fn multicast_locator_list(&self) -> &[Locator] {
        &self.multicast_locator_list
    }
}
