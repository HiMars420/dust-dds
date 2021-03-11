use rust_rtps::{
    behavior::stateless_writer::RTPSReaderLocator,
    structure::HistoryCache,
    types::{Locator, SequenceNumber},
};

pub struct ReaderLocator<'a, T: HistoryCache> {
    locator: Locator,
    writer_cache: &'a T,
    next_unsent_change: SequenceNumber,
}

impl<'a, T: HistoryCache> ReaderLocator<'a, T> {
    fn new(locator: Locator, writer_cache: &'a T) -> Self {
        Self {
            locator,
            writer_cache,
            next_unsent_change: 1,
        }
    }
}

impl<'a, T: HistoryCache> RTPSReaderLocator for ReaderLocator<'a, T> {
    type CacheChangeRepresentation = SequenceNumber;
    type CacheChangeRepresentationList = Vec<Self::CacheChangeRepresentation>;

    fn requested_changes(&self) -> Self::CacheChangeRepresentationList {
        todo!()
    }

    fn unsent_changes(&self) -> Self::CacheChangeRepresentationList {
        let max_history_cache_seq_num = self.writer_cache.get_seq_num_max().unwrap_or(0);
        (self.next_unsent_change..=max_history_cache_seq_num).collect()
    }

    fn locator(&self) -> Locator {
        self.locator
    }

    fn expects_inline_qos(&self) -> bool {
        todo!()
    }

    fn next_requested_change(&mut self) -> Option<&Self::CacheChangeRepresentation> {
        todo!()
    }

    fn next_unsent_change(&mut self) -> Option<&Self::CacheChangeRepresentation> {
        todo!()
        // next_seq_num := MIN { change.sequenceNumber
        //     SUCH-THAT change IN this.unsent_changes()};

        //     return change IN this.unsent_changes()
        //     SUCH-THAT (change.sequenceNumber == next_seq_num);
    }

    fn requested_changes_set(&mut self, req_seq_num_set: &[SequenceNumber]) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use rust_rtps::structure::CacheChange;

    use super::*;

    struct MockCacheChangeType;

    impl CacheChange for MockCacheChangeType {
        type Data = u8;

        fn new(
            _kind: rust_rtps::types::ChangeKind,
            _writer_guid: rust_rtps::types::GUID,
            _instance_handle: rust_rtps::types::InstanceHandle,
            _sequence_number: SequenceNumber,
            _data_value: Self::Data,
            _inline_qos: rust_rtps::messages::submessages::submessage_elements::ParameterList,
        ) -> Self {
            todo!()
        }

        fn kind(&self) -> rust_rtps::types::ChangeKind {
            todo!()
        }

        fn writer_guid(&self) -> rust_rtps::types::GUID {
            todo!()
        }

        fn instance_handle(&self) -> &rust_rtps::types::InstanceHandle {
            todo!()
        }

        fn sequence_number(&self) -> SequenceNumber {
            todo!()
        }

        fn data_value(&self) -> &Self::Data {
            todo!()
        }

        fn inline_qos(
            &self,
        ) -> &rust_rtps::messages::submessages::submessage_elements::ParameterList {
            todo!()
        }
    }

    struct MockHistoryCache {
        seq_num_min: Option<SequenceNumber>,
        seq_num_max: Option<SequenceNumber>,
    }

    impl HistoryCache for MockHistoryCache {
        type CacheChangeType = MockCacheChangeType;

        fn add_change(&mut self, _change: Self::CacheChangeType) {
            todo!()
        }

        fn remove_change(&mut self, _seq_num: SequenceNumber) {
            todo!()
        }

        fn get_change(&self, _seq_num: SequenceNumber) -> Option<&Self::CacheChangeType> {
            todo!()
        }

        fn get_seq_num_min(&self) -> Option<SequenceNumber> {
            self.seq_num_min
        }

        fn get_seq_num_max(&self) -> Option<SequenceNumber> {
            self.seq_num_max
        }
    }
    #[test]
    fn empty_unsent_changes() {
        let locator = Locator::new_udpv4(7400, [127, 0, 0, 1]);
        let history_cache = MockHistoryCache {
            seq_num_min: Some(0),
            seq_num_max: Some(0),
        };
        let reader_locator = ReaderLocator::new(locator, &history_cache);
        let unsent_changes = reader_locator.unsent_changes();

        assert!(unsent_changes.is_empty());
    }

    #[test]
    fn some_unsent_changes() {
        let locator = Locator::new_udpv4(7400, [127, 0, 0, 1]);
        let history_cache = MockHistoryCache {
            seq_num_min: Some(1),
            seq_num_max: Some(5),
        };
        let reader_locator = ReaderLocator::new(locator, &history_cache);
        let unsent_changes = reader_locator.unsent_changes();
        let expected_unsent_changes = vec![1, 2, 3, 4, 5];

        assert_eq!(unsent_changes, expected_unsent_changes);
    }
}
