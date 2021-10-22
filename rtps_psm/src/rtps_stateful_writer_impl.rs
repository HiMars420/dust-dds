use std::ops::{Deref, DerefMut};

use rust_rtps_pim::{
    behavior::{
        types::Duration,
        writer::{
            reader_proxy::RtpsReaderProxy,
            stateful_writer::{RtpsStatefulWriter, RtpsStatefulWriterOperations},
        },
    },
    structure::{
        types::{Guid, Locator, ReliabilityKind, TopicKind},
        RtpsHistoryCacheOperations,
    },
};

pub struct RtpsStatefulWriterImpl<C>(RtpsStatefulWriter<Vec<Locator>, C, ()>);

impl<C> RtpsStatefulWriterImpl<C> {
    pub fn new(
        guid: Guid,
        topic_kind: TopicKind,
        reliability_level: ReliabilityKind,
        unicast_locator_list: Vec<Locator>,
        multicast_locator_list: Vec<Locator>,
        push_mode: bool,
        heartbeat_period: Duration,
        nack_response_delay: Duration,
        nack_suppression_duration: Duration,
        data_max_size_serialized: Option<i32>,
    ) -> Self
    where
        C: for<'a> RtpsHistoryCacheOperations<'a>,
    {
        Self(RtpsStatefulWriter::new(
            guid,
            topic_kind,
            reliability_level,
            unicast_locator_list,
            multicast_locator_list,
            push_mode,
            heartbeat_period,
            nack_response_delay,
            nack_suppression_duration,
            data_max_size_serialized,
        ))
    }
}

impl<C> Deref for RtpsStatefulWriterImpl<C> {
    type Target = RtpsStatefulWriter<Vec<Locator>, C, ()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C> DerefMut for RtpsStatefulWriterImpl<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<C> RtpsStatefulWriterOperations<Vec<Locator>> for RtpsStatefulWriterImpl<C> {
    fn matched_reader_add(&mut self, _a_reader_proxy: RtpsReaderProxy<Vec<Locator>>) {
        todo!()
    }

    fn matched_reader_remove(&mut self, _reader_proxy_guid: &Guid) {
        todo!()
    }

    fn matched_reader_lookup(
        &self,
        _a_reader_guid: &Guid,
    ) -> Option<&RtpsReaderProxy<Vec<Locator>>> {
        todo!()
    }

    fn is_acked_by_all(&self) -> bool {
        todo!()
    }
}
