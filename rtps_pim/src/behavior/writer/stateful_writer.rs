use crate::{
    behavior::types::Duration,
    structure::types::{Guid, Locator, ReliabilityKind, TopicKind},
};

use super::reader_proxy::RtpsReaderProxy;

// pub struct RtpsStatefulWriter<L, C, R> {
//     pub writer: RtpsWriter<L, C>,
//     pub matched_readers: R,
// }

// impl<L, C, R> RtpsStatefulWriter<L, C, R>
// where
//     R: Default,
//     C: RtpsHistoryCacheConstructor,
// {
//     pub fn new(
//         guid: Guid,
//         topic_kind: TopicKind,
//         reliability_level: ReliabilityKind,
//         unicast_locator_list: L,
//         multicast_locator_list: L,
//         push_mode: bool,
//         heartbeat_period: Duration,
//         nack_response_delay: Duration,
//         nack_suppression_duration: Duration,
//         data_max_size_serialized: Option<i32>,
//     ) -> Self {
//         Self {
//             writer: RtpsWriter::new(
//                 guid,
//                 topic_kind,
//                 reliability_level,
//                 unicast_locator_list,
//                 multicast_locator_list,
//                 push_mode,
//                 heartbeat_period,
//                 nack_response_delay,
//                 nack_suppression_duration,
//                 data_max_size_serialized,
//             ),
//             matched_readers: R::default(),
//         }
//     }
// }

pub trait RtpsStatefulWriterAttributes {
    fn matched_readers(&self);
}

pub trait RtpsStatefulWriterConstructor {
    fn new(
        guid: Guid,
        topic_kind: TopicKind,
        reliability_level: ReliabilityKind,
        unicast_locator_list: &[Locator],
        multicast_locator_list: &[Locator],
        push_mode: bool,
        heartbeat_period: Duration,
        nack_response_delay: Duration,
        nack_suppression_duration: Duration,
        data_max_size_serialized: Option<i32>,
    ) -> Self;
}

pub trait RtpsStatefulWriterOperations<L> {
    type ReaderProxyType;

    fn matched_reader_add(&mut self, a_reader_proxy: RtpsReaderProxy<L>);

    fn matched_reader_remove(&mut self, reader_proxy_guid: &Guid);

    fn matched_reader_lookup(&self, a_reader_guid: &Guid) -> Option<&Self::ReaderProxyType>;

    fn is_acked_by_all(&self) -> bool;
}
