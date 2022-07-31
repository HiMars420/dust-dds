use std::sync::atomic::{self, AtomicU8};

use crate::dcps_psm::DURATION_ZERO;
use crate::dds_type::DdsType;
use crate::implementation::rtps::types::{
    EntityId, Guid, GuidPrefix, ReliabilityKind, TopicKind, USER_DEFINED_WRITER_NO_KEY,
    USER_DEFINED_WRITER_WITH_KEY,
};
use crate::implementation::rtps::{group::RtpsGroupImpl, stateful_writer::RtpsStatefulWriterImpl};
use crate::infrastructure::entity::StatusCondition;
use crate::return_type::{DdsError, DdsResult};
use crate::{
    publication::publisher_listener::PublisherListener,
    {
        dcps_psm::{Duration, InstanceHandle, StatusMask},
        infrastructure::{
            qos::{DataWriterQos, PublisherQos, TopicQos},
            qos_policy::ReliabilityQosPolicyKind,
        },
    },
};
use dds_transport::messages::submessages::AckNackSubmessage;

use crate::implementation::{
    data_representation_builtin_endpoints::discovered_reader_data::DiscoveredReaderData,
    utils::{
        discovery_traits::AddMatchedReader,
        rtps_communication_traits::{ReceiveRtpsAckNackSubmessage, SendRtpsMessage},
        shared_object::{DdsRwLock, DdsShared},
    },
};

use super::data_writer_impl::AnyDataWriterListener;
use super::{
    data_writer_impl::{DataWriterImpl, RtpsWriter},
    domain_participant_impl::DomainParticipantImpl,
    topic_impl::TopicImpl,
};

use dds_transport::TransportWrite;

pub struct PublisherImpl {
    qos: DdsRwLock<PublisherQos>,
    rtps_group: RtpsGroupImpl,
    data_writer_list: DdsRwLock<Vec<DdsShared<DataWriterImpl>>>,
    user_defined_data_writer_counter: AtomicU8,
    default_datawriter_qos: DataWriterQos,
    enabled: DdsRwLock<bool>,
}

impl PublisherImpl {
    pub fn new(qos: PublisherQos, rtps_group: RtpsGroupImpl) -> DdsShared<Self> {
        DdsShared::new(PublisherImpl {
            qos: DdsRwLock::new(qos),
            rtps_group,
            data_writer_list: DdsRwLock::new(Vec::new()),
            user_defined_data_writer_counter: AtomicU8::new(0),
            default_datawriter_qos: DataWriterQos::default(),
            enabled: DdsRwLock::new(false),
        })
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read_lock()
    }
}

pub trait PublisherEmpty {
    fn is_empty(&self) -> bool;
}

impl PublisherEmpty for DdsShared<PublisherImpl> {
    fn is_empty(&self) -> bool {
        self.data_writer_list.read_lock().is_empty()
    }
}

pub trait AddDataWriter {
    fn add_data_writer(&self, writer: DdsShared<DataWriterImpl>);
}

impl AddDataWriter for DdsShared<PublisherImpl> {
    fn add_data_writer(&self, writer: DdsShared<DataWriterImpl>) {
        self.data_writer_list.write_lock().push(writer);
    }
}

impl DdsShared<PublisherImpl> {
    pub fn create_datawriter<Foo>(
        &self,
        a_topic: &DdsShared<TopicImpl>,
        qos: Option<DataWriterQos>,
        a_listener: Option<Box<dyn AnyDataWriterListener + Send + Sync>>,
        _mask: StatusMask,
        parent_participant: &DdsShared<DomainParticipantImpl>,
    ) -> DdsResult<DdsShared<DataWriterImpl>>
    where
        Foo: DdsType,
    {
        let topic_shared = a_topic;

        // /////// Build the GUID
        let guid = {
            let user_defined_data_writer_counter = self
                .user_defined_data_writer_counter
                .fetch_add(1, atomic::Ordering::SeqCst);

            let entity_kind = match Foo::has_key() {
                true => USER_DEFINED_WRITER_WITH_KEY,
                false => USER_DEFINED_WRITER_NO_KEY,
            };

            Guid::new(
                self.rtps_group.guid().prefix(),
                EntityId::new(
                    [
                        self.rtps_group.guid().entity_id().entity_key()[0],
                        user_defined_data_writer_counter,
                        0,
                    ],
                    entity_kind,
                ),
            )
        };

        // /////// Create data writer
        let data_writer_shared = {
            let qos = qos.unwrap_or_else(|| self.default_datawriter_qos.clone());
            qos.is_consistent()?;

            let topic_kind = match Foo::has_key() {
                true => TopicKind::WithKey,
                false => TopicKind::NoKey,
            };

            let reliability_level = match qos.reliability.kind {
                ReliabilityQosPolicyKind::BestEffortReliabilityQos => ReliabilityKind::BestEffort,
                ReliabilityQosPolicyKind::ReliableReliabilityQos => ReliabilityKind::Reliable,
            };

            let rtps_writer_impl = RtpsWriter::Stateful(RtpsStatefulWriterImpl::new(
                guid,
                topic_kind,
                reliability_level,
                parent_participant.default_unicast_locator_list(),
                parent_participant.default_multicast_locator_list(),
                true,
                Duration::new(0, 200_000_000),
                DURATION_ZERO,
                DURATION_ZERO,
                None,
            ));

            let data_writer_shared = DataWriterImpl::new(
                qos,
                rtps_writer_impl,
                a_listener,
                topic_shared.clone(),
                self.downgrade(),
            );

            self.data_writer_list
                .write_lock()
                .push(data_writer_shared.clone());

            data_writer_shared
        };

        if *self.enabled.read_lock()
            && self
                .qos
                .read_lock()
                .entity_factory
                .autoenable_created_entities
        {
            data_writer_shared.enable(parent_participant)?;
        }

        Ok(data_writer_shared)
    }

    pub fn delete_datawriter(&self, a_datawriter: &DdsShared<DataWriterImpl>) -> DdsResult<()> {
        let data_writer_list = &mut self.data_writer_list.write_lock();
        let data_writer_list_position = data_writer_list
            .iter()
            .position(|x| x == a_datawriter)
            .ok_or_else(|| {
                DdsError::PreconditionNotMet(
                    "Data writer can only be deleted from its parent publisher".to_string(),
                )
            })?;
        data_writer_list.remove(data_writer_list_position);

        Ok(())
    }

    pub fn lookup_datawriter<Foo>(
        &self,
        topic: &DdsShared<TopicImpl>,
    ) -> DdsResult<DdsShared<DataWriterImpl>>
    where
        Foo: DdsType,
    {
        let data_writer_list = &self.data_writer_list.write_lock();

        data_writer_list
            .iter()
            .find_map(|data_writer_shared| {
                let data_writer_topic = data_writer_shared.get_topic().ok()?;

                if data_writer_topic.get_name().ok()? == topic.get_name().ok()?
                    && data_writer_topic.get_type_name().ok()? == Foo::type_name()
                {
                    Some(data_writer_shared.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| DdsError::PreconditionNotMet("Not found".to_string()))
    }

    pub fn suspend_publications(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn resume_publications(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn begin_coherent_changes(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn end_coherent_changes(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn wait_for_acknowledgments(&self, _max_wait: Duration) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn delete_contained_entities(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn set_default_datawriter_qos(&self, _qos: Option<DataWriterQos>) -> DdsResult<()> {
        todo!()
    }

    pub fn get_default_datawriter_qos(&self) -> DdsResult<DataWriterQos> {
        todo!()
    }

    pub fn copy_from_topic_qos(
        &self,
        _a_datawriter_qos: &mut DataWriterQos,
        _a_topic_qos: &TopicQos,
    ) -> DdsResult<()> {
        todo!()
    }
}

impl DdsShared<PublisherImpl> {
    pub fn set_qos(&self, qos: Option<PublisherQos>) -> DdsResult<()> {
        let qos = qos.unwrap_or_default();

        if *self.enabled.read_lock() {
            self.qos.read_lock().check_immutability(&qos)?;
        }

        *self.qos.write_lock() = qos;

        Ok(())
    }

    pub fn get_qos(&self) -> DdsResult<PublisherQos> {
        Ok(self.qos.read_lock().clone())
    }

    pub fn set_listener(
        &self,
        _a_listener: Option<Box<dyn PublisherListener>>,
        _mask: StatusMask,
    ) -> DdsResult<()> {
        todo!()
    }

    pub fn get_listener(&self) -> DdsResult<Option<Box<dyn PublisherListener>>> {
        todo!()
    }

    pub fn get_statuscondition(&self) -> DdsResult<StatusCondition> {
        todo!()
    }

    pub fn get_status_changes(&self) -> DdsResult<StatusMask> {
        todo!()
    }

    pub fn enable(&self, parent_participant: &DdsShared<DomainParticipantImpl>) -> DdsResult<()> {
        if !parent_participant.is_enabled() {
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
            for data_writer in self.data_writer_list.read_lock().iter() {
                data_writer.enable(parent_participant)?;
            }
        }

        Ok(())
    }

    pub fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        Ok(self.rtps_group.guid().into())
    }
}

impl AddMatchedReader for DdsShared<PublisherImpl> {
    fn add_matched_reader(&self, discovered_reader_data: &DiscoveredReaderData) {
        for data_writer in self.data_writer_list.read_lock().iter() {
            data_writer.add_matched_reader(discovered_reader_data)
        }
    }
}

impl ReceiveRtpsAckNackSubmessage for DdsShared<PublisherImpl> {
    fn on_acknack_submessage_received(
        &self,
        acknack_submessage: &AckNackSubmessage,
        source_guid_prefix: GuidPrefix,
    ) {
        for data_writer in self.data_writer_list.read_lock().iter() {
            data_writer.on_acknack_submessage_received(acknack_submessage, source_guid_prefix);
        }
    }
}

impl SendRtpsMessage for DdsShared<PublisherImpl> {
    fn send_message(&self, transport: &mut impl TransportWrite) {
        for data_writer in self.data_writer_list.read_lock().iter() {
            data_writer.send_message(transport);
        }
    }
}
