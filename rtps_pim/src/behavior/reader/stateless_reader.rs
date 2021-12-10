use crate::{
    behavior::types::Duration,
    structure::{
        history_cache::RtpsHistoryCacheConstructor,
        types::{Guid, ReliabilityKind, TopicKind},
    },
};

use super::reader::RtpsReader;

pub struct RtpsStatelessReader<L, C> {
    pub reader: RtpsReader<L, C>,
}

impl<L, C> RtpsStatelessReader<L, C>
where
    C: RtpsHistoryCacheConstructor,
{
    pub fn new(
        guid: Guid,
        topic_kind: TopicKind,
        reliability_level: ReliabilityKind,
        unicast_locator_list: L,
        multicast_locator_list: L,
        heartbeat_response_delay: Duration,
        heartbeat_supression_duration: Duration,
        expects_inline_qos: bool,
    ) -> Self {
        Self {
            reader: RtpsReader::new(
                guid,
                topic_kind,
                reliability_level,
                unicast_locator_list,
                multicast_locator_list,
                heartbeat_response_delay,
                heartbeat_supression_duration,
                expects_inline_qos,
            ),
        }
    }
}
