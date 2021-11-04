use super::{cache_change::RtpsCacheChange, types::SequenceNumber};

pub trait RtpsHistoryCacheConstructor {
    /// This operation creates a new RTPS HistoryCache. The newly-created history cache is initialized with an empty list of changes.
    fn new() -> Self;
}

pub trait RtpsHistoryCacheAddChange<P, D> {
    /// This operation inserts the CacheChange a_change into the HistoryCache.
    /// This operation will only fail if there are not enough resources to add the change to the HistoryCache. It is the responsibility
    /// of the DDS service implementation to configure the HistoryCache in a manner consistent with the DDS Entity RESOURCE_LIMITS QoS
    /// and to propagate any errors to the DDS-user in the manner specified by the DDS specification.
    fn add_change(&mut self, change: RtpsCacheChange<P, D>);
}

pub trait RtpsHistoryCacheGetChange<'a, P, D> {
    fn get_change(&'a self, seq_num: &SequenceNumber) -> Option<RtpsCacheChange<P, D>>;
}

pub trait RtpsHistoryCacheOperations {
    /// This operation indicates that a previously-added CacheChange has become irrelevant and the details regarding the CacheChange need
    /// not be maintained in the HistoryCache. The determination of irrelevance is made based on the QoS associated with the related DDS
    /// entity and on the acknowledgment status of the CacheChange. This is described in 8.4.1.
    fn remove_change(&mut self, seq_num: &SequenceNumber);

    /// This operation retrieves the smallest value of the CacheChange::sequenceNumber attribute among the CacheChange stored in the HistoryCache.
    fn get_seq_num_min(&self) -> Option<SequenceNumber>;

    /// This operation retrieves the largest value of the CacheChange::sequenceNumber attribute among the CacheChange stored in the HistoryCache.
    fn get_seq_num_max(&self) -> Option<SequenceNumber>;
}
