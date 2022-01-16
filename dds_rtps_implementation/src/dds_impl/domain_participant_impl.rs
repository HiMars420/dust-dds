use std::sync::{
    atomic::{self, AtomicBool, AtomicU8},
    Arc,
};

use rust_dds_api::{
    builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    dcps_psm::{BuiltInTopicKey, DomainId, Duration, InstanceHandle, StatusMask, Time},
    domain::{
        domain_participant::{DomainParticipant, DomainParticipantTopicFactory},
        domain_participant_listener::DomainParticipantListener,
    },
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::{DomainParticipantQos, PublisherQos, SubscriberQos, TopicQos},
    },
    publication::{publisher::Publisher, publisher_listener::PublisherListener},
    return_type::DDSResult,
    subscription::subscriber_listener::SubscriberListener,
    topic::{topic_description::TopicDescription, topic_listener::TopicListener},
};
use rust_rtps_pim::{
    discovery::{
        spdp::participant_proxy::ParticipantProxy,
        types::{BuiltinEndpointQos, BuiltinEndpointSet},
    },
    messages::types::Count,
    structure::{
        entity::RtpsEntityAttributes,
        participant::RtpsParticipantAttributes,
        types::{
            EntityId, Guid, GuidPrefix, Locator, ENTITYID_PARTICIPANT, PROTOCOLVERSION,
            USER_DEFINED_READER_GROUP, USER_DEFINED_WRITER_GROUP, VENDOR_ID_S2E,
        },
    },
};

use crate::{
    data_representation_builtin_endpoints::spdp_discovered_participant_data::SpdpDiscoveredParticipantData,
    dds_type::DdsType,
    rtps_impl::{rtps_group_impl::RtpsGroupImpl, rtps_participant_impl::RtpsParticipantImpl},
    utils::shared_object::{
        rtps_shared_new, rtps_shared_read_lock, rtps_shared_write_lock, RtpsShared,
    },
};

use super::{
    publisher_impl::PublisherImpl, subscriber_impl::SubscriberImpl, topic_impl::TopicImpl,
};

pub trait AnyTopic {}

impl<Foo> AnyTopic for TopicImpl<Foo> {}

pub struct DomainParticipantImpl {
    rtps_participant: RtpsParticipantImpl,
    domain_id: DomainId,
    domain_tag: Arc<String>,
    qos: DomainParticipantQos,
    builtin_subscriber: RtpsShared<SubscriberImpl>,
    builtin_publisher: RtpsShared<PublisherImpl>,
    user_defined_subscriber_list: RtpsShared<Vec<RtpsShared<SubscriberImpl>>>,
    user_defined_subscriber_counter: AtomicU8,
    default_subscriber_qos: SubscriberQos,
    user_defined_publisher_list: RtpsShared<Vec<RtpsShared<PublisherImpl>>>,
    user_defined_publisher_counter: AtomicU8,
    default_publisher_qos: PublisherQos,
    topic_list: RtpsShared<Vec<RtpsShared<dyn AnyTopic>>>,
    default_topic_qos: TopicQos,
    manual_liveliness_count: Count,
    lease_duration: rust_rtps_pim::behavior::types::Duration,
    metatraffic_unicast_locator_list: Vec<Locator>,
    metatraffic_multicast_locator_list: Vec<Locator>,
    enabled: Arc<AtomicBool>,
}

impl DomainParticipantImpl {
    pub fn new(
        guid_prefix: GuidPrefix,
        domain_id: DomainId,
        domain_tag: Arc<String>,
        domain_participant_qos: DomainParticipantQos,
        metatraffic_unicast_locator_list: Vec<Locator>,
        metatraffic_multicast_locator_list: Vec<Locator>,
        default_unicast_locator_list: Vec<Locator>,
        default_multicast_locator_list: Vec<Locator>,
        builtin_subscriber: RtpsShared<SubscriberImpl>,
        builtin_publisher: RtpsShared<PublisherImpl>,
        user_defined_subscriber_list: RtpsShared<Vec<RtpsShared<SubscriberImpl>>>,
        user_defined_publisher_list: RtpsShared<Vec<RtpsShared<PublisherImpl>>>,
        enabled: Arc<AtomicBool>,
    ) -> Self {
        let lease_duration = rust_rtps_pim::behavior::types::Duration::new(100, 0);
        let protocol_version = PROTOCOLVERSION;
        let vendor_id = VENDOR_ID_S2E;
        let rtps_participant = RtpsParticipantImpl::new(
            Guid::new(guid_prefix, ENTITYID_PARTICIPANT),
            protocol_version,
            vendor_id,
            default_unicast_locator_list,
            default_multicast_locator_list,
        );

        Self {
            rtps_participant,
            domain_id,
            domain_tag,
            qos: domain_participant_qos,
            builtin_subscriber,
            builtin_publisher,
            user_defined_subscriber_list,
            user_defined_subscriber_counter: AtomicU8::new(0),
            default_subscriber_qos: SubscriberQos::default(),
            user_defined_publisher_list,
            user_defined_publisher_counter: AtomicU8::new(0),
            default_publisher_qos: PublisherQos::default(),
            topic_list: rtps_shared_new(Vec::new()),
            default_topic_qos: TopicQos::default(),
            manual_liveliness_count: Count(0),
            lease_duration,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            enabled,
        }
    }

    pub fn as_spdp_discovered_participant_data(&self) -> SpdpDiscoveredParticipantData {
        SpdpDiscoveredParticipantData {
            dds_participant_data: ParticipantBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: (*self.rtps_participant.guid()).into(),
                },
                user_data: self.qos.user_data.clone(),
            },
            participant_proxy: ParticipantProxy {
                domain_id: self.domain_id as u32,
                domain_tag: self.domain_tag.as_ref().clone(),
                protocol_version: *self.rtps_participant.protocol_version(),
                guid_prefix: *self.rtps_participant.guid().prefix(),
                vendor_id: *self.rtps_participant.vendor_id(),
                expects_inline_qos: false,
                metatraffic_unicast_locator_list: self.metatraffic_unicast_locator_list.clone(),
                metatraffic_multicast_locator_list: self.metatraffic_multicast_locator_list.clone(),
                default_unicast_locator_list: self
                    .rtps_participant
                    .default_unicast_locator_list()
                    .to_vec(),
                default_multicast_locator_list: self
                    .rtps_participant
                    .default_multicast_locator_list()
                    .to_vec(),
                available_builtin_endpoints: BuiltinEndpointSet::default(),
                manual_liveliness_count: self.manual_liveliness_count,
                builtin_endpoint_qos: BuiltinEndpointQos::default(),
            },
            lease_duration: self.lease_duration,
        }
    }
}

impl<Foo> DomainParticipantTopicFactory<Foo> for DomainParticipantImpl
where
    Foo: DdsType + 'static,
{
    type TopicType = RtpsShared<TopicImpl<Foo>>;

    fn topic_factory_create_topic(
        &self,
        topic_name: &str,
        qos: Option<TopicQos>,
        _a_listener: Option<Box<dyn TopicListener>>,
        _mask: StatusMask,
    ) -> Option<Self::TopicType> {
        let topic_qos = qos.unwrap_or(self.default_topic_qos.clone());

        let _builtin_publisher_lock = rtps_shared_read_lock(&self.builtin_publisher);
        // if let Some(sedp_builtin_topics_writer) =
        //     builtin_publisher_lock.lookup_datawriter::<SedpDiscoveredTopicData>(&())
        // {
        //     let mut sedp_builtin_topics_writer_lock =
        //         rtps_shared_write_lock(&sedp_builtin_topics_writer);
        //     let sedp_discovered_topic_data = SedpDiscoveredTopicData {
        //         topic_builtin_topic_data: TopicBuiltinTopicData {
        //             key: BuiltInTopicKey { value: [1; 16] },
        //             name: topic_name.to_string(),
        //             type_name: Foo::type_name().to_string(),
        //             durability: topic_qos.durability.clone(),
        //             durability_service: topic_qos.durability_service.clone(),
        //             deadline: topic_qos.deadline.clone(),
        //             latency_budget: topic_qos.latency_budget.clone(),
        //             liveliness: topic_qos.liveliness.clone(),
        //             reliability: topic_qos.reliability.clone(),
        //             transport_priority: topic_qos.transport_priority.clone(),
        //             lifespan: topic_qos.lifespan.clone(),
        //             destination_order: topic_qos.destination_order.clone(),
        //             history: topic_qos.history.clone(),
        //             resource_limits: topic_qos.resource_limits.clone(),
        //             ownership: topic_qos.ownership.clone(),
        //             topic_data: topic_qos.topic_data.clone(),
        //         },
        //     };
        //     sedp_builtin_topics_writer_lock
        //         .write_w_timestamp(
        //             &sedp_discovered_topic_data,
        //             None,
        //             Time { sec: 0, nanosec: 0 },
        //         )
        //         .ok()?;
        // }

        let topic_impl = TopicImpl::new(topic_qos, Foo::type_name(), topic_name);
        let topic_impl_shared = rtps_shared_new(topic_impl);
        rtps_shared_write_lock(&self.topic_list).push(topic_impl_shared.clone());

        Some(topic_impl_shared)
    }

    fn topic_factory_delete_topic(&self, a_topic: &Self::TopicType) -> DDSResult<()> {
        let any_topic: RtpsShared<dyn AnyTopic> = a_topic.clone();
        rtps_shared_write_lock(&self.topic_list).retain(|x| !Arc::ptr_eq(x, &any_topic));
        Ok(())
    }

    fn topic_factory_find_topic(
        &self,
        _topic_name: &str,
        _timeout: Duration,
    ) -> Option<Self::TopicType> {
        todo!()
    }
}

impl DomainParticipant for DomainParticipantImpl {
    type PublisherType = RtpsShared<PublisherImpl>;
    type SubscriberType = RtpsShared<SubscriberImpl>;

    fn create_publisher(
        &self,
        qos: Option<PublisherQos>,
        _a_listener: Option<&'static dyn PublisherListener>,
        _mask: StatusMask,
    ) -> Option<Self::PublisherType> {
        let publisher_qos = qos.unwrap_or(self.default_publisher_qos.clone());
        let user_defined_publisher_counter = self
            .user_defined_publisher_counter
            .fetch_add(1, atomic::Ordering::SeqCst);
        let entity_id = EntityId::new(
            [user_defined_publisher_counter, 0, 0],
            USER_DEFINED_WRITER_GROUP,
        );
        let guid = Guid::new(*self.rtps_participant.guid().prefix(), entity_id);
        let rtps_group = RtpsGroupImpl::new(guid);
        let sedp_builtin_publications_topic =
            rtps_shared_new(TopicImpl::new(TopicQos::default(), "", ""));
        let sedp_builtin_publications_announcer = rtps_shared_read_lock(&self.builtin_publisher)
            .lookup_datawriter(&sedp_builtin_publications_topic);
        let publisher_impl = PublisherImpl::new(
            publisher_qos,
            rtps_group,
            Vec::new(),
            sedp_builtin_publications_announcer,
        );
        let publisher_impl_shared = rtps_shared_new(publisher_impl);
        rtps_shared_write_lock(&self.user_defined_publisher_list)
            .push(publisher_impl_shared.clone());

        Some(publisher_impl_shared)
    }

    fn delete_publisher(&self, a_publisher: &Self::PublisherType) -> DDSResult<()> {
        rtps_shared_write_lock(&self.user_defined_publisher_list)
            .retain(|x| !Arc::ptr_eq(&x, &a_publisher));
        Ok(())
    }

    fn create_subscriber(
        &self,
        qos: Option<SubscriberQos>,
        _a_listener: Option<&'static dyn SubscriberListener>,
        _mask: StatusMask,
    ) -> Option<Self::SubscriberType> {
        let subscriber_qos = qos.unwrap_or(self.default_subscriber_qos.clone());
        let user_defined_subscriber_counter = self
            .user_defined_subscriber_counter
            .fetch_add(1, atomic::Ordering::SeqCst);
        let entity_id = EntityId::new(
            [user_defined_subscriber_counter, 0, 0],
            USER_DEFINED_READER_GROUP,
        );
        let guid = Guid::new(*self.rtps_participant.guid().prefix(), entity_id);
        let rtps_group = RtpsGroupImpl::new(guid);
        let subscriber = SubscriberImpl::new(subscriber_qos, rtps_group, Vec::new());
        let subscriber_shared = rtps_shared_new(subscriber);
        rtps_shared_write_lock(&self.user_defined_subscriber_list).push(subscriber_shared.clone());
        Some(subscriber_shared)
    }

    fn delete_subscriber(&self, a_subscriber: &Self::SubscriberType) -> DDSResult<()> {
        rtps_shared_write_lock(&self.user_defined_subscriber_list)
            .retain(|x| !Arc::ptr_eq(&x, &a_subscriber));
        Ok(())
    }

    fn lookup_topicdescription<Foo>(
        &self,
        _name: &str,
    ) -> Option<&(dyn TopicDescription<DomainParticipant = Self>)> {
        todo!()
    }

    fn get_builtin_subscriber(&self) -> DDSResult<Self::SubscriberType> {
        Ok(self.builtin_subscriber.clone())
    }

    fn ignore_participant(&self, _handle: InstanceHandle) -> DDSResult<()> {
        todo!()
    }

    fn ignore_topic(&self, _handle: InstanceHandle) -> DDSResult<()> {
        todo!()
    }

    fn ignore_publication(&self, _handle: InstanceHandle) -> DDSResult<()> {
        todo!()
    }

    fn ignore_subscription(&self, _handle: InstanceHandle) -> DDSResult<()> {
        todo!()
    }

    fn get_domain_id(&self) -> DomainId {
        // self.domain_id
        todo!()
    }

    fn delete_contained_entities(&self) -> DDSResult<()> {
        todo!()
    }

    fn assert_liveliness(&self) -> DDSResult<()> {
        todo!()
    }

    fn set_default_publisher_qos(&mut self, qos: Option<PublisherQos>) -> DDSResult<()> {
        self.default_publisher_qos = qos.unwrap_or_default();
        Ok(())
    }

    fn get_default_publisher_qos(&self) -> PublisherQos {
        self.default_publisher_qos.clone()
    }

    fn set_default_subscriber_qos(&mut self, qos: Option<SubscriberQos>) -> DDSResult<()> {
        self.default_subscriber_qos = qos.unwrap_or_default();
        Ok(())
    }

    fn get_default_subscriber_qos(&self) -> SubscriberQos {
        self.default_subscriber_qos.clone()
    }

    fn set_default_topic_qos(&mut self, qos: Option<TopicQos>) -> DDSResult<()> {
        let topic_qos = qos.unwrap_or_default();
        topic_qos.is_consistent()?;
        self.default_topic_qos = topic_qos;
        Ok(())
    }

    fn get_default_topic_qos(&self) -> TopicQos {
        self.default_topic_qos.clone()
    }

    fn get_discovered_participants(
        &self,
        _participant_handles: &mut [InstanceHandle],
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_discovered_participant_data(
        &self,
        _participant_data: ParticipantBuiltinTopicData,
        _participant_handle: InstanceHandle,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_discovered_topics(&self, _topic_handles: &mut [InstanceHandle]) -> DDSResult<()> {
        todo!()
    }

    fn get_discovered_topic_data(
        &self,
        _topic_data: TopicBuiltinTopicData,
        _topic_handle: InstanceHandle,
    ) -> DDSResult<()> {
        todo!()
    }

    fn contains_entity(&self, _a_handle: InstanceHandle) -> bool {
        todo!()
    }

    fn get_current_time(&self) -> DDSResult<Time> {
        todo!()
    }
}

impl Entity for DomainParticipantImpl {
    type Qos = DomainParticipantQos;
    type Listener = &'static dyn DomainParticipantListener;

    fn set_qos(&mut self, _qos: Option<Self::Qos>) -> DDSResult<()> {
        // self.qos = qos.unwrap_or_default();
        // Ok(())
        todo!()
        // self.domain_participant_storage.lock().set_qos(qos)
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        todo!()
        // Ok(self.domain_participant_storage.lock().get_qos().clone())
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

    fn get_statuscondition(&self) -> DDSResult<StatusCondition> {
        todo!()
    }

    fn get_status_changes(&self) -> DDSResult<StatusMask> {
        todo!()
    }

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle> {
        todo!()
        // Ok(crate::utils::instance_handle_from_guid(
        //     &self.rtps_participant_impl.lock().guid(),
        // ))
    }

    fn enable(&self) -> DDSResult<()> {
        self.enabled.store(true, atomic::Ordering::SeqCst);

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::utils::shared_object::rtps_shared_read_lock;

    use super::*;
    use rust_dds_api::{infrastructure::qos_policy::UserDataQosPolicy, return_type::DDSError};
    use rust_rtps_pim::structure::types::GUID_UNKNOWN;

    #[test]
    fn set_default_publisher_qos_some_value() {
        let builtin_subcriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
        );
        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
            None,
        );
        let mut domain_participant = DomainParticipantImpl::new(
            GuidPrefix([3; 12]),
            1,
            Arc::new("".to_string()),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            rtps_shared_new(builtin_subcriber),
            rtps_shared_new(builtin_publisher),
            rtps_shared_new(Vec::new()),
            rtps_shared_new(Vec::new()),
            Arc::new(AtomicBool::new(false)),
        );
        let mut qos = PublisherQos::default();
        qos.group_data.value = vec![1, 2, 3, 4];
        domain_participant
            .set_default_publisher_qos(Some(qos.clone()))
            .unwrap();
        assert!(domain_participant.get_default_publisher_qos() == qos);
    }

    #[test]
    fn set_default_publisher_qos_none() {
        let builtin_subcriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
        );
        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
            None,
        );
        let mut domain_participant = DomainParticipantImpl::new(
            GuidPrefix([0; 12]),
            1,
            Arc::new("".to_string()),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            rtps_shared_new(builtin_subcriber),
            rtps_shared_new(builtin_publisher),
            rtps_shared_new(Vec::new()),
            rtps_shared_new(Vec::new()),
            Arc::new(AtomicBool::new(false)),
        );
        let mut qos = PublisherQos::default();
        qos.group_data.value = vec![1, 2, 3, 4];
        domain_participant
            .set_default_publisher_qos(Some(qos.clone()))
            .unwrap();

        domain_participant.set_default_publisher_qos(None).unwrap();
        assert!(domain_participant.get_default_publisher_qos() == PublisherQos::default());
    }

    #[test]
    fn set_default_subscriber_qos_some_value() {
        let builtin_subcriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
        );
        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
            None,
        );
        let mut domain_participant = DomainParticipantImpl::new(
            GuidPrefix([1; 12]),
            1,
            Arc::new("".to_string()),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            rtps_shared_new(builtin_subcriber),
            rtps_shared_new(builtin_publisher),
            rtps_shared_new(Vec::new()),
            rtps_shared_new(Vec::new()),
            Arc::new(AtomicBool::new(false)),
        );
        let mut qos = SubscriberQos::default();
        qos.group_data.value = vec![1, 2, 3, 4];
        domain_participant
            .set_default_subscriber_qos(Some(qos.clone()))
            .unwrap();
        assert_eq!(domain_participant.get_default_subscriber_qos(), qos);
    }

    #[test]
    fn set_default_subscriber_qos_none() {
        let builtin_subcriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
        );
        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
            None,
        );
        let mut domain_participant = DomainParticipantImpl::new(
            GuidPrefix([1; 12]),
            1,
            Arc::new("".to_string()),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            rtps_shared_new(builtin_subcriber),
            rtps_shared_new(builtin_publisher),
            rtps_shared_new(Vec::new()),
            rtps_shared_new(Vec::new()),
            Arc::new(AtomicBool::new(false)),
        );
        let mut qos = SubscriberQos::default();
        qos.group_data.value = vec![1, 2, 3, 4];
        domain_participant
            .set_default_subscriber_qos(Some(qos.clone()))
            .unwrap();

        domain_participant.set_default_subscriber_qos(None).unwrap();
        assert_eq!(
            domain_participant.get_default_subscriber_qos(),
            SubscriberQos::default()
        );
    }

    #[test]
    fn set_default_topic_qos_some_value() {
        let builtin_subcriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
        );
        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
            None,
        );
        let mut domain_participant = DomainParticipantImpl::new(
            GuidPrefix([1; 12]),
            1,
            Arc::new("".to_string()),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            rtps_shared_new(builtin_subcriber),
            rtps_shared_new(builtin_publisher),
            rtps_shared_new(Vec::new()),
            rtps_shared_new(Vec::new()),
            Arc::new(AtomicBool::new(false)),
        );
        let mut qos = TopicQos::default();
        qos.topic_data.value = vec![1, 2, 3, 4];
        domain_participant
            .set_default_topic_qos(Some(qos.clone()))
            .unwrap();
        assert_eq!(domain_participant.get_default_topic_qos(), qos);
    }

    #[test]
    fn set_default_topic_qos_inconsistent() {
        let builtin_subcriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
        );
        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
            None,
        );
        let mut domain_participant = DomainParticipantImpl::new(
            GuidPrefix([1; 12]),
            1,
            Arc::new("".to_string()),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            rtps_shared_new(builtin_subcriber),
            rtps_shared_new(builtin_publisher),
            rtps_shared_new(Vec::new()),
            rtps_shared_new(Vec::new()),
            Arc::new(AtomicBool::new(false)),
        );
        let mut qos = TopicQos::default();
        qos.resource_limits.max_samples_per_instance = 2;
        qos.resource_limits.max_samples = 1;
        let set_default_topic_qos_result =
            domain_participant.set_default_topic_qos(Some(qos.clone()));
        assert!(set_default_topic_qos_result == Err(DDSError::InconsistentPolicy));
    }

    #[test]
    fn set_default_topic_qos_none() {
        let builtin_subcriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
        );
        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
            None,
        );
        let mut domain_participant = DomainParticipantImpl::new(
            GuidPrefix([1; 12]),
            1,
            Arc::new("".to_string()),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            rtps_shared_new(builtin_subcriber),
            rtps_shared_new(builtin_publisher),
            rtps_shared_new(Vec::new()),
            rtps_shared_new(Vec::new()),
            Arc::new(AtomicBool::new(false)),
        );
        let mut qos = TopicQos::default();
        qos.topic_data.value = vec![1, 2, 3, 4];
        domain_participant
            .set_default_topic_qos(Some(qos.clone()))
            .unwrap();

        domain_participant.set_default_topic_qos(None).unwrap();
        assert_eq!(
            domain_participant.get_default_topic_qos(),
            TopicQos::default()
        );
    }

    #[test]
    fn create_publisher() {
        let builtin_subcriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
        );
        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
            None,
        );
        let domain_participant = DomainParticipantImpl::new(
            GuidPrefix([1; 12]),
            1,
            Arc::new("".to_string()),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            rtps_shared_new(builtin_subcriber),
            rtps_shared_new(builtin_publisher),
            rtps_shared_new(Vec::new()),
            rtps_shared_new(Vec::new()),
            Arc::new(AtomicBool::new(false)),
        );

        let publisher_counter_before = domain_participant
            .user_defined_publisher_counter
            .load(atomic::Ordering::Relaxed);
        let publisher = domain_participant.create_publisher(None, None, 0);

        let publisher_counter_after = domain_participant
            .user_defined_publisher_counter
            .load(atomic::Ordering::Relaxed);

        assert_eq!(
            rtps_shared_read_lock(&domain_participant.user_defined_publisher_list).len(),
            1
        );

        assert_ne!(publisher_counter_before, publisher_counter_after);
        assert!(publisher.is_some());
    }

    #[test]
    fn delete_publisher() {
        let builtin_subcriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
        );
        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
            None,
        );
        let domain_participant = DomainParticipantImpl::new(
            GuidPrefix([1; 12]),
            1,
            Arc::new("".to_string()),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            rtps_shared_new(builtin_subcriber),
            rtps_shared_new(builtin_publisher),
            rtps_shared_new(Vec::new()),
            rtps_shared_new(Vec::new()),
            Arc::new(AtomicBool::new(false)),
        );
        let a_publisher = domain_participant.create_publisher(None, None, 0).unwrap();

        domain_participant.delete_publisher(&a_publisher).unwrap();
        assert_eq!(
            rtps_shared_read_lock(&domain_participant.user_defined_publisher_list).len(),
            0
        );
    }

    #[test]
    fn domain_participant_as_spdp_discovered_participant_data() {
        let builtin_subcriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
        );
        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            vec![],
            None,
        );
        let domain_participant = DomainParticipantImpl::new(
            GuidPrefix([1; 12]),
            1,
            Arc::new("".to_string()),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            rtps_shared_new(builtin_subcriber),
            rtps_shared_new(builtin_publisher),
            rtps_shared_new(Vec::new()),
            rtps_shared_new(Vec::new()),
            Arc::new(AtomicBool::new(false)),
        );
        let spdp_discovered_participant_data =
            domain_participant.as_spdp_discovered_participant_data();
        let expected_spdp_discovered_participant_data = SpdpDiscoveredParticipantData {
            dds_participant_data: ParticipantBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0xc1],
                },
                user_data: UserDataQosPolicy { value: vec![] },
            },
            participant_proxy: ParticipantProxy {
                domain_id: 1,
                domain_tag: "".to_string(),
                protocol_version: PROTOCOLVERSION,
                guid_prefix: GuidPrefix([1; 12]),
                vendor_id: VENDOR_ID_S2E,
                expects_inline_qos: false,
                metatraffic_unicast_locator_list: vec![],
                metatraffic_multicast_locator_list: vec![],
                default_unicast_locator_list: vec![],
                default_multicast_locator_list: vec![],
                available_builtin_endpoints: BuiltinEndpointSet::default(),
                manual_liveliness_count: Count(0),
                builtin_endpoint_qos: BuiltinEndpointQos::default(),
            },
            lease_duration: rust_rtps_pim::behavior::types::Duration::new(100, 0),
        };

        assert_eq!(
            spdp_discovered_participant_data,
            expected_spdp_discovered_participant_data
        );
    }

    // #[test]
    // fn spdp_data_sent() {
    //     const SPDP_TEST_LOCATOR: Locator = Locator {
    //         kind: LOCATOR_KIND_UDPv4,
    //         port: 7400,
    //         address: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239, 255, 0, 1],
    //     };
    //     struct TestTransport;
    //     impl TransportRead for TestTransport {
    //         fn read(&mut self) -> Option<(Locator, RtpsMessageRead)> {
    //             None
    //         }
    //     }
    //     impl TransportWrite for TestTransport {
    //         fn write(&mut self, message: &RtpsMessageWrite, destination_locator: &Locator) {
    //             assert_eq!(message.submessages.len(), 1);
    //             match &message.submessages[0] {
    //                 RtpsSubmessageTypeWrite::Data(data_submessage) => {
    //                     assert_eq!(data_submessage.reader_id.value, ENTITYID_UNKNOWN);
    //                     assert_eq!(
    //                         data_submessage.writer_id.value,
    //                         ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER
    //                     );
    //                 }
    //                 _ => assert!(false),
    //             };
    //             assert_eq!(destination_locator, &SPDP_TEST_LOCATOR);
    //             println!("Writing {:?}, to {:?}", message, destination_locator);
    //         }
    //     }

    //     let guid_prefix = GuidPrefix([1; 12]);
    //     let mut spdp_builtin_participant_rtps_writer = RtpsStatelessWriterImpl::new(
    //         SpdpBuiltinParticipantWriter::create(guid_prefix, vec![], vec![]),
    //     );

    //     let spdp_discovery_locator = RtpsReaderLocator::new(SPDP_TEST_LOCATOR, false);

    //     spdp_builtin_participant_rtps_writer.reader_locator_add(spdp_discovery_locator);

    //     let spdp_builtin_participant_data_writer =
    //         Some(DataWriterImpl::<SpdpDiscoveredParticipantData, _, _>::new(
    //             DataWriterQos::default(),
    //             spdp_builtin_participant_rtps_writer,
    //             StdTimer::new(),
    //         ));

    //     let domain_participant = DomainParticipantImpl::new(
    //         guid_prefix,
    //         1,
    //         "".to_string(),
    //         DomainParticipantQos::default(),
    //         TestTransport,
    //         TestTransport,
    //         vec![],
    //         vec![],
    //         vec![],
    //         vec![],
    //         None,
    //         spdp_builtin_participant_data_writer,
    //         None,
    //         None,
    //         None,
    //         None,
    //         None,
    //         None,
    //     );
    //     let mut tasks = Vec::new();
    //     tasks.push(receiver.recv().unwrap());
    //     tasks.push(receiver.recv().unwrap());

    //     domain_participant.enable().unwrap();

    //     let builtin_communication_task = tasks
    //         .iter_mut()
    //         .find(|x| x.name == "builtin communication")
    //         .unwrap();

    //     (builtin_communication_task.task)()
    // }

    // #[test]
    // fn spdp_discovery_read() {
    //     const SPDP_TEST_LOCATOR: Locator = Locator {
    //         kind: LOCATOR_KIND_UDPv4,
    //         port: 7400,
    //         address: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239, 255, 0, 1],
    //     };
    //     struct TestTransport;
    //     impl TransportRead for TestTransport {
    //         fn read(&mut self) -> Option<(Locator, RtpsMessageRead)> {
    //             None
    //         }
    //     }
    //     impl TransportWrite for TestTransport {
    //         fn write(&mut self, message: &RtpsMessageWrite, destination_locator: &Locator) {
    //             assert_eq!(message.submessages.len(), 1);
    //             match &message.submessages[0] {
    //                 RtpsSubmessageTypeWrite::Data(data_submessage) => {
    //                     assert_eq!(data_submessage.reader_id.value, ENTITYID_UNKNOWN);
    //                     assert_eq!(
    //                         data_submessage.writer_id.value,
    //                         ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER
    //                     );
    //                 }
    //                 _ => assert!(false),
    //             };
    //             assert_eq!(destination_locator, &SPDP_TEST_LOCATOR);
    //             println!("Writing {:?}, to {:?}", message, destination_locator);
    //         }
    //     }

    //     let (sender, _receiver) = std::sync::mpsc::sync_channel(10);
    //     let spawner = Spawner::new(sender);

    //     let guid_prefix = GuidPrefix([1; 12]);
    //     let spdp_builtin_participant_rtps_reader =
    //         SpdpBuiltinParticipantReader::create(guid_prefix, vec![], vec![]);

    //     let mut spdp_builtin_participant_data_reader =
    //         DataReaderImpl::<SpdpDiscoveredParticipantData>::new(
    //             DataReaderQos::default(),
    //             spdp_builtin_participant_rtps_reader,
    //         );

    //     let spdp_discovered_participant_data = SpdpDiscoveredParticipantData {
    //         dds_participant_data: ParticipantBuiltinTopicData {
    //             key: BuiltInTopicKey { value: [2; 16] },
    //             user_data: UserDataQosPolicy { value: vec![] },
    //         },
    //         participant_proxy: ParticipantProxy {
    //             domain_id: 1,
    //             domain_tag: "".to_string(),
    //             protocol_version: PROTOCOLVERSION,
    //             guid_prefix: GuidPrefix([2; 12]),
    //             vendor_id: VENDOR_ID_S2E,
    //             expects_inline_qos: false,
    //             metatraffic_unicast_locator_list: vec![],
    //             metatraffic_multicast_locator_list: vec![],
    //             default_unicast_locator_list: vec![],
    //             default_multicast_locator_list: vec![],
    //             available_builtin_endpoints: BuiltinEndpointSet::default(),
    //             manual_liveliness_count: Count(1),
    //             builtin_endpoint_qos: BuiltinEndpointQos::default(),
    //         },
    //         lease_duration: rust_rtps_pim::behavior::types::Duration::new(100, 0),
    //     };

    //     let mut serialized_data = Vec::new();
    //     spdp_discovered_participant_data
    //         .serialize::<_, LittleEndian>(&mut serialized_data)
    //         .unwrap();

    //     spdp_builtin_participant_data_reader
    //         .rtps_reader
    //         .reader_cache
    //         .add_change(RtpsCacheChange {
    //             kind: ChangeKind::Alive,
    //             writer_guid: Guid::new(
    //                 GuidPrefix([2; 12]),
    //                 ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
    //             ),
    //             instance_handle: 1,
    //             sequence_number: 1,
    //             data_value: &serialized_data,
    //             inline_qos: &[],
    //         });

    //     let domain_participant = DomainParticipantImpl::new(
    //         guid_prefix,
    //         1,
    //         "".to_string(),
    //         DomainParticipantQos::default(),
    //         TestTransport,
    //         TestTransport,
    //         vec![],
    //         vec![],
    //         vec![],
    //         vec![],
    //         Some(spdp_builtin_participant_data_reader),
    //         None,
    //         None,
    //         None,
    //         None,
    //         None,
    //         None,
    //         None,
    //         spawner,
    //     );
    //     // let mut tasks = Vec::new();
    //     // tasks.push(receiver.recv().unwrap());
    //     // tasks.push(receiver.recv().unwrap());

    //     domain_participant.enable().unwrap();

    //     // let builtin_communication_task = tasks
    //     //     .iter_mut()
    //     //     .find(|x| x.name == "builtin communication")
    //     //     .unwrap();

    //     // (builtin_communication_task.task)()
    // }
}
