use std::sync::atomic::{self, AtomicU8};

use rust_dds_api::{
    builtin_topics::PublicationBuiltinTopicData,
    dcps_psm::{BuiltInTopicKey, Duration, InstanceHandle, StatusMask},
    domain::domain_participant::DomainParticipantTopicFactory,
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::{DataWriterQos, PublisherQos, TopicQos},
        qos_policy::{
            DeadlineQosPolicy, DestinationOrderQosPolicy, DurabilityQosPolicy,
            DurabilityServiceQosPolicy, GroupDataQosPolicy, LatencyBudgetQosPolicy,
            LifespanQosPolicy, LivelinessQosPolicy, OwnershipQosPolicy, OwnershipStrengthQosPolicy,
            PartitionQosPolicy, PresentationQosPolicy, ReliabilityQosPolicy,
            ReliabilityQosPolicyKind, TopicDataQosPolicy, UserDataQosPolicy,
        },
    },
    publication::{
        data_writer::DataWriter,
        data_writer_listener::DataWriterListener,
        publisher::{Publisher, PublisherDataWriterFactory},
        publisher_listener::PublisherListener,
    },
    return_type::{DDSError, DDSResult},
};

use rust_rtps_pim::{
    behavior::writer::{
        stateful_writer::RtpsStatefulWriterConstructor,
        writer::{RtpsWriterAttributes, RtpsWriterOperations},
    },
    structure::{
        entity::RtpsEntityAttributes,
        history_cache::RtpsHistoryCacheOperations,
        types::{
            EntityId, Guid, ReliabilityKind, TopicKind, USER_DEFINED_WRITER_NO_KEY,
            USER_DEFINED_WRITER_WITH_KEY,
        },
    },
};

use crate::{
    data_representation_builtin_endpoints::sedp_discovered_writer_data::{
        RtpsWriterProxy, SedpDiscoveredWriterData, DCPS_PUBLICATION,
    },
    dds_type::{DdsSerialize, DdsType},
    utils::{
        rtps_structure::RtpsStructure,
        shared_object::{RtpsShared, RtpsWeak},
    },
};

use super::{
    data_writer_proxy::DataWriterAttributes,
    data_writer_proxy::{DataWriterProxy, RtpsWriter},
    domain_participant_proxy::{DomainParticipantAttributes, DomainParticipantProxy},
    topic_proxy::TopicProxy,
};

pub struct PublisherAttributes<Rtps>
where
    Rtps: RtpsStructure,
{
    pub _qos: PublisherQos,
    pub rtps_group: Rtps::Group,
    pub data_writer_list: Vec<RtpsShared<DataWriterAttributes<Rtps>>>,
    pub user_defined_data_writer_counter: AtomicU8,
    pub default_datawriter_qos: DataWriterQos,
    pub parent_participant: RtpsWeak<DomainParticipantAttributes<Rtps>>,
}

impl<Rtps> PublisherAttributes<Rtps>
where
    Rtps: RtpsStructure,
{
    pub fn new(
        qos: PublisherQos,
        rtps_group: Rtps::Group,
        parent_participant: RtpsWeak<DomainParticipantAttributes<Rtps>>,
    ) -> Self {
        Self {
            _qos: qos,
            rtps_group,
            data_writer_list: Vec::new(),
            user_defined_data_writer_counter: AtomicU8::new(0),
            default_datawriter_qos: DataWriterQos::default(),
            parent_participant,
        }
    }
}

#[derive(Clone)]
pub struct PublisherProxy<Rtps>(pub(crate) RtpsWeak<PublisherAttributes<Rtps>>)
where
    Rtps: RtpsStructure;

impl<Rtps> PublisherProxy<Rtps>
where
    Rtps: RtpsStructure,
{
    pub fn new(publisher_impl: RtpsWeak<PublisherAttributes<Rtps>>) -> Self {
        Self(publisher_impl)
    }
}

impl<Rtps> AsRef<RtpsWeak<PublisherAttributes<Rtps>>> for PublisherProxy<Rtps>
where
    Rtps: RtpsStructure,
{
    fn as_ref(&self) -> &RtpsWeak<PublisherAttributes<Rtps>> {
        &self.0
    }
}

impl<Foo, Rtps> PublisherDataWriterFactory<Foo> for PublisherProxy<Rtps>
where
    Foo: DdsType + DdsSerialize + Send + Sync + 'static,
    Rtps: RtpsStructure,
    Rtps::Group: RtpsEntityAttributes,
    Rtps::StatelessWriter: RtpsWriterOperations<DataType = Vec<u8>, ParameterListType = Vec<u8>>
        + RtpsWriterAttributes,
    Rtps::StatefulWriter: RtpsWriterOperations<DataType = Vec<u8>, ParameterListType = Vec<u8>>
        + RtpsWriterAttributes
        + RtpsStatefulWriterConstructor,
    <Rtps::StatelessWriter as RtpsWriterAttributes>::WriterHistoryCacheType:
        RtpsHistoryCacheOperations<
            CacheChangeType = <Rtps::StatelessWriter as RtpsWriterOperations>::CacheChangeType,
        >,
    <Rtps::StatefulWriter as RtpsWriterAttributes>::WriterHistoryCacheType:
        RtpsHistoryCacheOperations<
            CacheChangeType = <Rtps::StatefulWriter as RtpsWriterOperations>::CacheChangeType,
        >,
{
    type TopicType = TopicProxy<Foo, Rtps>;
    type DataWriterType = DataWriterProxy<Foo, Rtps>;

    fn datawriter_factory_create_datawriter(
        &self,
        topic: &Self::TopicType,
        qos: Option<DataWriterQos>,
        _a_listener: Option<&'static dyn DataWriterListener>,
        _mask: StatusMask,
    ) -> Option<Self::DataWriterType> {
        let publisher_shared = self.0.upgrade().ok()?;

        let topic_shared = topic.as_ref().upgrade().ok()?;

        let qos = qos.unwrap_or(publisher_shared.read_lock().default_datawriter_qos.clone());
        let user_defined_data_writer_counter = publisher_shared
            .write_lock()
            .user_defined_data_writer_counter
            .fetch_add(1, atomic::Ordering::SeqCst);
        let (entity_kind, topic_kind) = match Foo::has_key() {
            true => (USER_DEFINED_WRITER_WITH_KEY, TopicKind::WithKey),
            false => (USER_DEFINED_WRITER_NO_KEY, TopicKind::NoKey),
        };
        let entity_id = EntityId::new(
            [
                publisher_shared
                    .read_lock()
                    .rtps_group
                    .guid()
                    .entity_id()
                    .entity_key()[0],
                user_defined_data_writer_counter,
                0,
            ],
            entity_kind,
        );
        let guid = Guid::new(
            *publisher_shared.read_lock().rtps_group.guid().prefix(),
            entity_id,
        );
        let reliability_level = match qos.reliability.kind {
            ReliabilityQosPolicyKind::BestEffortReliabilityQos => ReliabilityKind::BestEffort,
            ReliabilityQosPolicyKind::ReliableReliabilityQos => ReliabilityKind::Reliable,
        };
        let unicast_locator_list = &[];
        let multicast_locator_list = &[];
        let push_mode = true;
        let heartbeat_period = rust_rtps_pim::behavior::types::Duration::new(0, 200_000_000);
        let nack_response_delay = rust_rtps_pim::behavior::types::DURATION_ZERO;
        let nack_suppression_duration = rust_rtps_pim::behavior::types::DURATION_ZERO;
        let data_max_size_serialized = None;
        let rtps_writer_impl = RtpsWriter::Stateful(Rtps::StatefulWriter::new(
            guid,
            topic_kind,
            reliability_level,
            unicast_locator_list,
            multicast_locator_list,
            push_mode,
            heartbeat_period,
            nack_response_delay,
            nack_suppression_duration,
            data_max_size_serialized,
        ));

        let domain_participant = publisher_shared
            .read_lock()
            .parent_participant
            .upgrade()
            .ok()?;
        let domain_participant_proxy = DomainParticipantProxy::new(domain_participant.downgrade());
        let builtin_publisher = domain_participant.read_lock().builtin_publisher.clone()?;
        let builtin_publisher_proxy = PublisherProxy::new(builtin_publisher.downgrade());

        let publication_topic = domain_participant_proxy
            .topic_factory_find_topic(DCPS_PUBLICATION, Duration::new(0, 0))?;

        let mut sedp_builtin_publications_announcer =
            builtin_publisher_proxy.datawriter_factory_lookup_datawriter(&publication_topic)?;

        let sedp_discovered_writer_data = SedpDiscoveredWriterData {
            writer_proxy: RtpsWriterProxy {
                remote_writer_guid: guid,
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
                data_max_size_serialized: None,
                remote_group_entity_id: EntityId::new([0; 3], 0),
            },

            publication_builtin_topic_data: PublicationBuiltinTopicData {
                key: BuiltInTopicKey { value: guid.into() },
                participant_key: BuiltInTopicKey { value: [1; 16] },
                topic_name: topic_shared.read_lock().topic_name.clone(),
                type_name: Foo::type_name().to_string(),
                durability: DurabilityQosPolicy::default(),
                durability_service: DurabilityServiceQosPolicy::default(),
                deadline: DeadlineQosPolicy::default(),
                latency_budget: LatencyBudgetQosPolicy::default(),
                liveliness: LivelinessQosPolicy::default(),
                reliability: ReliabilityQosPolicy {
                    kind: ReliabilityQosPolicyKind::BestEffortReliabilityQos,
                    max_blocking_time: Duration::new(3, 0),
                },
                lifespan: LifespanQosPolicy::default(),
                user_data: UserDataQosPolicy::default(),
                ownership: OwnershipQosPolicy::default(),
                ownership_strength: OwnershipStrengthQosPolicy::default(),
                destination_order: DestinationOrderQosPolicy::default(),
                presentation: PresentationQosPolicy::default(),
                partition: PartitionQosPolicy::default(),
                topic_data: TopicDataQosPolicy::default(),
                group_data: GroupDataQosPolicy::default(),
            },
        };

        sedp_builtin_publications_announcer
            .write_w_timestamp(
                &sedp_discovered_writer_data,
                None,
                rust_dds_api::dcps_psm::Time { sec: 0, nanosec: 0 },
            )
            .unwrap();

        // let data_writer_impl = DataWriterImpl::new(qos, rtps_writer_impl);
        let data_writer_shared = RtpsShared::new(DataWriterAttributes {
            _qos: qos,
            rtps_writer: rtps_writer_impl,
            _listener: None,
            topic: topic_shared.clone(),
            publisher: publisher_shared.downgrade(),
        });

        publisher_shared
            .write_lock()
            .data_writer_list
            .push(data_writer_shared.clone());

        Some(DataWriterProxy::new(data_writer_shared.downgrade()))
    }

    fn datawriter_factory_delete_datawriter(
        &self,
        datawriter: &Self::DataWriterType,
    ) -> DDSResult<()> {
        let publisher_shared = self.0.upgrade()?;
        let datawriter_shared = datawriter.as_ref().upgrade()?;

        let data_writer_list = &mut publisher_shared.write_lock().data_writer_list;

        data_writer_list.remove(
            data_writer_list
                .iter()
                .position(|x| x == &datawriter_shared)
                .ok_or(DDSError::PreconditionNotMet(
                    "Data writer can only be deleted from its parent publisher".to_string(),
                ))?,
        );

        Ok(())
    }

    fn datawriter_factory_lookup_datawriter(
        &self,
        topic: &Self::TopicType,
    ) -> Option<Self::DataWriterType> {
        let publisher_shared = self.0.upgrade().ok()?;
        let data_writer_list = &publisher_shared.write_lock().data_writer_list;

        let topic_shared = topic.as_ref().upgrade().ok()?;
        let topic = topic_shared.read_lock();

        data_writer_list.iter().find_map(|data_writer_shared| {
            let data_writer_lock = data_writer_shared.read_lock();
            let data_writer_topic = data_writer_lock.topic.read_lock();

            if data_writer_topic.topic_name == topic.topic_name
                && data_writer_topic.type_name == Foo::type_name()
            {
                Some(DataWriterProxy::new(data_writer_shared.downgrade()))
            } else {
                None
            }
        })
    }
}

impl<Rtps> Publisher for PublisherProxy<Rtps>
where
    Rtps: RtpsStructure,
{
    type DomainParticipant = DomainParticipantProxy<Rtps>;

    fn suspend_publications(&self) -> DDSResult<()> {
        // self.rtps_writer_group_impl
        //     .upgrade()?
        //     .suspend_publications()
        todo!()
    }

    fn resume_publications(&self) -> DDSResult<()> {
        // self.rtps_writer_group_impl.upgrade()?.resume_publications()
        todo!()
    }

    fn begin_coherent_changes(&self) -> DDSResult<()> {
        todo!()
    }

    fn end_coherent_changes(&self) -> DDSResult<()> {
        todo!()
    }

    fn wait_for_acknowledgments(
        &self,
        _max_wait: rust_dds_api::dcps_psm::Duration,
    ) -> DDSResult<()> {
        todo!()
    }

    fn delete_contained_entities(&self) -> DDSResult<()> {
        todo!()
    }

    fn set_default_datawriter_qos(&mut self, _qos: Option<DataWriterQos>) -> DDSResult<()> {
        // self.rtps_writer_group_impl
        //     .upgrade()?
        //     .set_default_datawriter_qos(qos)
        todo!()
    }

    fn get_default_datawriter_qos(&self) -> DataWriterQos {
        // self.default_datawriter_qos.lock().unwrap().clone()
        todo!()
    }

    fn copy_from_topic_qos(
        &self,
        _a_datawriter_qos: &mut DataWriterQos,
        _a_topic_qos: &TopicQos,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_participant(&self) -> DDSResult<Self::DomainParticipant> {
        let publisher_attributes = self.0.upgrade()?;
        let publisher_attributes_lock = publisher_attributes.read_lock();
        Ok(DomainParticipantProxy::new(
            publisher_attributes_lock.parent_participant.clone(),
        ))
    }
}

impl<Rtps> Entity for PublisherProxy<Rtps>
where
    Rtps: RtpsStructure,
{
    type Qos = PublisherQos;
    type Listener = &'static dyn PublisherListener;

    fn set_qos(&mut self, _qos: Option<Self::Qos>) -> DDSResult<()> {
        // rtps_shared_write_lock(&rtps_weak_upgrade(&self.publisher_impl)?).set_qos(qos)
        todo!()
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.publisher_impl)?).get_qos()
        todo!()
    }

    fn set_listener(
        &self,
        _a_listener: Option<Self::Listener>,
        _mask: StatusMask,
    ) -> DDSResult<()> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.publisher_impl)?)
        //     .set_listener(a_listener, mask)
        todo!()
    }

    fn get_listener(&self) -> DDSResult<Option<Self::Listener>> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.publisher_impl)?).get_listener()
        todo!()
    }

    fn get_statuscondition(&self) -> DDSResult<StatusCondition> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.publisher_impl)?).get_statuscondition()
        todo!()
    }

    fn get_status_changes(&self) -> DDSResult<StatusMask> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.publisher_impl)?).get_status_changes()
        todo!()
    }

    fn enable(&self) -> DDSResult<()> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.publisher_impl)?).enable()
        todo!()
    }

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.publisher_impl)?).get_instance_handle()
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, sync::atomic::AtomicU8, vec};

    use rust_dds_api::{
        dcps_psm::DomainId,
        infrastructure::qos::{DataWriterQos, DomainParticipantQos, PublisherQos, TopicQos},
        publication::publisher::PublisherDataWriterFactory,
        return_type::{DDSError, DDSResult},
    };
    use rust_rtps_pim::{
        behavior::{
            types::Duration,
            writer::{
                stateful_writer::RtpsStatefulWriterConstructor,
                writer::{RtpsWriterAttributes, RtpsWriterOperations},
            },
        },
        structure::{
            entity::RtpsEntityAttributes,
            history_cache::RtpsHistoryCacheOperations,
            participant::RtpsParticipantConstructor,
            types::{
                ChangeKind, Guid, GuidPrefix, InstanceHandle, Locator, ReliabilityKind,
                SequenceNumber, TopicKind, GUID_UNKNOWN,
            },
        }, discovery::sedp::builtin_endpoints::SedpBuiltinPublicationsWriter,
    };

    use crate::{
        data_representation_builtin_endpoints::sedp_discovered_writer_data::{
            SedpDiscoveredWriterData, DCPS_PUBLICATION,
        },
        dds_impl::{
            domain_participant_proxy::DomainParticipantAttributes,
            topic_proxy::{TopicAttributes, TopicProxy}, data_writer_proxy::{DataWriterAttributes, RtpsWriter},
        },
        dds_type::{DdsSerialize, DdsType, Endianness},
        utils::{
            rtps_structure::RtpsStructure,
            shared_object::{RtpsShared, RtpsWeak},
        },
    };

    use super::{PublisherAttributes, PublisherProxy};

    #[derive(Default)]
    struct EmptyGroup;

    impl RtpsEntityAttributes for EmptyGroup {
        fn guid(&self) -> &Guid {
            &GUID_UNKNOWN
        }
    }

    struct EmptyHistoryCache {}
    impl RtpsHistoryCacheOperations for EmptyHistoryCache {
        type CacheChangeType = ();
        fn add_change(&mut self, _change: ()) {}
        fn remove_change(&mut self, _seq_num: &SequenceNumber) {}
        fn get_seq_num_min(&self) -> Option<SequenceNumber> {
            None
        }
        fn get_seq_num_max(&self) -> Option<SequenceNumber> {
            None
        }
    }

    struct EmptyWriter {
        push_mode: bool,
        heartbeat_period: Duration,
        nack_response_delay: Duration,
        nack_suppression_duration: Duration,
        last_change_sequence_number: SequenceNumber,
        data_max_serialized: Option<i32>,
        writer_cache: EmptyHistoryCache,
    }
    impl RtpsWriterOperations for EmptyWriter {
        type DataType = Vec<u8>;
        type ParameterListType = Vec<u8>;
        type CacheChangeType = ();

        fn new_change(
            &mut self,
            _kind: ChangeKind,
            _data: Vec<u8>,
            _inline_qos: Vec<u8>,
            _handle: InstanceHandle,
        ) -> () {
            ()
        }
    }
    impl RtpsWriterAttributes for EmptyWriter {
        type WriterHistoryCacheType = EmptyHistoryCache;

        fn push_mode(&self) -> &bool {
            &self.push_mode
        }
        fn heartbeat_period(&self) -> &Duration {
            &self.heartbeat_period
        }
        fn nack_response_delay(&self) -> &Duration {
            &self.nack_response_delay
        }
        fn nack_suppression_duration(&self) -> &Duration {
            &self.nack_suppression_duration
        }
        fn last_change_sequence_number(&self) -> &SequenceNumber {
            &self.last_change_sequence_number
        }
        fn data_max_size_serialized(&self) -> &Option<i32> {
            &self.data_max_serialized
        }
        fn writer_cache(&mut self) -> &mut EmptyHistoryCache {
            &mut self.writer_cache
        }
    }

    impl RtpsStatefulWriterConstructor for EmptyWriter {
        fn new(
            _guid: Guid,
            _topic_kind: TopicKind,
            _reliability_level: ReliabilityKind,
            _unicast_locator_list: &[Locator],
            _multicast_locator_list: &[Locator],
            _push_mode: bool,
            _heartbeat_period: Duration,
            _nack_response_delay: Duration,
            _nack_suppression_duration: Duration,
            _data_max_size_serialized: Option<i32>,
        ) -> Self {
            EmptyWriter {
                push_mode: false,
                heartbeat_period: Duration::new(0, 0),
                nack_response_delay: Duration::new(0, 0),
                nack_suppression_duration: Duration::new(0, 0),
                last_change_sequence_number: SequenceNumber::default(),
                data_max_serialized: None,
                writer_cache: EmptyHistoryCache {},
            }
        }
    }

    #[derive(Default)]
    struct EmptyParticipant {}

    impl RtpsParticipantConstructor for EmptyParticipant {
        fn new(
            _guid: Guid,
            _protocol_version: rust_rtps_pim::structure::types::ProtocolVersion,
            _vendor_id: rust_rtps_pim::structure::types::VendorId,
            _default_unicast_locator_list: &[Locator],
            _default_multicast_locator_list: &[Locator],
        ) -> Self {
            EmptyParticipant {}
        }
    }

    struct EmptyRtps {}

    impl RtpsStructure for EmptyRtps {
        type Group = EmptyGroup;
        type Participant = EmptyParticipant;
        type StatelessWriter = EmptyWriter;
        type StatefulWriter = EmptyWriter;
        type StatelessReader = ();
        type StatefulReader = ();
    }

    macro_rules! make_empty_dds_type {
        ($type_name:ident) => {
            struct $type_name {}

            impl DdsSerialize for $type_name {
                fn serialize<W: Write, E: Endianness>(&self, _writer: W) -> DDSResult<()> {
                    Ok(())
                }
            }

            impl DdsType for $type_name {
                fn type_name() -> &'static str {
                    stringify!($type_name)
                }

                fn has_key() -> bool {
                    false
                }
            }
        };
    }

    make_empty_dds_type!(Foo);

    fn make_publisher<Rtps: RtpsStructure>(
        parent: RtpsWeak<DomainParticipantAttributes<Rtps>>,
    ) -> RtpsShared<PublisherAttributes<Rtps>>
    where
        Rtps::Group: Default,
    {
        RtpsShared::new(PublisherAttributes {
            _qos: PublisherQos::default(),
            rtps_group: Rtps::Group::default(),
            data_writer_list: Vec::new(),
            user_defined_data_writer_counter: AtomicU8::new(0),
            default_datawriter_qos: DataWriterQos::default(),
            parent_participant: parent,
        })
    }

    fn default_participant<Rtps>() -> RtpsShared<DomainParticipantAttributes<Rtps>>
    where
        Rtps: RtpsStructure<StatefulWriter = EmptyWriter>,
        Rtps::Participant: Default + RtpsParticipantConstructor,
        Rtps::Group: Default,
    {
        let domain_participant = RtpsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([0; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));

        domain_participant.write_lock().builtin_publisher =
            Some(RtpsShared::new(PublisherAttributes::new(
                PublisherQos::default(),
                Rtps::Group::default(),
                domain_participant.downgrade(),
            )));

        let sedp_topic_publication = RtpsShared::new(TopicAttributes::<Rtps>::new(
            TopicQos::default(),
            SedpDiscoveredWriterData::type_name(),
            DCPS_PUBLICATION,
            RtpsWeak::new(),
        ));

        domain_participant
            .write_lock()
            .topic_list
            .push(sedp_topic_publication.clone());

        let sedp_builtin_publications_rtps_writer =
            SedpBuiltinPublicationsWriter::create::<EmptyWriter>(GuidPrefix([0;12]), &[], &[]);
        let sedp_builtin_publications_data_writer = RtpsShared::new(DataWriterAttributes::new(
            DataWriterQos::default(),
            RtpsWriter::Stateful(sedp_builtin_publications_rtps_writer),
            sedp_topic_publication.clone(),
            domain_participant.read_lock().builtin_publisher.as_ref().unwrap().downgrade(),
        ));
        domain_participant.read_lock().builtin_publisher.as_ref().unwrap()
            .write_lock()
            .data_writer_list
            .push(sedp_builtin_publications_data_writer.clone());

        domain_participant
    }

    fn make_topic<Rtps: RtpsStructure>(
        type_name: &'static str,
        topic_name: &'static str,
    ) -> TopicAttributes<Rtps> {
        TopicAttributes::new(TopicQos::default(), type_name, topic_name, RtpsWeak::new())
    }

    #[test]
    fn datawriter_factory_create_datawriter() {
        let domain_participant = default_participant::<EmptyRtps>();

        let publisher = make_publisher::<EmptyRtps>(domain_participant.downgrade());
        let publisher_proxy = PublisherProxy::new(publisher.downgrade());

        let topic = RtpsShared::new(make_topic(Foo::type_name(), "topic"));
        let topic_proxy = TopicProxy::<Foo, EmptyRtps>::new(topic.downgrade());

        let data_writer =
            publisher_proxy.datawriter_factory_create_datawriter(&topic_proxy, None, None, 0);

        assert!(data_writer.is_some());
        assert_eq!(1, publisher.read_lock().data_writer_list.len());
    }

    #[test]
    fn datawriter_factory_delete_datawriter() {
        let domain_participant = default_participant::<EmptyRtps>();

        let publisher = make_publisher::<EmptyRtps>(domain_participant.downgrade());
        let publisher_proxy = PublisherProxy::new(publisher.downgrade());

        let topic = RtpsShared::new(make_topic(Foo::type_name(), "topic"));
        let topic_proxy = TopicProxy::<Foo, EmptyRtps>::new(topic.downgrade());

        let data_writer = publisher_proxy
            .datawriter_factory_create_datawriter(&topic_proxy, None, None, 0)
            .unwrap();

        assert_eq!(1, publisher.read_lock().data_writer_list.len());

        publisher_proxy
            .datawriter_factory_delete_datawriter(&data_writer)
            .unwrap();

        assert_eq!(0, publisher.read_lock().data_writer_list.len());
        assert!(data_writer.as_ref().upgrade().is_err())
    }

    #[test]
    fn datawriter_factory_delete_datawriter_from_other_publisher() {
        let domain_participant = default_participant::<EmptyRtps>();

        let publisher = make_publisher::<EmptyRtps>(domain_participant.downgrade());
        let publisher_proxy = PublisherProxy::new(publisher.downgrade());

        let publisher2 = make_publisher::<EmptyRtps>(domain_participant.downgrade());
        let publisher2_proxy = PublisherProxy::new(publisher2.downgrade());

        let topic = RtpsShared::new(make_topic(Foo::type_name(), "topic"));
        let topic_proxy = TopicProxy::<Foo, EmptyRtps>::new(topic.downgrade());

        let data_writer = publisher_proxy
            .datawriter_factory_create_datawriter(&topic_proxy, None, None, 0)
            .unwrap();

        assert_eq!(1, publisher.read_lock().data_writer_list.len());
        assert_eq!(0, publisher2.read_lock().data_writer_list.len());

        assert!(matches!(
            publisher2_proxy.datawriter_factory_delete_datawriter(&data_writer),
            Err(DDSError::PreconditionNotMet(_))
        ));
        assert!(data_writer.as_ref().upgrade().is_ok())
    }

    #[test]
    fn datawriter_factory_lookup_datawriter_with_no_datawriter() {
        let domain_participant = default_participant::<EmptyRtps>();

        let publisher = make_publisher::<EmptyRtps>(domain_participant.downgrade());
        let publisher_proxy = PublisherProxy::new(publisher.downgrade());

        let topic = RtpsShared::new(make_topic(Foo::type_name(), "topic"));
        let topic_proxy = TopicProxy::<Foo, EmptyRtps>::new(topic.downgrade());

        assert!(publisher_proxy
            .datawriter_factory_lookup_datawriter(&topic_proxy)
            .is_none());
    }

    #[test]
    fn datawriter_factory_lookup_datawriter_with_one_datawriter() {
        let domain_participant = default_participant::<EmptyRtps>();

        let publisher = make_publisher::<EmptyRtps>(domain_participant.downgrade());
        let publisher_proxy = PublisherProxy::new(publisher.downgrade());

        let topic = RtpsShared::new(make_topic(Foo::type_name(), "topic"));
        let topic_proxy = TopicProxy::<Foo, EmptyRtps>::new(topic.downgrade());

        let data_writer = publisher_proxy
            .datawriter_factory_create_datawriter(&topic_proxy, None, None, 0)
            .unwrap();

        assert!(
            publisher_proxy
                .datawriter_factory_lookup_datawriter(&topic_proxy)
                .unwrap()
                .as_ref()
                .upgrade()
                .unwrap()
                == data_writer.as_ref().upgrade().unwrap()
        );
    }

    make_empty_dds_type!(Bar);

    #[test]
    fn datawriter_factory_lookup_datawriter_with_one_datawriter_with_wrong_type() {
        let domain_participant = default_participant::<EmptyRtps>();

        let publisher = make_publisher::<EmptyRtps>(domain_participant.downgrade());
        let publisher_proxy = PublisherProxy::new(publisher.downgrade());

        let topic_foo = RtpsShared::new(make_topic(Foo::type_name(), "topic"));
        let topic_foo_proxy = TopicProxy::<Foo, EmptyRtps>::new(topic_foo.downgrade());

        let topic_bar = RtpsShared::new(make_topic(Bar::type_name(), "topic"));
        let topic_bar_proxy = TopicProxy::<Bar, EmptyRtps>::new(topic_bar.downgrade());

        publisher_proxy
            .datawriter_factory_create_datawriter(&topic_bar_proxy, None, None, 0)
            .unwrap();

        assert!(publisher_proxy
            .datawriter_factory_lookup_datawriter(&topic_foo_proxy)
            .is_none());
    }

    #[test]
    fn datawriter_factory_lookup_datawriter_with_one_datawriter_with_wrong_topic() {
        let domain_participant = default_participant::<EmptyRtps>();

        let publisher = make_publisher::<EmptyRtps>(domain_participant.downgrade());
        let publisher_proxy = PublisherProxy::new(publisher.downgrade());

        let topic1 = RtpsShared::new(make_topic(Foo::type_name(), "topic1"));
        let topic1_proxy = TopicProxy::<Foo, EmptyRtps>::new(topic1.downgrade());

        let topic2 = RtpsShared::new(make_topic(Bar::type_name(), "topic2"));
        let topic2_proxy = TopicProxy::<Bar, EmptyRtps>::new(topic2.downgrade());

        publisher_proxy
            .datawriter_factory_create_datawriter(&topic2_proxy, None, None, 0)
            .unwrap();

        assert!(publisher_proxy
            .datawriter_factory_lookup_datawriter(&topic1_proxy)
            .is_none());
    }

    #[test]
    fn datawriter_factory_lookup_datawriter_with_two_dawriters_with_different_types() {
        let domain_participant = default_participant::<EmptyRtps>();

        let publisher = make_publisher::<EmptyRtps>(domain_participant.downgrade());
        let publisher_proxy = PublisherProxy::new(publisher.downgrade());

        let topic_foo = RtpsShared::new(make_topic::<EmptyRtps>(Foo::type_name(), "topic"));
        let topic_foo_proxy = TopicProxy::<Foo, EmptyRtps>::new(topic_foo.downgrade());

        let topic_bar = RtpsShared::new(make_topic::<EmptyRtps>(Bar::type_name(), "topic"));
        let topic_bar_proxy = TopicProxy::<Bar, EmptyRtps>::new(topic_bar.downgrade());

        let data_writer_foo = publisher_proxy
            .datawriter_factory_create_datawriter(&topic_foo_proxy, None, None, 0)
            .unwrap();
        let data_writer_bar = publisher_proxy
            .datawriter_factory_create_datawriter(&topic_bar_proxy, None, None, 0)
            .unwrap();

        assert!(
            publisher_proxy
                .datawriter_factory_lookup_datawriter(&topic_foo_proxy)
                .unwrap()
                .as_ref()
                .upgrade()
                .unwrap()
                == data_writer_foo.as_ref().upgrade().unwrap()
        );

        assert!(
            publisher_proxy
                .datawriter_factory_lookup_datawriter(&topic_bar_proxy)
                .unwrap()
                .as_ref()
                .upgrade()
                .unwrap()
                == data_writer_bar.as_ref().upgrade().unwrap()
        );
    }

    #[test]
    fn datawriter_factory_lookup_datawriter_with_two_datawriters_with_different_topics() {
        let domain_participant = default_participant::<EmptyRtps>();

        let publisher = make_publisher::<EmptyRtps>(domain_participant.downgrade());
        let publisher_proxy = PublisherProxy::new(publisher.downgrade());

        let topic1 = RtpsShared::new(make_topic::<EmptyRtps>(Foo::type_name(), "topic1"));
        let topic1_proxy = TopicProxy::<Foo, EmptyRtps>::new(topic1.downgrade());

        let topic2 = RtpsShared::new(make_topic::<EmptyRtps>(Bar::type_name(), "topic2"));
        let topic2_proxy = TopicProxy::<Bar, EmptyRtps>::new(topic2.downgrade());

        let data_writer1 = publisher_proxy
            .datawriter_factory_create_datawriter(&topic1_proxy, None, None, 0)
            .unwrap();
        let data_writer2 = publisher_proxy
            .datawriter_factory_create_datawriter(&topic2_proxy, None, None, 0)
            .unwrap();

        assert!(
            publisher_proxy
                .datawriter_factory_lookup_datawriter(&topic1_proxy)
                .unwrap()
                .as_ref()
                .upgrade()
                .unwrap()
                == data_writer1.as_ref().upgrade().unwrap()
        );

        assert!(
            publisher_proxy
                .datawriter_factory_lookup_datawriter(&topic2_proxy)
                .unwrap()
                .as_ref()
                .upgrade()
                .unwrap()
                == data_writer2.as_ref().upgrade().unwrap()
        );
    }
}
