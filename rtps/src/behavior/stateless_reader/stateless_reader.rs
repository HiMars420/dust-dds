use std::ops::{Deref, DerefMut};

use crate::{behavior::{types::Duration, Reader}, types::{GUID, Locator, ReliabilityKind, TopicKind}};

pub struct StatelessReader {
    pub reader: Reader,
}

impl Deref for StatelessReader {
    type Target = Reader;
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}
impl DerefMut for StatelessReader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}

impl StatelessReader {
    pub fn new(
        guid: GUID,
        unicast_locator_list: Vec<Locator>,
        multicast_locator_list: Vec<Locator>,
        topic_kind: TopicKind,
        reliability_level: ReliabilityKind,
        expects_inline_qos: bool,
        heartbeat_response_delay: Duration,
        heartbeat_supression_duration: Duration,
    ) -> Self {
        assert!(
            reliability_level == ReliabilityKind::BestEffort,
            "Only BestEffort supported on stateless reader"
        );

        let reader = Reader::new(
            guid,
            unicast_locator_list,
            multicast_locator_list,
            topic_kind,
            reliability_level,
            expects_inline_qos,
            heartbeat_response_delay,
            heartbeat_supression_duration,
        );
        Self { reader }
    }
}
