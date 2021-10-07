use std::{
    any::Any,
    sync::{Arc, Mutex, RwLock},
};

use rust_dds_api::{
    dcps_psm::StatusMask,
    infrastructure::{
        entity::Entity,
        qos::{DataReaderQos, SubscriberQos},
        qos_policy::ReliabilityQosPolicyKind,
    },
    return_type::DDSResult,
    subscription::{
        data_reader_listener::DataReaderListener,
        subscriber::{DataReaderGAT, Subscriber},
        subscriber_listener::SubscriberListener,
    },
};
use rust_rtps_pim::{
    behavior::reader::{reader::RtpsReader, stateless_reader::RtpsStatelessReader},
    messages::{submessage_elements::Parameter, submessages::DataSubmessage},
    structure::{
        types::{
            EntityId, Guid, GuidPrefix, ReliabilityKind, TopicKind, USER_DEFINED_WRITER_NO_KEY,
            USER_DEFINED_WRITER_WITH_KEY,
        },
        RtpsEndpoint, RtpsEntity, RtpsGroup, RtpsHistoryCache,
    },
};

use crate::{
    dds_type::DdsType,
    rtps_impl::rtps_reader_history_cache_impl::ReaderHistoryCache,
    utils::{
        message_receiver::{ImmutableProcessDataSubmessage, MutableProcessDataSubmessage},
        shared_object::{
            rtps_shared_downgrade, rtps_shared_new, rtps_shared_write_lock, RtpsShared, RtpsWeak,
        },
    },
};

use super::data_reader_impl::{DataReaderImpl, RtpsReaderFlavor};

pub trait DataReaderObject: Any + Send + Sync + ImmutableProcessDataSubmessage {
    fn as_any(&self) -> &dyn Any;
}

impl<T> DataReaderObject for T
where
    T: Any + Send + Sync + ImmutableProcessDataSubmessage,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T> ImmutableProcessDataSubmessage for RwLock<T>
where
    T: MutableProcessDataSubmessage,
{
    fn process_data_submessage(
        &self,
        source_guid_prefix: GuidPrefix,
        data: &DataSubmessage<Vec<Parameter<'_>>>,
    ) {
        self.write()
            .unwrap()
            .process_data_submessage(source_guid_prefix, data)
    }
}

pub struct SubscriberImpl {
    qos: SubscriberQos,
    rtps_group: RtpsGroup,
    data_reader_list: Mutex<Vec<Arc<dyn DataReaderObject>>>,
    user_defined_data_reader_counter: u8,
    default_data_reader_qos: DataReaderQos,
}

impl SubscriberImpl {
    pub fn new(
        qos: SubscriberQos,
        rtps_group: RtpsGroup,
        data_reader_list: Vec<Arc<dyn DataReaderObject>>,
    ) -> Self {
        Self {
            qos,
            rtps_group,
            data_reader_list: Mutex::new(data_reader_list),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
        }
    }
}

impl<T> DataReaderGAT<'_, '_, T> for SubscriberImpl
where
    T: DdsType + 'static,
{
    type TopicType = ();
    type DataReaderType = RtpsWeak<DataReaderImpl<T>>;

    fn create_datareader_gat(
        &'_ self,
        _a_topic: &'_ Self::TopicType,
        qos: Option<DataReaderQos>,
        _a_listener: Option<&'static dyn DataReaderListener<DataType = T>>,
        _mask: StatusMask,
    ) -> Option<Self::DataReaderType> {
        let qos = qos.unwrap_or(self.default_data_reader_qos.clone());
        qos.is_consistent().ok()?;

        let (entity_kind, topic_kind) = match T::has_key() {
            true => (USER_DEFINED_WRITER_WITH_KEY, TopicKind::WithKey),
            false => (USER_DEFINED_WRITER_NO_KEY, TopicKind::NoKey),
        };
        let entity_id = EntityId::new(
            [
                self.rtps_group.entity.guid.entity_id().entity_key()[0],
                self.user_defined_data_reader_counter,
                0,
            ],
            entity_kind,
        );
        let guid = Guid::new(*self.rtps_group.entity.guid.prefix(), entity_id);
        let reliability_level = match qos.reliability.kind {
            ReliabilityQosPolicyKind::BestEffortReliabilityQos => ReliabilityKind::BestEffort,
            ReliabilityQosPolicyKind::ReliableReliabilityQos => ReliabilityKind::Reliable,
        };

        let unicast_locator_list = vec![];
        let multicast_locator_list = vec![];
        let heartbeat_response_delay = rust_rtps_pim::behavior::types::DURATION_ZERO;
        let heartbeat_supression_duration = rust_rtps_pim::behavior::types::DURATION_ZERO;
        let expects_inline_qos = false;
        let rtps_reader = RtpsReaderFlavor::Stateless(RtpsStatelessReader(RtpsReader {
            endpoint: RtpsEndpoint {
                entity: RtpsEntity { guid },
                topic_kind,
                reliability_level,
                unicast_locator_list,
                multicast_locator_list,
            },
            heartbeat_response_delay,
            heartbeat_supression_duration,
            reader_cache: ReaderHistoryCache::new(),
            expects_inline_qos,
        }));
        let reader_storage = DataReaderImpl::new(qos, rtps_reader);
        let reader_storage_shared = rtps_shared_new(reader_storage);
        let reader_storage_weak = rtps_shared_downgrade(&reader_storage_shared);
        self.data_reader_list
            .lock()
            .unwrap()
            .push(reader_storage_shared);
        Some(reader_storage_weak)
    }

    fn delete_datareader_gat(&self, _a_datareader: &Self::DataReaderType) -> DDSResult<()> {
        todo!()
    }

    fn lookup_datareader_gat(
        &'_ self,
        _topic: &'_ Self::TopicType,
    ) -> Option<Self::DataReaderType> {
        let guard = self.data_reader_list.lock().unwrap();
        Some(Arc::downgrade(guard[0].as_any().downcast_ref().unwrap()))
    }
}

impl Subscriber for SubscriberImpl {
    fn begin_access(&self) -> DDSResult<()> {
        todo!()
    }

    fn end_access(&self) -> DDSResult<()> {
        todo!()
    }

    fn get_datareaders(
        &self,
        _readers: &mut [&mut dyn rust_dds_api::subscription::data_reader::AnyDataReader],
        _sample_states: &[rust_dds_api::dcps_psm::SampleStateKind],
        _view_states: &[rust_dds_api::dcps_psm::ViewStateKind],
        _instance_states: &[rust_dds_api::dcps_psm::InstanceStateKind],
    ) -> DDSResult<()> {
        todo!()
    }

    fn notify_datareaders(&self) -> DDSResult<()> {
        todo!()
    }

    fn get_participant(&self) -> &dyn rust_dds_api::domain::domain_participant::DomainParticipant {
        todo!()
    }

    fn get_sample_lost_status(
        &self,
        _status: &mut rust_dds_api::dcps_psm::SampleLostStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn delete_contained_entities(&self) -> DDSResult<()> {
        todo!()
    }

    fn set_default_datareader_qos(&self, _qos: Option<DataReaderQos>) -> DDSResult<()> {
        todo!()
    }

    fn get_default_datareader_qos(&self) -> DDSResult<DataReaderQos> {
        todo!()
    }

    fn copy_from_topic_qos(
        &self,
        _a_datareader_qos: &mut DataReaderQos,
        _a_topic_qos: &rust_dds_api::infrastructure::qos::TopicQos,
    ) -> DDSResult<()> {
        todo!()
    }
}

impl Entity for SubscriberImpl {
    type Qos = SubscriberQos;
    type Listener = &'static dyn SubscriberListener;

    fn set_qos(&mut self, qos: Option<Self::Qos>) -> DDSResult<()> {
        let qos = qos.unwrap_or_default();
        self.qos = qos;
        Ok(())
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        // &self.qos
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

    fn get_statuscondition(
        &self,
    ) -> DDSResult<rust_dds_api::infrastructure::entity::StatusCondition> {
        todo!()
    }

    fn get_status_changes(&self) -> DDSResult<StatusMask> {
        todo!()
    }

    fn enable(&self) -> DDSResult<()> {
        todo!()
    }

    fn get_instance_handle(&self) -> DDSResult<rust_dds_api::dcps_psm::InstanceHandle> {
        todo!()
    }
}

impl ImmutableProcessDataSubmessage for SubscriberImpl {
    fn process_data_submessage(
        &self,
        source_guid_prefix: GuidPrefix,
        data: &DataSubmessage<Vec<Parameter<'_>>>,
    ) {
        let data_reader_list = self.data_reader_list.lock().unwrap();
        for reader in data_reader_list.iter() {
            reader.process_data_submessage(source_guid_prefix, data);
            //  rtps_reader_mut() {
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn lookup_datareader() {
        // todo!()
    }
}
