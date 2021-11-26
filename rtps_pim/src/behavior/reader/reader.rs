use crate::{
    behavior::types::Duration,
    structure::{
        endpoint::RtpsEndpoint,
        history_cache::RtpsHistoryCacheConstructor,
        types::{Guid, ReliabilityKind, TopicKind},
    },
};

pub struct RtpsReader<L, C> {
    pub endpoint: RtpsEndpoint<L>,
    pub heartbeat_response_delay: Duration,
    pub heartbeat_supression_duration: Duration,
    pub reader_cache: C,
    pub expects_inline_qos: bool,
}

impl<L, C> RtpsReader<L, C> {
    pub fn new(
        guid: Guid,
        topic_kind: TopicKind,
        reliability_level: ReliabilityKind,
        unicast_locator_list: L,
        multicast_locator_list: L,
        heartbeat_response_delay: Duration,
        heartbeat_supression_duration: Duration,
        expects_inline_qos: bool,
    ) -> Self
    where
        C: RtpsHistoryCacheConstructor,
    {
        Self {
            endpoint: RtpsEndpoint::new(
                guid,
                topic_kind,
                reliability_level,
                unicast_locator_list,
                multicast_locator_list,
            ),
            heartbeat_response_delay,
            heartbeat_supression_duration,
            reader_cache: C::new(),
            expects_inline_qos,
        }
    }
}
