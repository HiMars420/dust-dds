use rust_dds_api::{
    builtin_topics::PublicationBuiltinTopicData,
    dcps_psm::{
        InstanceHandle, InstanceStateKind, LivelinessChangedStatus, RequestedDeadlineMissedStatus,
        RequestedIncompatibleQosStatus, SampleLostStatus, SampleRejectedStatus, SampleStateKind,
        StatusMask, SubscriptionMatchedStatus, ViewStateKind,
    },
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::DataReaderQos,
        read_condition::ReadCondition,
        sample_info::SampleInfo,
    },
    return_type::DDSResult,
    subscription::{
        data_reader::AnyDataReader, data_reader_listener::DataReaderListener,
        query_condition::QueryCondition, subscriber::Subscriber,
    },
    topic::topic_description::TopicDescription,
};
use rust_rtps_pim::{
    behavior::reader::reader::RTPSReader,
    structure::{RTPSCacheChange, RTPSHistoryCache},
};

use crate::utils::shared_object::RtpsWeak;

pub struct DataReaderImpl<'dr, T: 'static, Reader> {
    _subscriber: &'dr dyn Subscriber,
    _topic: &'dr dyn TopicDescription<T>,
    reader: RtpsWeak<Reader>,
}

impl<'dr, T, Reader> rust_dds_api::subscription::data_reader::DataReader<T>
    for DataReaderImpl<'dr, T, Reader>
where
    T: for<'de> serde::Deserialize<'de>,
    Reader: RTPSReader,
    Reader::HistoryCacheType: RTPSHistoryCache,
    <Reader::HistoryCacheType as RTPSHistoryCache>::CacheChange: RTPSCacheChange,
{
    type Samples = T;

    fn read(
        &self,
        _max_samples: i32,
        _sample_states: &[SampleStateKind],
        _view_states: &[ViewStateKind],
        _instance_states: &[InstanceStateKind],
    ) -> DDSResult<Self::Samples> {
        let shared_reader = self.reader.upgrade()?;
        let reader = shared_reader.lock();
        let reader_cache = reader.reader_cache();
        let cc1 = reader_cache.get_change(&1).unwrap();
        let data = cc1.data_value();
        let value = rust_serde_cdr::deserializer::from_bytes(data).unwrap();
        Ok(value)
    }

    fn take(
        &self,
        _data_values: &mut [T],
        _sample_infos: &mut [SampleInfo],
        _max_samples: i32,
        _sample_states: &[SampleStateKind],
        _view_states: &[ViewStateKind],
        _instance_states: &[InstanceStateKind],
    ) -> DDSResult<()> {
        todo!()
    }

    fn read_w_condition(
        &self,
        _data_values: &mut [T],
        _sample_infos: &mut [SampleInfo],
        _max_samples: i32,
        _a_condition: ReadCondition,
    ) -> DDSResult<()> {
        todo!()
    }

    fn take_w_condition(
        &self,
        _data_values: &mut [T],
        _sample_infos: &mut [SampleInfo],
        _max_samples: i32,
        _a_condition: ReadCondition,
    ) -> DDSResult<()> {
        todo!()
    }

    fn read_next_sample(
        &self,
        _data_value: &mut [T],
        _sample_info: &mut [SampleInfo],
    ) -> DDSResult<()> {
        todo!()
    }

    fn take_next_sample(
        &self,
        _data_value: &mut [T],
        _sample_info: &mut [SampleInfo],
    ) -> DDSResult<()> {
        todo!()
    }

    fn read_instance(
        &self,
        _data_values: &mut [T],
        _sample_infos: &mut [SampleInfo],
        _max_samples: i32,
        _a_handle: InstanceHandle,
        _sample_states: &[SampleStateKind],
        _view_states: &[ViewStateKind],
        _instance_states: &[InstanceStateKind],
    ) -> DDSResult<()> {
        todo!()
    }

    fn take_instance(
        &self,
        _data_values: &mut [T],
        _sample_infos: &mut [SampleInfo],
        _max_samples: i32,
        _a_handle: InstanceHandle,
        _sample_states: &[SampleStateKind],
        _view_states: &[ViewStateKind],
        _instance_states: &[InstanceStateKind],
    ) -> DDSResult<()> {
        todo!()
    }

    fn read_next_instance(
        &self,
        _data_values: &mut [T],
        _sample_infos: &mut [SampleInfo],
        _max_samples: i32,
        _previous_handle: InstanceHandle,
        _sample_states: &[SampleStateKind],
        _view_states: &[ViewStateKind],
        _instance_states: &[InstanceStateKind],
    ) -> DDSResult<()> {
        todo!()
    }

    fn take_next_instance(
        &self,
        _data_values: &mut [T],
        _sample_infos: &mut [SampleInfo],
        _max_samples: i32,
        _previous_handle: InstanceHandle,
        _sample_states: &[SampleStateKind],
        _view_states: &[ViewStateKind],
        _instance_states: &[InstanceStateKind],
    ) -> DDSResult<()> {
        todo!()
    }

    fn read_next_instance_w_condition(
        &self,
        _data_values: &mut [T],
        _sample_infos: &mut [SampleInfo],
        _max_samples: i32,
        _previous_handle: InstanceHandle,
        _a_condition: ReadCondition,
    ) -> DDSResult<()> {
        todo!()
    }

    fn take_next_instance_w_condition(
        &self,
        _data_values: &mut [T],
        _sample_infos: &mut [SampleInfo],
        _max_samples: i32,
        _previous_handle: InstanceHandle,
        _a_condition: ReadCondition,
    ) -> DDSResult<()> {
        todo!()
    }

    fn return_loan(
        &self,
        _data_values: &mut [T],
        _sample_infos: &mut [SampleInfo],
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_key_value(&self, _key_holder: &mut T, _handle: InstanceHandle) -> DDSResult<()> {
        todo!()
    }

    fn lookup_instance(&self, _instance: &T) -> InstanceHandle {
        todo!()
    }

    fn create_readcondition(
        &self,
        _sample_states: &[SampleStateKind],
        _view_states: &[ViewStateKind],
        _instance_states: &[InstanceStateKind],
    ) -> ReadCondition {
        todo!()
    }

    fn create_querycondition(
        &self,
        _sample_states: &[SampleStateKind],
        _view_states: &[ViewStateKind],
        _instance_states: &[InstanceStateKind],
        _query_expression: &'static str,
        _query_parameters: &[&'static str],
    ) -> QueryCondition {
        todo!()
    }

    fn delete_readcondition(&self, _a_condition: ReadCondition) -> DDSResult<()> {
        todo!()
    }

    fn get_liveliness_changed_status(
        &self,
        _status: &mut LivelinessChangedStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_requested_deadline_missed_status(
        &self,
        _status: &mut RequestedDeadlineMissedStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_requested_incompatible_qos_status(
        &self,
        _status: &mut RequestedIncompatibleQosStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_sample_lost_status(&self, _status: &mut SampleLostStatus) -> DDSResult<()> {
        todo!()
    }

    fn get_sample_rejected_status(&self, _status: &mut SampleRejectedStatus) -> DDSResult<()> {
        todo!()
    }

    fn get_subscription_matched_status(
        &self,
        _status: &mut SubscriptionMatchedStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    // fn get_topicdescription(&self) -> &dyn TopicDescription {
    //     todo!()
    // }

    fn delete_contained_entities(&self) -> DDSResult<()> {
        todo!()
    }

    fn wait_for_historical_data(&self) -> DDSResult<()> {
        todo!()
    }

    fn get_matched_publication_data(
        &self,
        _publication_data: &mut PublicationBuiltinTopicData,
        _publication_handle: InstanceHandle,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_match_publication(&self, _publication_handles: &mut [InstanceHandle]) -> DDSResult<()> {
        todo!()
    }

    fn get_topicdescription(
        &self,
    ) -> &dyn rust_dds_api::topic::topic_description::TopicDescription<T> {
        todo!()
    }

    fn get_subscriber(&self) -> &dyn rust_dds_api::subscription::subscriber::Subscriber {
        todo!()
    }
}

impl<'dr, T, Reader> Entity for DataReaderImpl<'dr, T, Reader> {
    type Qos = DataReaderQos;
    type Listener = &'static dyn DataReaderListener<DataPIM = T>;

    fn set_qos(&self, _qos: Option<Self::Qos>) -> DDSResult<()> {
        todo!()
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        todo!()
    }

    fn set_listener(
        &self,
        _a_listener: Option<Self::Listener>,
        _mask: StatusMask,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_listener(&self) -> DDSResult<Option<Self::Listener>> {
        todo!()
    }

    fn get_statuscondition(&self) -> StatusCondition {
        todo!()
    }

    fn get_status_changes(&self) -> StatusMask {
        todo!()
    }

    fn enable(&self) -> DDSResult<()> {
        todo!()
    }

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle> {
        todo!()
    }
}

impl<'dr, T, Reader> AnyDataReader for DataReaderImpl<'dr, T, Reader> {}

#[cfg(test)]
mod tests {
    use rust_dds_api::subscription::data_reader::DataReader;
    use std::marker::PhantomData;

    use rust_dds_api::{
        infrastructure::qos::{SubscriberQos, TopicQos},
        topic::topic_listener::TopicListener,
    };
    use rust_rtps_pim::structure::{RTPSCacheChange, RTPSHistoryCache};

    use crate::{
        dds_impl::data_reader_impl::DataReaderImpl,
        rtps_impl::rtps_history_cache_impl::RTPSHistoryCacheImpl, utils::shared_object::RtpsShared,
    };

    struct MockSubcriber;
    impl rust_dds_api::subscription::subscriber::Subscriber for MockSubcriber {
        fn begin_access(&self) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn end_access(&self) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn get_datareaders(
            &self,
            readers: &mut [&mut dyn rust_dds_api::subscription::data_reader::AnyDataReader],
            sample_states: &[rust_dds_api::dcps_psm::SampleStateKind],
            view_states: &[rust_dds_api::dcps_psm::ViewStateKind],
            instance_states: &[rust_dds_api::dcps_psm::InstanceStateKind],
        ) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn notify_datareaders(&self) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn get_participant(
            &self,
        ) -> &dyn rust_dds_api::domain::domain_participant::DomainParticipant {
            todo!()
        }

        fn get_sample_lost_status(
            &self,
            status: &mut rust_dds_api::dcps_psm::SampleLostStatus,
        ) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn delete_contained_entities(&self) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn set_default_datareader_qos(
            &self,
            qos: Option<rust_dds_api::infrastructure::qos::DataReaderQos>,
        ) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn get_default_datareader_qos(
            &self,
        ) -> rust_dds_api::return_type::DDSResult<rust_dds_api::infrastructure::qos::DataReaderQos>
        {
            todo!()
        }

        fn copy_from_topic_qos(
            &self,
            a_datareader_qos: &mut rust_dds_api::infrastructure::qos::DataReaderQos,
            a_topic_qos: &rust_dds_api::infrastructure::qos::TopicQos,
        ) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }
    }

    impl rust_dds_api::infrastructure::entity::Entity for MockSubcriber {
        type Qos = SubscriberQos;
        type Listener =
            &'static dyn rust_dds_api::subscription::subscriber_listener::SubscriberListener;

        fn set_qos(&self, qos: Option<Self::Qos>) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn get_qos(&self) -> rust_dds_api::return_type::DDSResult<Self::Qos> {
            todo!()
        }

        fn set_listener(
            &self,
            a_listener: Option<Self::Listener>,
            mask: rust_dds_api::dcps_psm::StatusMask,
        ) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn get_listener(&self) -> rust_dds_api::return_type::DDSResult<Option<Self::Listener>> {
            todo!()
        }

        fn get_statuscondition(&self) -> rust_dds_api::infrastructure::entity::StatusCondition {
            todo!()
        }

        fn get_status_changes(&self) -> rust_dds_api::dcps_psm::StatusMask {
            todo!()
        }

        fn enable(&self) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn get_instance_handle(
            &self,
        ) -> rust_dds_api::return_type::DDSResult<rust_dds_api::dcps_psm::InstanceHandle> {
            todo!()
        }
    }

    struct MockTopic<T>(PhantomData<T>);

    impl<T: 'static> rust_dds_api::topic::topic_description::TopicDescription<T> for MockTopic<T> {
        fn get_participant(
            &self,
        ) -> &dyn rust_dds_api::domain::domain_participant::DomainParticipant {
            todo!()
        }

        fn get_type_name(&self) -> rust_dds_api::return_type::DDSResult<&'static str> {
            todo!()
        }

        fn get_name(&self) -> rust_dds_api::return_type::DDSResult<&str> {
            todo!()
        }
    }

    impl<T: 'static> rust_dds_api::infrastructure::entity::Entity for MockTopic<T> {
        type Qos = TopicQos;
        type Listener = &'static dyn TopicListener<DataPIM = T>;

        fn set_qos(&self, qos: Option<Self::Qos>) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn get_qos(&self) -> rust_dds_api::return_type::DDSResult<Self::Qos> {
            todo!()
        }

        fn set_listener(
            &self,
            a_listener: Option<Self::Listener>,
            mask: rust_dds_api::dcps_psm::StatusMask,
        ) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn get_listener(&self) -> rust_dds_api::return_type::DDSResult<Option<Self::Listener>> {
            todo!()
        }

        fn get_statuscondition(&self) -> rust_dds_api::infrastructure::entity::StatusCondition {
            todo!()
        }

        fn get_status_changes(&self) -> rust_dds_api::dcps_psm::StatusMask {
            todo!()
        }

        fn enable(&self) -> rust_dds_api::return_type::DDSResult<()> {
            todo!()
        }

        fn get_instance_handle(
            &self,
        ) -> rust_dds_api::return_type::DDSResult<rust_dds_api::dcps_psm::InstanceHandle> {
            todo!()
        }
    }

    struct MockCacheChange(Vec<u8>);

    impl RTPSCacheChange for MockCacheChange {
        type InlineQosType = ();

        fn kind(&self) -> &rust_rtps_pim::structure::types::ChangeKind {
            todo!()
        }

        fn writer_guid(&self) -> &rust_rtps_pim::structure::types::GUID {
            todo!()
        }

        fn instance_handle(&self) -> &rust_rtps_pim::structure::types::InstanceHandle {
            todo!()
        }

        fn sequence_number(&self) -> &rust_rtps_pim::structure::types::SequenceNumber {
            todo!()
        }

        fn data_value(&self) -> &[u8] {
            &self.0
        }

        fn inline_qos(&self) -> &Self::InlineQosType {
            todo!()
        }
    }

    struct MockHistoryCache(MockCacheChange);

    impl RTPSHistoryCache for MockHistoryCache {
        type CacheChange = MockCacheChange;

        fn new() -> Self
        where
            Self: Sized,
        {
            todo!()
        }

        fn add_change(&mut self, change: Self::CacheChange) {
            todo!()
        }

        fn remove_change(&mut self, seq_num: &rust_rtps_pim::structure::types::SequenceNumber) {
            todo!()
        }

        fn get_change(
            &self,
            seq_num: &rust_rtps_pim::structure::types::SequenceNumber,
        ) -> Option<&Self::CacheChange> {
            Some(&self.0)
        }

        fn get_seq_num_min(&self) -> Option<rust_rtps_pim::structure::types::SequenceNumber> {
            todo!()
        }

        fn get_seq_num_max(&self) -> Option<rust_rtps_pim::structure::types::SequenceNumber> {
            todo!()
        }
    }

    struct MockRtpsReader(MockHistoryCache);

    impl rust_rtps_pim::behavior::reader::reader::RTPSReader for MockRtpsReader {
        type HistoryCacheType = MockHistoryCache;

        fn heartbeat_response_delay(&self) -> &rust_rtps_pim::behavior::types::Duration {
            todo!()
        }

        fn heartbeat_supression_duration(&self) -> &rust_rtps_pim::behavior::types::Duration {
            todo!()
        }

        fn reader_cache(&self) -> &Self::HistoryCacheType {
            &self.0
        }

        fn reader_cache_mut(&mut self) -> &mut Self::HistoryCacheType {
            todo!()
        }

        fn expects_inline_qos(&self) -> bool {
            todo!()
        }
    }

    #[test]
    fn read() {
        let reader = MockRtpsReader(MockHistoryCache(MockCacheChange(vec![1])));
        let shared_reader = RtpsShared::new(reader);

        let data_reader = DataReaderImpl::<u8, _> {
            _subscriber: &MockSubcriber,
            _topic: &MockTopic(PhantomData),
            reader: shared_reader.downgrade(),
        };

        let sample = data_reader.read(1, &[], &[], &[]).unwrap();
        assert_eq!(sample, 1);
    }
}
