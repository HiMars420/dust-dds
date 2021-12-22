use crate::{
    behavior::types::Duration,
    structure::types::{ChangeKind, InstanceHandle, SequenceNumber},
};

pub trait RtpsWriterAttributes {
    type WriterHistoryCacheType;

    fn push_mode(&self) -> &bool;
    fn heartbeat_period(&self) -> &Duration;
    fn nack_response_delay(&self) -> &Duration;
    fn nack_suppression_duration(&self) -> &Duration;
    fn last_change_sequence_number(&self) -> &SequenceNumber;
    fn data_max_size_serialized(&self) -> &Option<i32>;
    fn writer_cache(&mut self) -> &mut Self::WriterHistoryCacheType;
}

pub trait RtpsWriterOperations {
    type DataType;
    type ParameterListType;
    type CacheChangeType;

    fn new_change(
        &mut self,
        kind: ChangeKind,
        data: Self::DataType,
        inline_qos: Self::ParameterListType,
        handle: InstanceHandle,
    ) -> Self::CacheChangeType;
}
