use crate::dcps_psm::DURATION_ZERO;
use crate::implementation::rtps::types::{
    EntityId, Guid, GuidPrefix, ReliabilityKind, TopicKind, USER_DEFINED_WRITER_NO_KEY,
    USER_DEFINED_WRITER_WITH_KEY,
};
use crate::implementation::rtps::{group::RtpsGroupImpl, stateful_reader::RtpsStatefulReaderImpl};
use crate::return_type::{DdsError, DdsResult};
use crate::subscription::data_reader::AnyDataReader;
use crate::subscription::subscriber_listener::SubscriberListener;
use crate::{
    dds_type::DdsType,
    {
        dcps_psm::{
            InstanceHandle, InstanceStateMask, SampleLostStatus, SampleStateMask, StatusMask,
            ViewStateMask,
        },
        infrastructure::{
            entity::{Entity, StatusCondition},
            qos::{DataReaderQos, SubscriberQos, TopicQos},
            qos_policy::ReliabilityQosPolicyKind,
        },
    },
};
use dds_transport::messages::submessages::{DataSubmessage, HeartbeatSubmessage};

use crate::implementation::{
    data_representation_builtin_endpoints::{
        discovered_reader_data::{DiscoveredReaderData, RtpsReaderProxy},
        discovered_writer_data::DiscoveredWriterData,
    },
    utils::{
        discovery_traits::AddMatchedWriter,
        rtps_communication_traits::{
            ReceiveRtpsDataSubmessage, ReceiveRtpsHeartbeatSubmessage, SendRtpsMessage,
        },
        shared_object::{DdsRwLock, DdsShared, DdsWeak},
        timer::ThreadTimer,
    },
};

use super::{
    data_reader_impl::{DataReaderImpl, RtpsReader},
    domain_participant_impl::{DataReaderDiscovery, DomainParticipantImpl},
    topic_impl::TopicImpl,
};

use dds_transport::TransportWrite;

pub struct SubscriberImpl {
    qos: DdsRwLock<SubscriberQos>,
    rtps_group: RtpsGroupImpl,
    data_reader_list: DdsRwLock<Vec<DdsShared<DataReaderImpl<ThreadTimer>>>>,
    user_defined_data_reader_counter: u8,
    default_data_reader_qos: DataReaderQos,
    parent_domain_participant: DdsWeak<DomainParticipantImpl>,
    enabled: DdsRwLock<bool>,
}

impl SubscriberImpl {
    pub fn new(
        qos: SubscriberQos,
        rtps_group: RtpsGroupImpl,
        parent_domain_participant: DdsWeak<DomainParticipantImpl>,
    ) -> DdsShared<Self> {
        DdsShared::new(SubscriberImpl {
            qos: DdsRwLock::new(qos),
            rtps_group,
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant,
            enabled: DdsRwLock::new(false),
        })
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read_lock()
    }
}

pub trait SubscriberEmpty {
    fn is_empty(&self) -> bool;
}

impl SubscriberEmpty for DdsShared<SubscriberImpl> {
    fn is_empty(&self) -> bool {
        self.data_reader_list.read_lock().is_empty()
    }
}
pub trait AddDataReader {
    fn add_data_reader(&self, reader: DdsShared<DataReaderImpl<ThreadTimer>>);
}

impl AddDataReader for DdsShared<SubscriberImpl> {
    fn add_data_reader(&self, reader: DdsShared<DataReaderImpl<ThreadTimer>>) {
        self.data_reader_list.write_lock().push(reader);
    }
}

pub trait AnnounceDataReader {
    fn announce_datareader(&self, sedp_discovered_reader_data: DiscoveredReaderData);
}

impl AnnounceDataReader for DdsShared<SubscriberImpl> {
    fn announce_datareader(&self, sedp_discovered_reader_data: DiscoveredReaderData) {
        if let Ok(domain_participant) = self.parent_domain_participant.upgrade() {
            domain_participant.add_created_data_reader(&DiscoveredReaderData {
                reader_proxy: RtpsReaderProxy {
                    unicast_locator_list: domain_participant
                        .default_unicast_locator_list()
                        .to_vec(),
                    multicast_locator_list: domain_participant
                        .default_multicast_locator_list()
                        .to_vec(),
                    ..sedp_discovered_reader_data.reader_proxy
                },
                ..sedp_discovered_reader_data
            });
        }
    }
}

impl DdsShared<SubscriberImpl> {
    pub fn create_datareader<Foo>(
        &self,
        a_topic: &DdsShared<TopicImpl>,
        qos: Option<DataReaderQos>,
        a_listener: Option<<DdsShared<DataReaderImpl<ThreadTimer>> as Entity>::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<DdsShared<DataReaderImpl<ThreadTimer>>>
    where
        Foo: DdsType,
    {
        // /////// Build the GUID
        let entity_id = {
            let entity_kind = match Foo::has_key() {
                true => USER_DEFINED_WRITER_WITH_KEY,
                false => USER_DEFINED_WRITER_NO_KEY,
            };

            EntityId::new(
                [
                    self.rtps_group.guid().entity_id().entity_key()[0],
                    self.user_defined_data_reader_counter,
                    0,
                ],
                entity_kind,
            )
        };

        let guid = Guid::new(self.rtps_group.guid().prefix(), entity_id);

        // /////// Create data reader
        let data_reader_shared = {
            let qos = qos.unwrap_or_else(|| self.default_data_reader_qos.clone());
            qos.is_consistent()?;

            let topic_kind = match Foo::has_key() {
                true => TopicKind::WithKey,
                false => TopicKind::NoKey,
            };

            let reliability_level = match qos.reliability.kind {
                ReliabilityQosPolicyKind::BestEffortReliabilityQos => ReliabilityKind::BestEffort,
                ReliabilityQosPolicyKind::ReliableReliabilityQos => ReliabilityKind::Reliable,
            };

            let domain_participant = self.parent_domain_participant.upgrade().ok();
            let rtps_reader = RtpsReader::Stateful(RtpsStatefulReaderImpl::new(
                guid,
                topic_kind,
                reliability_level,
                domain_participant
                    .as_ref()
                    .map(|dp| dp.default_unicast_locator_list())
                    .unwrap_or(&[]),
                domain_participant
                    .as_ref()
                    .map(|dp| dp.default_multicast_locator_list())
                    .unwrap_or(&[]),
                DURATION_ZERO,
                DURATION_ZERO,
                false,
            ));

            let data_reader_shared = DataReaderImpl::new(
                qos,
                rtps_reader,
                a_topic.clone(),
                a_listener,
                self.downgrade(),
            );

            self.data_reader_list
                .write_lock()
                .push(data_reader_shared.clone());

            data_reader_shared
        };

        if *self.enabled.read_lock()
            && self
                .qos
                .read_lock()
                .entity_factory
                .autoenable_created_entities
        {
            data_reader_shared.enable()?;
        }

        Ok(data_reader_shared)
    }

    pub fn delete_datareader(
        &self,
        a_datareader: &DdsShared<DataReaderImpl<ThreadTimer>>,
    ) -> DdsResult<()> {
        let data_reader_list = &mut self.data_reader_list.write_lock();
        let data_reader_list_position = data_reader_list
            .iter()
            .position(|x| x == a_datareader)
            .ok_or_else(|| {
                DdsError::PreconditionNotMet(
                    "Data reader can only be deleted from its parent subscriber".to_string(),
                )
            })?;
        data_reader_list.remove(data_reader_list_position);

        Ok(())
    }

    pub fn lookup_datareader<Foo>(
        &self,
        topic: &DdsShared<TopicImpl>,
    ) -> DdsResult<DdsShared<DataReaderImpl<ThreadTimer>>>
    where
        Foo: DdsType,
    {
        let data_reader_list = &self.data_reader_list.write_lock();

        data_reader_list
            .iter()
            .find_map(|data_reader_shared| {
                let data_reader_topic = data_reader_shared.get_topicdescription().unwrap();

                if data_reader_topic.get_name().ok()? == topic.get_name().ok()?
                    && data_reader_topic.get_type_name().ok()? == Foo::type_name()
                {
                    Some(data_reader_shared.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| DdsError::PreconditionNotMet("Not found".to_string()))
    }

    pub fn begin_access(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn end_access(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn get_datareaders(
        &self,
        _readers: &mut [&mut dyn AnyDataReader],
        _sample_states: SampleStateMask,
        _view_states: ViewStateMask,
        _instance_states: InstanceStateMask,
    ) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn notify_datareaders(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn get_participant(&self) -> DdsResult<DdsShared<DomainParticipantImpl>> {
        Ok(self
            .parent_domain_participant
            .upgrade()
            .expect("Failed to get parent participant of subscriber"))
    }

    pub fn get_sample_lost_status(&self, _status: &mut SampleLostStatus) -> DdsResult<()> {
        todo!()
    }

    pub fn delete_contained_entities(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn set_default_datareader_qos(&self, _qos: Option<DataReaderQos>) -> DdsResult<()> {
        todo!()
    }

    pub fn get_default_datareader_qos(&self) -> DdsResult<DataReaderQos> {
        todo!()
    }

    pub fn copy_from_topic_qos(
        &self,
        _a_datareader_qos: &mut DataReaderQos,
        _a_topic_qos: &TopicQos,
    ) -> DdsResult<()> {
        todo!()
    }
}

impl Entity for DdsShared<SubscriberImpl> {
    type Qos = SubscriberQos;
    type Listener = Box<dyn SubscriberListener>;

    fn set_qos(&self, qos: Option<Self::Qos>) -> DdsResult<()> {
        let qos = qos.unwrap_or_default();

        if *self.enabled.read_lock() {
            self.qos.read_lock().check_immutability(&qos)?;
        }

        *self.qos.write_lock() = qos;

        Ok(())
    }

    fn get_qos(&self) -> DdsResult<Self::Qos> {
        Ok(self.qos.read_lock().clone())
    }

    fn set_listener(
        &self,
        _a_listener: Option<Self::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<()> {
        todo!()
    }

    fn get_listener(&self) -> DdsResult<Option<Self::Listener>> {
        todo!()
    }

    fn get_statuscondition(&self) -> DdsResult<StatusCondition> {
        todo!()
    }

    fn get_status_changes(&self) -> DdsResult<StatusMask> {
        todo!()
    }

    fn enable(&self) -> DdsResult<()> {
        if !self.parent_domain_participant.upgrade()?.is_enabled() {
            return Err(DdsError::PreconditionNotMet(
                "Parent participant is disabled".to_string(),
            ));
        }

        *self.enabled.write_lock() = true;

        if self
            .qos
            .read_lock()
            .entity_factory
            .autoenable_created_entities
        {
            for data_reader in self.data_reader_list.read_lock().iter() {
                data_reader.enable()?;
            }
        }

        Ok(())
    }

    fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        Ok(self.rtps_group.guid().into())
    }
}

impl AddMatchedWriter for DdsShared<SubscriberImpl> {
    fn add_matched_writer(&self, discovered_writer_data: &DiscoveredWriterData) {
        for data_reader in self.data_reader_list.read_lock().iter() {
            data_reader.add_matched_writer(discovered_writer_data)
        }
    }
}

impl ReceiveRtpsDataSubmessage for DdsShared<SubscriberImpl> {
    fn on_data_submessage_received(
        &self,
        data_submessage: &DataSubmessage<'_>,
        source_guid_prefix: GuidPrefix,
    ) {
        for data_reader in self.data_reader_list.read_lock().iter() {
            data_reader.on_data_submessage_received(data_submessage, source_guid_prefix)
        }
    }
}

impl ReceiveRtpsHeartbeatSubmessage for DdsShared<SubscriberImpl> {
    fn on_heartbeat_submessage_received(
        &self,
        heartbeat_submessage: &HeartbeatSubmessage,
        source_guid_prefix: GuidPrefix,
    ) {
        for data_reader in self.data_reader_list.read_lock().iter() {
            data_reader.on_heartbeat_submessage_received(heartbeat_submessage, source_guid_prefix)
        }
    }
}

impl SendRtpsMessage for DdsShared<SubscriberImpl> {
    fn send_message(&self, transport: &mut impl TransportWrite) {
        for data_reader in self.data_reader_list.read_lock().iter() {
            data_reader.send_message(transport);
        }
    }
}
