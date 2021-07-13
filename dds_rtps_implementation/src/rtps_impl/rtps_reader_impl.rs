use rust_rtps_pim::{
    behavior::{
        reader::reader::{RTPSReader, RTPSReaderOperations},
        types::Duration,
    },
    structure::{
        types::{Locator, ReliabilityKind, TopicKind, GUID},
        RTPSEndpoint, RTPSEntity, RTPSHistoryCache,
    },
};

use crate::utils::shared_object::RtpsLock;

use super::rtps_history_cache_impl::RTPSHistoryCacheImpl;

pub struct RTPSReaderImpl {
    guid: GUID,
    topic_kind: TopicKind,
    reliability_level: ReliabilityKind,
    unicast_locator_list: Vec<Locator>,
    multicast_locator_list: Vec<Locator>,
    heartbeat_response_delay: Duration,
    heartbeat_supression_duration: Duration,
    expects_inline_qos: bool,
    reader_cache: RTPSHistoryCacheImpl,
}

impl RTPSEntity for RtpsLock<'_, RTPSReaderImpl> {
    fn guid(&self) -> &GUID {
        &self.guid
    }
}

impl RTPSReader for RtpsLock<'_, RTPSReaderImpl> {
    type HistoryCacheType = RTPSHistoryCacheImpl;

    fn heartbeat_response_delay(&self) -> &Duration {
        &self.heartbeat_response_delay
    }

    fn heartbeat_supression_duration(&self) -> &Duration {
        &self.heartbeat_supression_duration
    }

    fn reader_cache(&self) -> &Self::HistoryCacheType {
        &self.reader_cache
    }

    fn reader_cache_mut(&mut self) -> &mut Self::HistoryCacheType {
        &mut self.reader_cache
    }

    fn expects_inline_qos(&self) -> bool {
        self.expects_inline_qos
    }
}

impl RTPSReaderOperations for RTPSReaderImpl {
    fn new(
        guid: GUID,
        topic_kind: TopicKind,
        reliability_level: ReliabilityKind,
        unicast_locator_list: &[Locator],
        multicast_locator_list: &[Locator],
        heartbeat_response_delay: Duration,
        heartbeat_supression_duration: Duration,
        expects_inline_qos: bool,
    ) -> Self {
        Self {
            guid,
            topic_kind,
            reliability_level,
            unicast_locator_list: unicast_locator_list.into_iter().cloned().collect(),
            multicast_locator_list: multicast_locator_list.into_iter().cloned().collect(),
            heartbeat_response_delay,
            heartbeat_supression_duration,
            expects_inline_qos,
            reader_cache: RTPSHistoryCacheImpl::new(),
        }
    }
}

impl RTPSEndpoint for RTPSReaderImpl {
    fn topic_kind(&self) -> &TopicKind {
        &self.topic_kind
    }

    fn reliability_level(&self) -> &ReliabilityKind {
        &self.reliability_level
    }

    fn unicast_locator_list(&self) -> &[Locator] {
        &self.unicast_locator_list
    }

    fn multicast_locator_list(&self) -> &[Locator] {
        &self.multicast_locator_list
    }
}
