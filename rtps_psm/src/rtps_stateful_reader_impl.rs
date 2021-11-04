use std::ops::{Deref, DerefMut};

use rust_rtps_pim::{
    behavior::{
        reader::{
            stateful_reader::{RtpsStatefulReader, RtpsStatefulReaderOperations},
            writer_proxy::RtpsWriterProxy,
        },
        types::Duration,
    },
    structure::{
        history_cache::RtpsHistoryCacheConstructor,
        types::{Guid, Locator, ReliabilityKind, TopicKind},
    },
};

use crate::rtps_writer_proxy_impl::RtpsWriterProxyImpl;

pub struct RtpsStatefulReaderImpl<C>(RtpsStatefulReader<Vec<Locator>, C, Vec<RtpsWriterProxyImpl>>);

impl<C> RtpsStatefulReaderImpl<C> {
    pub fn new(
        guid: Guid,
        topic_kind: TopicKind,
        reliability_level: ReliabilityKind,
        unicast_locator_list: Vec<Locator>,
        multicast_locator_list: Vec<Locator>,
        heartbeat_response_delay: Duration,
        heartbeat_supression_duration: Duration,
        expects_inline_qos: bool,
    ) -> Self
    where
        C: RtpsHistoryCacheConstructor,
    {
        Self(RtpsStatefulReader::new(
            guid,
            topic_kind,
            reliability_level,
            unicast_locator_list,
            multicast_locator_list,
            heartbeat_response_delay,
            heartbeat_supression_duration,
            expects_inline_qos,
        ))
    }
}

impl<C> Deref for RtpsStatefulReaderImpl<C> {
    type Target = RtpsStatefulReader<Vec<Locator>, C, Vec<RtpsWriterProxyImpl>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C> DerefMut for RtpsStatefulReaderImpl<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<C> RtpsStatefulReaderOperations<Vec<Locator>> for RtpsStatefulReaderImpl<C> {
    fn matched_writer_add(&mut self, a_writer_proxy: RtpsWriterProxy<Vec<Locator>>) {
        let writer_proxy = RtpsWriterProxyImpl::new(a_writer_proxy);
        self.matched_writers.push(writer_proxy)
    }

    fn matched_writer_remove(&mut self, writer_proxy_guid: &Guid) {
        self.matched_writers
            .retain(|x| &x.remote_writer_guid != writer_proxy_guid);
    }

    fn matched_writer_lookup(
        &self,
        a_writer_guid: &Guid,
    ) -> Option<&RtpsWriterProxy<Vec<Locator>>> {
        self.matched_writers
            .iter()
            .find(|&x| &x.remote_writer_guid == a_writer_guid)
            .map(|x| x.deref())
    }
}
