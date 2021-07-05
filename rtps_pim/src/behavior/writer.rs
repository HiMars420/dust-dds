use crate::structure::{
    types::{ChangeKind, SequenceNumber},
    RTPSCacheChange, RTPSHistoryCache,
};

pub trait RTPSWriter {
    type HistoryCacheType: RTPSHistoryCache;
    type DurationType;

    fn push_mode(&self) -> bool;
    fn heartbeat_period(&self) -> &Self::DurationType;
    fn nack_response_delay(&self) -> &Self::DurationType;
    fn nack_suppression_duration(&self) -> &Self::DurationType;
    fn last_change_sequence_number(&self) -> &SequenceNumber;
    fn data_max_size_serialized(&self) -> i32;
    fn writer_cache(&self) -> &Self::HistoryCacheType;
    fn writer_cache_mut(&mut self) -> &mut Self::HistoryCacheType;

    fn new_change(
        &mut self,
        kind: ChangeKind,
        data: <<Self::HistoryCacheType as RTPSHistoryCache>::CacheChange as RTPSCacheChange>::DataType,
        inline_qos: <<Self::HistoryCacheType as RTPSHistoryCache>::CacheChange as RTPSCacheChange>::InlineQosType,
        handle: <<Self::HistoryCacheType as RTPSHistoryCache>::CacheChange as RTPSCacheChange>::InstanceHandleType,
    ) -> <Self::HistoryCacheType as RTPSHistoryCache>::CacheChange
    where
        <Self::HistoryCacheType as RTPSHistoryCache>::CacheChange: RTPSCacheChange;
}
