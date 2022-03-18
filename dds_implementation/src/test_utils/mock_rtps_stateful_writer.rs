use mockall::mock;
use rtps_pim::{
    behavior::{
        types::Duration,
        writer::{
            stateful_writer::RtpsStatefulWriterConstructor,
            writer::{RtpsWriterAttributes, RtpsWriterOperations},
        },
    },
    structure::types::{
        ChangeKind, Guid, InstanceHandle, Locator, ReliabilityKind, SequenceNumber, TopicKind,
    },
};

use super::{
    mock_rtps_cache_change::MockRtpsCacheChange, mock_rtps_history_cache::MockRtpsHistoryCache,
};

mock! {
    pub RtpsStatefulWriter{}

    impl RtpsWriterAttributes for RtpsStatefulWriter {
        type HistoryCacheType = MockRtpsHistoryCache;

        fn push_mode(&self) -> bool;
        fn heartbeat_period(&self) -> Duration;
        fn nack_response_delay(&self) -> Duration;
        fn nack_suppression_duration(&self) -> Duration;
        fn last_change_sequence_number(&self) -> SequenceNumber;
        fn data_max_size_serialized(&self) -> Option<i32>;
        fn writer_cache(&mut self) -> &mut MockRtpsHistoryCache;
    }

    impl RtpsWriterOperations for RtpsStatefulWriter {
        type DataType = Vec<u8>;
        type ParameterListType = Vec<u8>;
        type CacheChangeType = MockRtpsCacheChange;

        fn new_change(
            &mut self,
            kind: ChangeKind,
            data: Vec<u8>,
            inline_qos: Vec<u8>,
            handle: InstanceHandle,
        ) -> MockRtpsCacheChange;
    }
}

impl RtpsStatefulWriterConstructor for MockRtpsStatefulWriter {
    fn new(
        _guid: Guid,
        _topic_kind: TopicKind,
        _reliability_level: ReliabilityKind,
        _unicast_locator_list: &[Locator],
        _multicast_locator_list: &[Locator],
        _push_mode: bool,
        _heartbeat_period: Duration,
        _nack_response_delay: Duration,
        _nack_suppression_duration: Duration,
        _data_max_size_serialized: Option<i32>,
    ) -> Self {
        MockRtpsStatefulWriter::new()
    }
}
