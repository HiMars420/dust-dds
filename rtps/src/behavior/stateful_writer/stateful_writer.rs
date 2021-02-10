use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{
    behavior::{types::Duration, Writer},
    types::{ReliabilityKind, TopicKind, GUID},
};

use super::ReaderProxy;

pub struct StatefulWriter {
    pub writer: Writer,
    matched_readers: HashMap<GUID, ReaderProxy>,
}

impl Deref for StatefulWriter {
    type Target = Writer;
    fn deref(&self) -> &Self::Target {
        &self.writer
    }
}
impl DerefMut for StatefulWriter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.writer
    }
}

impl StatefulWriter {
    pub fn new(
        guid: GUID,
        topic_kind: TopicKind,
        reliability_level: ReliabilityKind,
        push_mode: bool,
        data_max_sized_serialized: Option<i32>,
        heartbeat_period: Duration,
        nack_response_delay: Duration,
        nack_suppression_duration: Duration,
    ) -> Self {
        let writer = Writer::new(
            guid,
            topic_kind,
            reliability_level,
            push_mode,
            heartbeat_period,
            nack_response_delay,
            nack_suppression_duration,
            data_max_sized_serialized,
        );
        Self {
            writer,
            matched_readers: HashMap::new(),
        }
    }

    pub fn matched_reader_add(&mut self, a_reader_proxy: ReaderProxy) {
        let remote_reader_guid = a_reader_proxy.remote_reader_guid;
        self.matched_readers
            .insert(remote_reader_guid, a_reader_proxy);
    }

    pub fn matched_reader_remove(&mut self, reader_proxy_guid: &GUID) {
        self.matched_readers.remove(reader_proxy_guid);
    }

    pub fn matched_reader_lookup(&self, a_reader_guid: GUID) -> Option<&ReaderProxy> {
        match self.matched_readers.get(&a_reader_guid) {
            Some(rp) => Some(rp),
            None => None,
        }
    }

    pub fn is_acked_by_all(&self) -> bool {
        todo!()
    }
}
