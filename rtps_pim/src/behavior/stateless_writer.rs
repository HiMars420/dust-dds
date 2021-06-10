use crate::{
    behavior::RTPSWriter,
    messages::{
        submessage_elements::{self, SequenceNumberSet},
        submessages::{
            AckNackSubmessage, DataSubmessage, DataSubmessagePIM, GapSubmessage, GapSubmessagePIM,
        },
        types::{CountPIM, ParameterIdPIM, SubmessageKindPIM},
        RtpsSubmessageHeaderPIM,
    },
    structure::{
        types::{
            ChangeKind, DataPIM, EntityIdPIM, GUIDType, GuidPrefixPIM, InstanceHandlePIM,
            LocatorPIM, ParameterListPIM, SequenceNumberPIM, GUIDPIM,
        },
        RTPSCacheChange, RTPSHistoryCache,
    },
};

use super::types::DurationPIM;
pub trait RTPSReaderLocator<PSM: LocatorPIM + SequenceNumberPIM> {
    type SequenceNumberVector: IntoIterator<Item = PSM::SequenceNumberType>;

    fn locator(&self) -> &PSM::LocatorType;

    fn expects_inline_qos(&self) -> bool;

    fn next_requested_change(&mut self) -> Option<PSM::SequenceNumberType>;

    fn next_unsent_change(
        &mut self,
        last_change_sequence_number: &PSM::SequenceNumberType,
    ) -> Option<PSM::SequenceNumberType>;

    fn requested_changes(&self) -> Self::SequenceNumberVector;

    fn requested_changes_set(
        &mut self,
        req_seq_num_set: &[PSM::SequenceNumberType],
        last_change_sequence_number: &PSM::SequenceNumberType,
    );

    fn unsent_changes(
        &self,
        last_change_sequence_number: PSM::SequenceNumberType,
    ) -> Self::SequenceNumberVector;
}

pub trait RTPSStatelessWriter<
    PSM: GuidPrefixPIM
        + EntityIdPIM
        + DurationPIM
        + DataPIM
        + InstanceHandlePIM
        + LocatorPIM
        + SequenceNumberPIM
        + GUIDPIM<PSM>
        + ParameterIdPIM
        + ParameterListPIM<PSM>,
>: RTPSWriter<PSM>
{
    type ReaderLocatorPIM: RTPSReaderLocator<PSM>;

    fn reader_locators(&mut self) -> (&mut [Self::ReaderLocatorPIM], &Self::HistoryCacheType);

    fn reader_locator_add(&mut self, a_locator: Self::ReaderLocatorPIM);

    fn reader_locator_remove(&mut self, a_locator: &PSM::LocatorType);

    fn unsent_changes_reset(&mut self);
}

pub fn best_effort_send_unsent_data<
    'a,
    PSM: LocatorPIM
        + SequenceNumberPIM
        + GuidPrefixPIM
        + EntityIdPIM
        + InstanceHandlePIM
        + DataPIM
        + ParameterIdPIM
        + ParameterListPIM<PSM>
        + GUIDPIM<PSM>
        + SubmessageKindPIM
        + RtpsSubmessageHeaderPIM<PSM>
        + DataSubmessagePIM<'a, PSM>
        + GapSubmessagePIM<PSM>,
    HistoryCache: RTPSHistoryCache<PSM>,
>(
    reader_locator: &mut impl RTPSReaderLocator<PSM>,
    last_change_sequence_number: &PSM::SequenceNumberType,
    writer_cache: &'a HistoryCache,
    mut send_data: impl FnMut(<PSM as DataSubmessagePIM<'a, PSM>>::DataSubmessageType),
    mut send_gap: impl FnMut(PSM::GapSubmessageType),
) where
    PSM::DataType: 'a,
    PSM::ParameterListType: 'a,
    HistoryCache::CacheChange: 'a,
{
    while let Some(seq_num) = reader_locator.next_unsent_change(&last_change_sequence_number) {
        if let Some(change) = writer_cache.get_change(&seq_num) {
            let endianness_flag = true;
            let inline_qos_flag = true;
            let (data_flag, key_flag) = match change.kind() {
                ChangeKind::Alive => (true, false),
                ChangeKind::NotAliveDisposed | ChangeKind::NotAliveUnregistered => (false, true),
                _ => todo!(),
            };
            let non_standard_payload_flag = false;
            let reader_id = submessage_elements::EntityId::new(&PSM::ENTITYID_UNKNOWN);
            let writer_id = submessage_elements::EntityId::new(change.writer_guid().entity_id());
            let writer_sn = submessage_elements::SequenceNumber::new(change.sequence_number());
            let inline_qos = change.inline_qos();
            let data = change.data_value();
            let serialized_payload = submessage_elements::SerializedData::new(data.as_ref());
            let data_submessage = PSM::DataSubmessageType::new(
                endianness_flag,
                inline_qos_flag,
                data_flag,
                key_flag,
                non_standard_payload_flag,
                reader_id,
                writer_id,
                writer_sn,
                inline_qos,
                serialized_payload,
            );
            send_data(data_submessage)
        } else {
            let endianness_flag = true;
            let reader_id = submessage_elements::EntityId::new(&PSM::ENTITYID_UNKNOWN);
            let writer_id = submessage_elements::EntityId::new(&PSM::ENTITYID_UNKNOWN);
            let gap_start = submessage_elements::SequenceNumber::new(&seq_num);
            let set = &[];
            let gap_list = submessage_elements::SequenceNumberSet::new(&seq_num, set);
            let gap_submessage = PSM::GapSubmessageType::new(
                endianness_flag,
                reader_id,
                writer_id,
                gap_start,
                gap_list,
            );
            send_gap(gap_submessage)
        }
    }
}

pub fn reliable_send_unsent_data<PSM: LocatorPIM + SequenceNumberPIM>(
    reader_locator: &mut impl RTPSReaderLocator<PSM>,
    last_change_sequence_number: PSM::SequenceNumberType,
    mut send: impl FnMut(PSM::SequenceNumberType),
) {
    while let Some(seq_num) = reader_locator.next_unsent_change(&last_change_sequence_number) {
        send(seq_num)
    }
}

pub fn reliable_receive_acknack<
    PSM: LocatorPIM
        + SequenceNumberPIM
        + SubmessageKindPIM
        + EntityIdPIM
        + CountPIM
        + RtpsSubmessageHeaderPIM<PSM>,
>(
    reader_locator: &mut impl RTPSReaderLocator<PSM>,
    acknack: &impl AckNackSubmessage<PSM>,
    last_change_sequence_number: &PSM::SequenceNumberType,
) {
    reader_locator
        .requested_changes_set(acknack.reader_sn_state().set(), last_change_sequence_number);
}

pub fn reliable_after_nack_response_delay<PSM: LocatorPIM + SequenceNumberPIM>(
    reader_locator: &mut impl RTPSReaderLocator<PSM>,
    mut send: impl FnMut(PSM::SequenceNumberType),
) {
    while let Some(seq_num) = reader_locator.next_requested_change() {
        send(seq_num)
    }
}

#[cfg(test)]
mod tests {
    use core::iter::FromIterator;

    use crate::{
        messages::{
            submessage_elements::{Parameter, ParameterList},
            RtpsSubmessageHeaderType, Submessage,
        },
        structure::{
            types::{GUIDType, LocatorType},
            RTPSCacheChange, RTPSHistoryCache,
        },
    };

    use super::*;

    #[derive(Clone, Copy, PartialEq)]
    struct MockGUID;

    impl GUIDType<MockPSM> for [u8; 16] {
        fn new(_prefix: [u8; 12], _entity_id: [u8; 4]) -> Self {
            todo!()
        }

        fn prefix(&self) -> &[u8; 12] {
            todo!()
        }

        fn entity_id(&self) -> &[u8; 4] {
            &MockPSM::ENTITYID_UNKNOWN
        }
    }
    #[derive(Clone, Copy, PartialEq)]
    struct MockLocator;

    impl LocatorType for MockLocator {
        type LocatorKind = [u8; 4];

        type LocatorPort = [u8; 4];

        type LocatorAddress = [u8; 16];

        fn kind(&self) -> &Self::LocatorKind {
            todo!()
        }

        fn port(&self) -> &Self::LocatorPort {
            todo!()
        }

        fn address(&self) -> &Self::LocatorAddress {
            todo!()
        }
    }

    struct MockParameter;

    impl Parameter<MockPSM> for MockParameter {
        fn parameter_id(&self) -> () {
            todo!()
        }

        fn length(&self) -> i16 {
            todo!()
        }

        fn value(&self) -> &[u8] {
            todo!()
        }
    }
    struct MockParameterList;

    impl ParameterList<MockPSM> for MockParameterList {
        type Parameter = MockParameter;

        fn new(_parameter: &[Self::Parameter]) -> Self {
            todo!()
        }

        fn parameter(&self) -> &[Self::Parameter] {
            todo!()
        }
    }

    struct MockPSM;

    impl ParameterIdPIM for MockPSM {
        type ParameterIdType = ();
    }

    impl ParameterListPIM<MockPSM> for MockPSM {
        type ParameterListType = MockParameterList;
    }

    impl DataPIM for MockPSM {
        type DataType = [u8; 0];
    }

    impl InstanceHandlePIM for MockPSM {
        type InstanceHandleType = ();
    }

    impl EntityIdPIM for MockPSM {
        type EntityIdType = [u8; 4];
        const ENTITYID_UNKNOWN: Self::EntityIdType = [0; 4];
        const ENTITYID_PARTICIPANT: Self::EntityIdType = [0; 4];
    }

    impl GuidPrefixPIM for MockPSM {
        type GuidPrefixType = [u8; 12];
        const GUIDPREFIX_UNKNOWN: Self::GuidPrefixType = [0; 12];
    }

    impl GUIDPIM<Self> for MockPSM {
        type GUIDType = [u8; 16];
        const GUID_UNKNOWN: Self::GUIDType = [0; 16];
    }

    impl SequenceNumberPIM for MockPSM {
        type SequenceNumberType = i64;
        const SEQUENCE_NUMBER_UNKNOWN: Self::SequenceNumberType = -1;
    }

    impl LocatorPIM for MockPSM {
        type LocatorType = MockLocator;

        const LOCATOR_INVALID: Self::LocatorType = MockLocator;
        const LOCATOR_KIND_INVALID: <Self::LocatorType as LocatorType>::LocatorKind = [0; 4];
        const LOCATOR_KIND_RESERVED: <Self::LocatorType as LocatorType>::LocatorKind = [0; 4];
        #[allow(non_upper_case_globals)]
        const LOCATOR_KIND_UDPv4: <Self::LocatorType as LocatorType>::LocatorKind = [0; 4];
        #[allow(non_upper_case_globals)]
        const LOCATOR_KIND_UDPv6: <Self::LocatorType as LocatorType>::LocatorKind = [0; 4];

        const LOCATOR_PORT_INVALID: <Self::LocatorType as LocatorType>::LocatorPort = [0; 4];

        const LOCATOR_ADDRESS_INVALID: <Self::LocatorType as LocatorType>::LocatorAddress = [0; 16];
    }

    impl SubmessageKindPIM for MockPSM {
        type SubmessageKindType = u8;

        const DATA: Self::SubmessageKindType = 0;
        const GAP: Self::SubmessageKindType = 0;
        const HEARTBEAT: Self::SubmessageKindType = 0;
        const ACKNACK: Self::SubmessageKindType = 0;
        const PAD: Self::SubmessageKindType = 0;
        const INFO_TS: Self::SubmessageKindType = 0;
        const INFO_REPLY: Self::SubmessageKindType = 0;
        const INFO_DST: Self::SubmessageKindType = 0;
        const INFO_SRC: Self::SubmessageKindType = 0;
        const DATA_FRAG: Self::SubmessageKindType = 0;
        const NACK_FRAG: Self::SubmessageKindType = 0;
        const HEARTBEAT_FRAG: Self::SubmessageKindType = 0;
    }

    impl<'a> DataSubmessagePIM<'a, Self> for MockPSM {
        type DataSubmessageType = MockDataSubmessage;
    }

    impl RtpsSubmessageHeaderPIM<Self> for MockPSM {
        type RtpsSubmessageHeaderType = MockSubmessageHeader;
    }

    struct MockSubmessageHeader;

    impl RtpsSubmessageHeaderType<MockPSM> for MockSubmessageHeader {
        fn submessage_id(&self) -> u8 {
            todo!()
        }

        fn flags(&self) -> [bool; 8] {
            todo!()
        }

        fn submessage_length(&self) -> u16 {
            todo!()
        }
    }

    struct MockDataSubmessage(i64);

    impl Submessage<MockPSM> for MockDataSubmessage {
        fn submessage_header(&self) -> MockSubmessageHeader {
            todo!()
        }
    }

    impl submessage_elements::EntityId<MockPSM> for [u8; 4] {
        fn new(value: &[u8; 4]) -> Self {
            value.clone()
        }

        fn value(&self) -> &[u8; 4] {
            self
        }
    }

    impl submessage_elements::SequenceNumber<MockPSM> for i64 {
        fn new(value: &i64) -> Self {
            value.clone()
        }

        fn value(&self) -> &i64 {
            self
        }
    }

    impl<'a> submessage_elements::SerializedData<'a> for &'a [u8] {
        fn new(value: &'a [u8]) -> Self {
            value
        }

        fn value(&self) -> &[u8] {
            self
        }
    }

    impl<'a> DataSubmessage<'a, MockPSM> for MockDataSubmessage {
        type EntityId = [u8; 4];
        type SequenceNumber = i64;
        type SerializedData = &'a [u8];

        fn new(
            _endianness_flag: bool,
            _inline_qos_flag: bool,
            _data_flag: bool,
            _key_flag: bool,
            _non_standard_payload_flag: bool,
            _reader_id: Self::EntityId,
            _writer_id: Self::EntityId,
            writer_sn: Self::SequenceNumber,
            _inline_qos: &'a MockParameterList,
            _serialized_payload: Self::SerializedData,
        ) -> Self {
            Self(writer_sn)
        }

        fn endianness_flag(&self) -> bool {
            todo!()
        }

        fn inline_qos_flag(&self) -> bool {
            todo!()
        }

        fn data_flag(&self) -> bool {
            todo!()
        }

        fn key_flag(&self) -> bool {
            todo!()
        }

        fn non_standard_payload_flag(&self) -> bool {
            todo!()
        }

        fn reader_id(&self) -> &Self::EntityId {
            todo!()
        }

        fn writer_id(&self) -> &Self::EntityId {
            todo!()
        }

        fn writer_sn(&self) -> &Self::SequenceNumber {
            &self.0
        }

        fn inline_qos(&self) -> &MockParameterList {
            todo!()
        }

        fn serialized_payload(&self) -> &Self::SerializedData {
            todo!()
        }
    }

    struct MockSequenceNumberVectorIter;
    impl Iterator for MockSequenceNumberVectorIter {
        type Item = i64;

        fn next(&mut self) -> Option<Self::Item> {
            todo!()
        }
    }
    struct MockSequenceNumberVector;

    impl FromIterator<i64> for MockSequenceNumberVector {
        fn from_iter<T: IntoIterator<Item = i64>>(_iter: T) -> Self {
            MockSequenceNumberVector
        }
    }

    impl IntoIterator for MockSequenceNumberVector {
        type Item = i64;
        type IntoIter = MockSequenceNumberVectorIter;

        fn into_iter(self) -> Self::IntoIter {
            todo!()
        }
    }

    impl GapSubmessagePIM<MockPSM> for MockPSM {
        type GapSubmessageType = MockGapSubmessage;
    }

    struct MockSequenceNumberSet;

    impl submessage_elements::SequenceNumberSet<MockPSM> for MockSequenceNumberSet {
        fn new(_base: &i64, _set: &[i64]) -> Self {
            MockSequenceNumberSet
        }

        fn base(&self) -> &i64 {
            todo!()
        }

        fn set(&self) -> &[i64] {
            todo!()
        }
    }

    struct MockGapSubmessage(i64);

    impl Submessage<MockPSM> for MockGapSubmessage {
        fn submessage_header(&self) -> MockSubmessageHeader {
            todo!()
        }
    }

    impl GapSubmessage<MockPSM> for MockGapSubmessage {
        type EntityId = [u8; 4];
        type SequenceNumber = i64;
        type SequenceNumberSet = MockSequenceNumberSet;

        fn new(
            _endianness_flag: bool,
            _reader_id: Self::EntityId,
            _writer_id: Self::EntityId,
            gap_start: Self::SequenceNumber,
            _gap_list: Self::SequenceNumberSet,
        ) -> Self {
            MockGapSubmessage(gap_start)
        }

        fn endianness_flag(&self) -> bool {
            todo!()
        }

        fn reader_id(&self) -> &Self::EntityId {
            todo!()
        }

        fn writer_id(&self) -> &Self::EntityId {
            todo!()
        }

        fn gap_start(&self) -> &Self::SequenceNumber {
            &self.0
        }

        fn gap_list(&self) -> &Self::SequenceNumberSet {
            todo!()
        }
    }

    struct MockReaderLocator {
        last_sent_sequence_number: i64,
    }

    impl<'a> RTPSReaderLocator<MockPSM> for MockReaderLocator {
        type SequenceNumberVector = Option<i64>;

        fn locator(&self) -> &MockLocator {
            &MockLocator
        }

        fn expects_inline_qos(&self) -> bool {
            todo!()
        }

        fn next_requested_change(&mut self) -> Option<i64> {
            todo!()
        }

        fn next_unsent_change(&mut self, last_change_sequence_number: &i64) -> Option<i64> {
            if &self.last_sent_sequence_number < last_change_sequence_number {
                self.last_sent_sequence_number += 1;
                Some(self.last_sent_sequence_number)
            } else {
                None
            }
        }

        fn requested_changes(&self) -> Self::SequenceNumberVector {
            todo!()
        }

        fn requested_changes_set(
            &mut self,
            _req_seq_num_set: &[i64],
            _last_change_sequence_number: &i64,
        ) {
            todo!()
        }

        fn unsent_changes(&self, _last_change_sequence_number: i64) -> Self::SequenceNumberVector {
            todo!()
        }
    }

    struct MockCacheChange {
        kind: ChangeKind,
        sequence_number: i64,
    }

    impl RTPSCacheChange<MockPSM> for MockCacheChange {
        fn kind(&self) -> crate::structure::types::ChangeKind {
            self.kind
        }

        fn writer_guid(&self) -> &[u8; 16] {
            &[0; 16]
        }

        fn instance_handle(&self) -> &() {
            todo!()
        }

        fn sequence_number(&self) -> &i64 {
            &self.sequence_number
        }

        fn data_value(&self) -> &[u8; 0] {
            &[]
        }

        fn inline_qos(&self) -> &MockParameterList {
            &MockParameterList
        }
    }
    struct MockHistoryCache<const N: usize> {
        changes: [MockCacheChange; N],
    }

    impl<const N: usize> RTPSHistoryCache<MockPSM> for MockHistoryCache<N> {
        type CacheChange = MockCacheChange;

        fn new() -> Self
        where
            Self: Sized,
        {
            todo!()
        }

        fn add_change(&mut self, _change: Self::CacheChange) {
            todo!()
        }

        fn remove_change(&mut self, _seq_num: &i64) {
            todo!()
        }

        fn get_change(&self, seq_num: &i64) -> Option<&Self::CacheChange> {
            self.changes.iter().find(|&x| &x.sequence_number == seq_num)
        }

        fn get_seq_num_min(&self) -> Option<&i64> {
            todo!()
        }

        fn get_seq_num_max(&self) -> Option<&i64> {
            todo!()
        }
    }

    #[test]
    fn stateless_writer_best_effort_send_unsent_data_only_data() {
        let mut sent_data_seq_num = [0, 0];
        let mut total_data = 0;
        let mut sent_gap_seq_num = [];
        let mut total_gap = 0;

        let expected_total_data = 2;
        let expected_sent_data_seq_num = [1, 2];
        let expected_total_gap = 0;
        let expected_sent_gap_seq_num = [];

        let mut reader_locator = MockReaderLocator {
            last_sent_sequence_number: 0,
        };
        let writer_cache = MockHistoryCache::<2> {
            changes: [
                MockCacheChange {
                    kind: ChangeKind::Alive,
                    sequence_number: 1,
                },
                MockCacheChange {
                    kind: ChangeKind::Alive,
                    sequence_number: 2,
                },
            ],
        };
        best_effort_send_unsent_data(
            &mut reader_locator,
            &2,
            &writer_cache,
            |data_submessage| {
                sent_data_seq_num[total_data] = data_submessage.writer_sn().clone();
                total_data += 1;
            },
            |gap_submessage| {
                sent_gap_seq_num[total_gap] = gap_submessage.gap_start().clone();
                total_gap += 1;
            },
        );

        assert_eq!(total_data, expected_total_data);
        assert_eq!(sent_data_seq_num, expected_sent_data_seq_num);
        assert_eq!(total_gap, expected_total_gap);
        assert_eq!(sent_gap_seq_num, expected_sent_gap_seq_num);
    }

    #[test]
    fn stateless_writer_best_effort_send_unsent_data_only_gap() {
        let mut sent_data_seq_num = [];
        let mut total_data = 0;
        let mut sent_gap_seq_num = [0, 0];
        let mut total_gap = 0;

        let expected_total_data = 0;
        let expected_sent_data_seq_num = [];
        let expected_total_gap = 2;
        let expected_sent_gap_seq_num = [1, 2];

        let mut reader_locator = MockReaderLocator {
            last_sent_sequence_number: 0,
        };
        let writer_cache = MockHistoryCache::<0> { changes: [] };
        best_effort_send_unsent_data(
            &mut reader_locator,
            &2,
            &writer_cache,
            |data_submessage| {
                sent_data_seq_num[total_data] = data_submessage.writer_sn().clone();
                total_data += 1;
            },
            |gap_submessage| {
                sent_gap_seq_num[total_gap] = gap_submessage.gap_start().clone();
                total_gap += 1;
            },
        );

        assert_eq!(total_data, expected_total_data);
        assert_eq!(sent_data_seq_num, expected_sent_data_seq_num);
        assert_eq!(total_gap, expected_total_gap);
        assert_eq!(sent_gap_seq_num, expected_sent_gap_seq_num);
    }

    #[test]
    fn stateless_writer_best_effort_send_unsent_data_data_and_gap() {
        let mut sent_data_seq_num = [0];
        let mut total_data = 0;
        let mut sent_gap_seq_num = [0];
        let mut total_gap = 0;

        let expected_total_data = 1;
        let expected_sent_data_seq_num = [2];
        let expected_total_gap = 1;
        let expected_sent_gap_seq_num = [1];

        let mut reader_locator = MockReaderLocator {
            last_sent_sequence_number: 0,
        };
        let writer_cache = MockHistoryCache::<1> {
            changes: [MockCacheChange {
                kind: ChangeKind::Alive,
                sequence_number: 2,
            }],
        };
        best_effort_send_unsent_data(
            &mut reader_locator,
            &2,
            &writer_cache,
            |data_submessage| {
                sent_data_seq_num[total_data] = data_submessage.writer_sn().clone();
                total_data += 1;
            },
            |gap_submessage| {
                sent_gap_seq_num[total_gap] = gap_submessage.gap_start().clone();
                total_gap += 1;
            },
        );

        assert_eq!(total_data, expected_total_data);
        assert_eq!(sent_data_seq_num, expected_sent_data_seq_num);
        assert_eq!(total_gap, expected_total_gap);
        assert_eq!(sent_gap_seq_num, expected_sent_gap_seq_num);
    }
}
