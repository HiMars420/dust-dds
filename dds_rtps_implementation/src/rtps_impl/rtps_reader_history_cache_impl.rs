use rust_dds_api::dcps_psm::{InstanceStateKind, SampleStateKind, ViewStateKind};
use rust_rtps_pim::{
    messages::{submessage_elements::Parameter, types::Time},
    structure::{
        cache_change::RtpsCacheChange,
        history_cache::{
            RtpsHistoryCacheAddChange, RtpsHistoryCacheConstructor, RtpsHistoryCacheGetChange,
            RtpsHistoryCacheOperations,
        },
        types::{ChangeKind, Guid, InstanceHandle, SequenceNumber},
    },
};

use crate::dds_type::DdsDeserialize;

struct ReaderCacheChange<T> {
    kind: ChangeKind,
    writer_guid: Guid,
    sequence_number: SequenceNumber,
    instance_handle: InstanceHandle,
    data: T,
    _source_timestamp: Option<Time>,
    _reception_timestamp: Option<Time>,
    _sample_state_kind: SampleStateKind,
    _view_state_kind: ViewStateKind,
    _instance_state_kind: InstanceStateKind,
}

pub struct ReaderHistoryCache<T> {
    changes: Vec<ReaderCacheChange<T>>,
    source_timestamp: Option<Time>,
}

impl<T> ReaderHistoryCache<T> {
    /// Set the Rtps history cache impl's info.
    pub fn set_source_timestamp(&mut self, info: Option<Time>) {
        self.source_timestamp = info;
    }
}

impl<T> RtpsHistoryCacheConstructor for ReaderHistoryCache<T> {
    fn new() -> Self {
        Self {
            changes: Vec::new(),
            source_timestamp: None,
        }
    }
}

impl<T> RtpsHistoryCacheAddChange<&'_ [Parameter<&'_ [u8]>], &'_ [u8]> for ReaderHistoryCache<T>
where
    T: for<'a> DdsDeserialize<'a>,
{
    fn add_change(&mut self, mut change: RtpsCacheChange<&[Parameter<&[u8]>], &[u8]>) {
        let instance_state_kind = match change.kind {
            ChangeKind::Alive => InstanceStateKind::Alive,
            ChangeKind::AliveFiltered => InstanceStateKind::Alive,
            ChangeKind::NotAliveDisposed => InstanceStateKind::NotAliveDisposed,
            ChangeKind::NotAliveUnregistered => todo!(),
        };

        let current_time = std::time::SystemTime::now();
        let reception_timestamp = Time(
            current_time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );

        let data = DdsDeserialize::deserialize(&mut change.data_value).unwrap();

        let local_change = ReaderCacheChange {
            kind: change.kind,
            writer_guid: change.writer_guid,
            sequence_number: change.sequence_number,
            instance_handle: change.instance_handle,
            data,
            _source_timestamp: self.source_timestamp,
            _reception_timestamp: Some(reception_timestamp),
            _sample_state_kind: SampleStateKind::NotRead,
            _view_state_kind: ViewStateKind::New,
            _instance_state_kind: instance_state_kind,
        };

        self.changes.push(local_change)
    }
}

impl<'a, T> RtpsHistoryCacheGetChange<'a, &'a [Parameter<&'a [u8]>], &'a T>
    for ReaderHistoryCache<T>
{
    fn get_change(
        &'a self,
        seq_num: &SequenceNumber,
    ) -> Option<RtpsCacheChange<&'a [Parameter<&'a [u8]>], &'a T>> {
        let local_change = self
            .changes
            .iter()
            .find(|&cc| &cc.sequence_number == seq_num)?;

        Some(RtpsCacheChange {
            kind: local_change.kind,
            writer_guid: local_change.writer_guid,
            instance_handle: local_change.instance_handle,
            sequence_number: local_change.sequence_number,
            data_value: &local_change.data,
            inline_qos: &[],
        })
    }
}

impl<T> RtpsHistoryCacheOperations for ReaderHistoryCache<T> {
    fn remove_change(&mut self, seq_num: &SequenceNumber) {
        self.changes.retain(|cc| &cc.sequence_number != seq_num)
    }

    fn get_seq_num_min(&self) -> Option<SequenceNumber> {
        self.changes
            .iter()
            .map(|cc| cc.sequence_number)
            .min()
            .clone()
    }

    fn get_seq_num_max(&self) -> Option<SequenceNumber> {
        self.changes
            .iter()
            .map(|cc| cc.sequence_number)
            .max()
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use rust_rtps_pim::structure::types::GUID_UNKNOWN;

    use super::*;
    struct MockDdsDeserialize;

    impl DdsDeserialize<'_> for MockDdsDeserialize {
        fn deserialize(_buf: &mut &'_ [u8]) -> rust_dds_api::return_type::DDSResult<Self> {
            Ok(Self)
        }
    }

    #[test]
    fn add_change() {
        let mut hc: ReaderHistoryCache<MockDdsDeserialize> = ReaderHistoryCache::new();
        let change = RtpsCacheChange {
            kind: rust_rtps_pim::structure::types::ChangeKind::Alive,
            writer_guid: GUID_UNKNOWN,
            instance_handle: 0,
            sequence_number: 1,
            data_value: &[][..],
            inline_qos: &[][..],
        };
        hc.add_change(change);
        assert!(hc.get_change(&1).is_some());
    }

    #[test]
    fn remove_change() {
        let mut hc: ReaderHistoryCache<MockDdsDeserialize> = ReaderHistoryCache::new();
        let change = RtpsCacheChange {
            kind: rust_rtps_pim::structure::types::ChangeKind::Alive,
            writer_guid: GUID_UNKNOWN,
            instance_handle: 0,
            sequence_number: 1,
            data_value: &[][..],
            inline_qos: &[][..],
        };
        hc.add_change(change);
        hc.remove_change(&1);
        assert!(hc.get_change(&1).is_none());
    }

    #[test]
    fn get_change() {
        let mut hc: ReaderHistoryCache<MockDdsDeserialize> = ReaderHistoryCache::new();
        let change = RtpsCacheChange {
            kind: rust_rtps_pim::structure::types::ChangeKind::Alive,
            writer_guid: GUID_UNKNOWN,
            instance_handle: 0,
            sequence_number: 1,
            data_value: &[][..],
            inline_qos: &[][..],
        };
        hc.add_change(change);
        assert!(hc.get_change(&1).is_some());
        assert!(hc.get_change(&2).is_none());
    }

    #[test]
    fn get_seq_num_min() {
        let mut hc: ReaderHistoryCache<MockDdsDeserialize> = ReaderHistoryCache::new();
        let change1 = RtpsCacheChange {
            kind: rust_rtps_pim::structure::types::ChangeKind::Alive,
            writer_guid: GUID_UNKNOWN,
            instance_handle: 0,
            sequence_number: 1,
            data_value: &[][..],
            inline_qos: &[][..],
        };
        let change2 = RtpsCacheChange {
            kind: rust_rtps_pim::structure::types::ChangeKind::Alive,
            writer_guid: GUID_UNKNOWN,
            instance_handle: 0,
            sequence_number: 2,
            data_value: &[][..],
            inline_qos: &[][..],
        };
        hc.add_change(change1);
        hc.add_change(change2);
        assert_eq!(hc.get_seq_num_min(), Some(1));
    }

    #[test]
    fn get_seq_num_max() {
        let mut hc: ReaderHistoryCache<MockDdsDeserialize> = ReaderHistoryCache::new();
        let change1 = RtpsCacheChange {
            kind: rust_rtps_pim::structure::types::ChangeKind::Alive,
            writer_guid: GUID_UNKNOWN,
            instance_handle: 0,
            sequence_number: 1,
            data_value: &[][..],
            inline_qos: &[][..],
        };
        let change2 = RtpsCacheChange {
            kind: rust_rtps_pim::structure::types::ChangeKind::Alive,
            writer_guid: GUID_UNKNOWN,
            instance_handle: 0,
            sequence_number: 2,
            data_value: &[][..],
            inline_qos: &[][..],
        };
        hc.add_change(change1);
        hc.add_change(change2);
        assert_eq!(hc.get_seq_num_max(), Some(2));
    }
}
