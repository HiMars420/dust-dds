use std::{
    sync::atomic::{AtomicU8, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use dds_api::{
    builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    dcps_psm::{BuiltInTopicKey, DomainId, InstanceHandle, StatusMask, Time},
    domain::{
        domain_participant::{DomainParticipant, DomainParticipantTopicFactory},
        domain_participant_listener::DomainParticipantListener,
    },
    infrastructure::{
        entity::Entity,
        qos::{DomainParticipantQos, PublisherQos, SubscriberQos, TopicQos},
    },
    publication::{
        data_writer::DataWriter,
        publisher::{Publisher, PublisherDataWriterFactory},
    },
    return_type::{DdsError, DdsResult},
    subscription::subscriber::Subscriber,
    topic::topic_description::TopicDescription,
};
use rtps_pim::{
    messages::types::Count,
    structure::{
        entity::RtpsEntityAttributes,
        group::RtpsGroupConstructor,
        participant::RtpsParticipantConstructor,
        types::{
            EntityId, Guid, GuidPrefix, Locator, ENTITYID_PARTICIPANT, PROTOCOLVERSION,
            USER_DEFINED_READER_GROUP, USER_DEFINED_WRITER_GROUP, VENDOR_ID_S2E,
        },
    },
};

use crate::{
    data_representation_builtin_endpoints::discovered_topic_data::{
        DiscoveredTopicData, DCPS_TOPIC,
    },
    dds_type::DdsType,
    utils::{
        rtps_structure::RtpsStructure,
        shared_object::{DdsRwLock, DdsShared},
    },
};

use super::{
    domain_participant_proxy::DomainParticipantProxy, publisher_attributes::PublisherAttributes,
    publisher_proxy::PublisherProxy, subscriber_attributes::SubscriberAttributes,
    topic_attributes::TopicAttributes,
};

pub struct DomainParticipantAttributes<Rtps>
where
    Rtps: RtpsStructure,
{
    pub rtps_participant: Rtps::Participant,
    pub domain_id: DomainId,
    pub domain_tag: String,
    pub qos: DomainParticipantQos,
    pub builtin_subscriber: DdsRwLock<Option<DdsShared<SubscriberAttributes<Rtps>>>>,
    pub builtin_publisher: DdsRwLock<Option<DdsShared<PublisherAttributes<Rtps>>>>,
    pub user_defined_subscriber_list: DdsRwLock<Vec<DdsShared<SubscriberAttributes<Rtps>>>>,
    pub user_defined_subscriber_counter: AtomicU8,
    pub default_subscriber_qos: SubscriberQos,
    pub user_defined_publisher_list: DdsRwLock<Vec<DdsShared<PublisherAttributes<Rtps>>>>,
    pub user_defined_publisher_counter: AtomicU8,
    pub default_publisher_qos: PublisherQos,
    pub topic_list: DdsRwLock<Vec<DdsShared<TopicAttributes<Rtps>>>>,
    pub default_topic_qos: TopicQos,
    pub manual_liveliness_count: Count,
    pub lease_duration: rtps_pim::behavior::types::Duration,
    pub metatraffic_unicast_locator_list: Vec<Locator>,
    pub metatraffic_multicast_locator_list: Vec<Locator>,
    pub enabled: DdsRwLock<bool>,
}

impl<Rtps> DomainParticipantAttributes<Rtps>
where
    Rtps: RtpsStructure,
{
    pub fn new(
        guid_prefix: GuidPrefix,
        domain_id: DomainId,
        domain_tag: String,
        domain_participant_qos: DomainParticipantQos,
        metatraffic_unicast_locator_list: Vec<Locator>,
        metatraffic_multicast_locator_list: Vec<Locator>,
        default_unicast_locator_list: Vec<Locator>,
        default_multicast_locator_list: Vec<Locator>,
    ) -> Self {
        let lease_duration = rtps_pim::behavior::types::Duration::new(100, 0);
        let protocol_version = PROTOCOLVERSION;
        let vendor_id = VENDOR_ID_S2E;
        let rtps_participant = Rtps::Participant::new(
            Guid::new(guid_prefix, ENTITYID_PARTICIPANT),
            &default_unicast_locator_list,
            &default_multicast_locator_list,
            protocol_version,
            vendor_id,
        );

        Self {
            rtps_participant,
            domain_id,
            domain_tag,
            qos: domain_participant_qos,
            builtin_subscriber: DdsRwLock::new(None),
            builtin_publisher: DdsRwLock::new(None),
            user_defined_subscriber_list: DdsRwLock::new(Vec::new()),
            user_defined_subscriber_counter: AtomicU8::new(0),
            default_subscriber_qos: SubscriberQos::default(),
            user_defined_publisher_list: DdsRwLock::new(Vec::new()),
            user_defined_publisher_counter: AtomicU8::new(0),
            default_publisher_qos: PublisherQos::default(),
            topic_list: DdsRwLock::new(Vec::new()),
            default_topic_qos: TopicQos::default(),
            manual_liveliness_count: Count(0),
            lease_duration,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            enabled: DdsRwLock::new(false),
        }
    }
}

impl<Rtps, Foo> DomainParticipantTopicFactory<Foo> for DdsShared<DomainParticipantAttributes<Rtps>>
where
    Rtps: RtpsStructure,
    Foo: DdsType,
{
    type TopicType = DdsShared<TopicAttributes<Rtps>>;

    fn topic_factory_create_topic(
        &self,
        topic_name: &str,
        qos: Option<TopicQos>,
        _a_listener: Option<<Self::TopicType as Entity>::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<Self::TopicType>
    where
        Self::TopicType: Entity,
    {
        let qos = qos.unwrap_or(self.default_topic_qos.clone());

        // /////// Create topic
        let topic_shared = DdsShared::new(TopicAttributes::new(
            qos.clone(),
            Foo::type_name(),
            topic_name,
            self.downgrade(),
        ));

        self.topic_list.write_lock().push(topic_shared.clone());

        // /////// Announce the topic creation
        {
            let domain_participant_proxy = DomainParticipantProxy::new(self.downgrade());
            let builtin_publisher_option = self.builtin_publisher.read_lock().clone();
            if let Some(builtin_publisher) = builtin_publisher_option {
                let builtin_publisher_proxy = PublisherProxy::new(builtin_publisher.downgrade());

                if let Ok(topic_creation_topic) =
                    domain_participant_proxy.topic_factory_lookup_topicdescription(DCPS_TOPIC)
                {
                    if let Ok(sedp_builtin_topic_announcer) = builtin_publisher_proxy
                        .datawriter_factory_lookup_datawriter(&topic_creation_topic)
                    {
                        let sedp_discovered_topic_data = DiscoveredTopicData {
                            topic_builtin_topic_data: TopicBuiltinTopicData {
                                key: BuiltInTopicKey { value: [1; 16] },
                                name: topic_name.to_string(),
                                type_name: Foo::type_name().to_string(),
                                durability: qos.durability.clone(),
                                durability_service: qos.durability_service.clone(),
                                deadline: qos.deadline.clone(),
                                latency_budget: qos.latency_budget.clone(),
                                liveliness: qos.liveliness.clone(),
                                reliability: qos.reliability.clone(),
                                transport_priority: qos.transport_priority.clone(),
                                lifespan: qos.lifespan.clone(),
                                destination_order: qos.destination_order.clone(),
                                history: qos.history.clone(),
                                resource_limits: qos.resource_limits.clone(),
                                ownership: qos.ownership.clone(),
                                topic_data: qos.topic_data.clone(),
                            },
                        };

                        sedp_builtin_topic_announcer
                            .write(&sedp_discovered_topic_data, None)
                            .unwrap();
                    }
                }
            }
        }

        Ok(topic_shared)
    }

    fn topic_factory_delete_topic(&self, a_topic: &Self::TopicType) -> DdsResult<()> {
        let mut topic_list = self.topic_list.write_lock();
        let topic_list_position = topic_list.iter().position(|topic| topic == a_topic).ok_or(
            DdsError::PreconditionNotMet(
                "Topic can only be deleted from its parent publisher".to_string(),
            ),
        )?;
        topic_list.remove(topic_list_position);

        Ok(())
    }

    fn topic_factory_find_topic(
        &self,
        topic_name: &str,
        _timeout: dds_api::dcps_psm::Duration,
    ) -> DdsResult<Self::TopicType> {
        self.topic_list
            .read_lock()
            .iter()
            .find_map(|topic| {
                if topic.get_name().unwrap() == topic_name
                    && topic.get_type_name().unwrap() == Foo::type_name()
                {
                    Some(topic.clone())
                } else {
                    None
                }
            })
            .ok_or(DdsError::PreconditionNotMet("Not found".to_string()))
    }

    fn topic_factory_lookup_topicdescription(
        &self,
        topic_name: &str,
    ) -> DdsResult<Self::TopicType> {
        self.topic_list
            .read_lock()
            .iter()
            .find_map(|topic| {
                if topic.get_name().unwrap() == topic_name
                    && topic.get_type_name().unwrap() == Foo::type_name()
                {
                    Some(topic.clone())
                } else {
                    None
                }
            })
            .ok_or(DdsError::PreconditionNotMet("Not found".to_string()))
    }
}

impl<Rtps> DomainParticipant for DdsShared<DomainParticipantAttributes<Rtps>>
where
    Rtps: RtpsStructure,
{
    type PublisherType = DdsShared<PublisherAttributes<Rtps>>;
    type SubscriberType = DdsShared<SubscriberAttributes<Rtps>>;

    fn create_publisher(
        &self,
        qos: Option<PublisherQos>,
        _a_listener: Option<<Self::PublisherType as Entity>::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<Self::PublisherType>
    where
        Self::PublisherType: Entity,
    {
        let publisher_qos = qos.unwrap_or(self.default_publisher_qos.clone());
        let publisher_counter = self
            .user_defined_publisher_counter
            .fetch_add(1, Ordering::Relaxed);
        let entity_id = EntityId::new([publisher_counter, 0, 0], USER_DEFINED_WRITER_GROUP);
        let guid = Guid::new(self.rtps_participant.guid().prefix(), entity_id);
        let rtps_group = Rtps::Group::new(guid);
        let publisher_impl = PublisherAttributes::new(publisher_qos, rtps_group, self.downgrade());
        let publisher_impl_shared = DdsShared::new(publisher_impl);
        self.user_defined_publisher_list
            .write_lock()
            .push(publisher_impl_shared.clone());

        Ok(publisher_impl_shared)
    }

    fn delete_publisher(&self, a_publisher: &Self::PublisherType) -> DdsResult<()> {
        if std::ptr::eq(&a_publisher.get_participant()?.upgrade()?, self) {
            self.user_defined_publisher_list
                .write_lock()
                .retain(|x| x != a_publisher);
            Ok(())
        } else {
            Err(DdsError::PreconditionNotMet(
                "Publisher can only be deleted from its parent participant".to_string(),
            ))
        }
    }

    fn create_subscriber(
        &self,
        qos: Option<SubscriberQos>,
        _a_listener: Option<<Self::SubscriberType as Entity>::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<Self::SubscriberType>
    where
        Self::SubscriberType: Entity,
    {
        let subscriber_qos = qos.unwrap_or(self.default_subscriber_qos.clone());
        let subcriber_counter = self
            .user_defined_subscriber_counter
            .fetch_add(1, Ordering::Relaxed);
        let entity_id = EntityId::new([subcriber_counter, 0, 0], USER_DEFINED_READER_GROUP);
        let guid = Guid::new(self.rtps_participant.guid().prefix(), entity_id);
        let rtps_group = Rtps::Group::new(guid);
        let subscriber = SubscriberAttributes::new(subscriber_qos, rtps_group, self.downgrade());
        let subscriber_shared = DdsShared::new(subscriber);
        self.user_defined_subscriber_list
            .write_lock()
            .push(subscriber_shared.clone());

        Ok(subscriber_shared)
    }

    fn delete_subscriber(&self, a_subscriber: &Self::SubscriberType) -> DdsResult<()> {
        if std::ptr::eq(&a_subscriber.get_participant()?.upgrade()?, self) {
            self.user_defined_subscriber_list
                .write_lock()
                .retain(|x| x != a_subscriber);
            Ok(())
        } else {
            Err(DdsError::PreconditionNotMet(
                "Subscriber can only be deleted from its parent participant".to_string(),
            ))
        }
    }

    fn get_builtin_subscriber(&self) -> DdsResult<Self::SubscriberType> {
        Ok(self
            .builtin_subscriber
            .read_lock()
            .as_ref()
            .unwrap()
            .clone())
    }

    fn ignore_participant(&self, _handle: InstanceHandle) -> DdsResult<()> {
        todo!()
    }

    fn ignore_topic(&self, _handle: InstanceHandle) -> DdsResult<()> {
        todo!()
    }

    fn ignore_publication(&self, _handle: InstanceHandle) -> DdsResult<()> {
        todo!()
    }

    fn ignore_subscription(&self, _handle: InstanceHandle) -> DdsResult<()> {
        todo!()
    }

    fn get_domain_id(&self) -> DdsResult<DomainId> {
        todo!()
    }

    fn delete_contained_entities(&self) -> DdsResult<()> {
        todo!()
    }

    fn assert_liveliness(&self) -> DdsResult<()> {
        todo!()
    }

    fn set_default_publisher_qos(&self, _qos: Option<PublisherQos>) -> DdsResult<()> {
        todo!()
    }

    fn get_default_publisher_qos(&self) -> DdsResult<PublisherQos> {
        todo!()
    }

    fn set_default_subscriber_qos(&self, _qos: Option<SubscriberQos>) -> DdsResult<()> {
        todo!()
    }

    fn get_default_subscriber_qos(&self) -> DdsResult<SubscriberQos> {
        todo!()
    }

    fn set_default_topic_qos(&self, _qos: Option<TopicQos>) -> DdsResult<()> {
        todo!()
    }

    fn get_default_topic_qos(&self) -> DdsResult<TopicQos> {
        todo!()
    }

    fn get_discovered_participants(&self) -> DdsResult<Vec<InstanceHandle>> {
        todo!()
    }

    fn get_discovered_participant_data(
        &self,
        _participant_data: ParticipantBuiltinTopicData,
        _participant_handle: InstanceHandle,
    ) -> DdsResult<()> {
        todo!()
    }

    fn get_discovered_topics(&self, _topic_handles: &mut [InstanceHandle]) -> DdsResult<()> {
        todo!()
    }

    fn get_discovered_topic_data(
        &self,
        _topic_data: TopicBuiltinTopicData,
        _topic_handle: InstanceHandle,
    ) -> DdsResult<()> {
        todo!()
    }

    fn contains_entity(&self, _a_handle: InstanceHandle) -> DdsResult<bool> {
        todo!()
    }

    fn get_current_time(&self) -> DdsResult<dds_api::dcps_psm::Time> {
        let now_system_time = SystemTime::now();
        match now_system_time.duration_since(UNIX_EPOCH) {
            Ok(unix_time) => Ok(Time {
                sec: unix_time.as_secs() as i32,
                nanosec: unix_time.subsec_nanos(),
            }),
            Err(_) => Err(DdsError::Error),
        }
    }
}

impl<Rtps> Entity for DdsShared<DomainParticipantAttributes<Rtps>>
where
    Rtps: RtpsStructure,
{
    type Qos = DomainParticipantQos;
    type Listener = Box<dyn DomainParticipantListener>;

    fn set_qos(&self, _qos: Option<Self::Qos>) -> DdsResult<()> {
        todo!()
    }

    fn get_qos(&self) -> DdsResult<Self::Qos> {
        todo!()
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

    fn get_statuscondition(&self) -> DdsResult<dds_api::infrastructure::entity::StatusCondition> {
        todo!()
    }

    fn get_status_changes(&self) -> DdsResult<StatusMask> {
        todo!()
    }

    fn enable(&self) -> DdsResult<()> {
        *self.enabled.write_lock() = true;
        Ok(())
    }

    fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    use dds_api::{
        domain::domain_participant::DomainParticipantTopicFactory,
        return_type::{DdsError, DdsResult},
    };

    use super::*;
    use crate::{
        dds_impl::{domain_participant_proxy::DomainParticipantProxy, topic_proxy::TopicProxy},
        dds_type::{DdsSerialize, DdsType, Endianness},
        test_utils::mock_rtps::MockRtps,
    };
    use std::io::Write;

    macro_rules! make_empty_dds_type {
        ($type_name:ident) => {
            struct $type_name {}

            impl DdsSerialize for $type_name {
                fn serialize<W: Write, E: Endianness>(&self, _writer: W) -> DdsResult<()> {
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

    #[test]
    fn topic_factory_create_topic() {
        type Topic = TopicProxy<Foo, MockRtps>;

        let domain_participant = DdsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([1; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));
        let domain_participant_proxy = DomainParticipantProxy::new(domain_participant.downgrade());

        let len_before = domain_participant.topic_list.read_lock().len();

        let topic = domain_participant_proxy.topic_factory_create_topic("topic", None, None, 0)
            as DdsResult<Topic>;

        assert!(topic.is_ok());
        assert_eq!(
            len_before + 1,
            domain_participant.topic_list.read_lock().len()
        );
    }

    #[test]
    fn topic_factory_delete_topic() {
        type Topic = TopicProxy<Foo, MockRtps>;

        let domain_participant = DdsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([1; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));
        let domain_participant_proxy = DomainParticipantProxy::new(domain_participant.downgrade());

        let len_before = domain_participant.topic_list.read_lock().len();

        let topic = domain_participant_proxy
            .topic_factory_create_topic("topic", None, None, 0)
            .unwrap() as Topic;

        assert_eq!(
            len_before + 1,
            domain_participant.topic_list.read_lock().len()
        );

        domain_participant_proxy
            .topic_factory_delete_topic(&topic)
            .unwrap();

        assert_eq!(len_before, domain_participant.topic_list.read_lock().len());
        assert!(topic.as_ref().upgrade().is_err());
    }

    #[test]
    fn topic_factory_delete_topic_from_other_participant() {
        type Topic = TopicProxy<Foo, MockRtps>;

        let domain_participant = DdsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([1; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));
        let domain_participant_proxy = DomainParticipantProxy::new(domain_participant.downgrade());

        let domain_participant2 = DdsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([1; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));
        let domain_participant2_proxy =
            DomainParticipantProxy::new(domain_participant2.downgrade());

        let len_before = domain_participant.topic_list.read_lock().len();
        let len_before2 = domain_participant2.topic_list.read_lock().len();

        let topic = domain_participant_proxy
            .topic_factory_create_topic("topic", None, None, 0)
            .unwrap() as Topic;

        assert_eq!(
            len_before + 1,
            domain_participant.topic_list.read_lock().len()
        );
        assert_eq!(
            len_before2,
            domain_participant2.topic_list.read_lock().len()
        );

        assert!(matches!(
            domain_participant2_proxy.topic_factory_delete_topic(&topic),
            Err(DdsError::PreconditionNotMet(_))
        ));
        assert!(topic.as_ref().upgrade().is_ok());
    }

    #[test]
    fn topic_factory_lookup_topic_with_no_topic() {
        type Topic = TopicProxy<Foo, MockRtps>;

        let domain_participant = DdsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([1; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));
        let domain_participant_proxy = DomainParticipantProxy::new(domain_participant.downgrade());

        assert!(
            (domain_participant_proxy.topic_factory_lookup_topicdescription("topic")
                as DdsResult<Topic>)
                .is_err()
        );
    }

    #[test]
    fn topic_factory_lookup_topic_with_one_topic() {
        type Topic = TopicProxy<Foo, MockRtps>;

        let domain_participant = DdsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([1; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));
        let domain_participant_proxy = DomainParticipantProxy::new(domain_participant.downgrade());

        let topic = domain_participant_proxy
            .topic_factory_create_topic("topic", None, None, 0)
            .unwrap() as Topic;

        assert!(
            (domain_participant_proxy.topic_factory_lookup_topicdescription("topic")
                as DdsResult<Topic>)
                .unwrap()
                .as_ref()
                .upgrade()
                .unwrap()
                == topic.as_ref().upgrade().unwrap()
        );
    }

    make_empty_dds_type!(Bar);

    #[test]
    fn topic_factory_lookup_topic_with_one_topic_with_wrong_type() {
        type TopicFoo = TopicProxy<Foo, MockRtps>;
        type TopicBar = TopicProxy<Bar, MockRtps>;

        let domain_participant = DdsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([1; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));
        let domain_participant_proxy = DomainParticipantProxy::new(domain_participant.downgrade());

        domain_participant_proxy
            .topic_factory_create_topic("topic", None, None, 0)
            .unwrap() as TopicBar;

        assert!(
            (domain_participant_proxy.topic_factory_lookup_topicdescription("topic")
                as DdsResult<TopicFoo>)
                .is_err()
        );
    }

    #[test]
    fn topic_factory_lookup_topic_with_one_topic_with_wrong_name() {
        type Topic = TopicProxy<Foo, MockRtps>;

        let domain_participant = DdsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([1; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));
        let domain_participant_proxy = DomainParticipantProxy::new(domain_participant.downgrade());

        domain_participant_proxy
            .topic_factory_create_topic("other_topic", None, None, 0)
            .unwrap() as Topic;

        assert!(
            (domain_participant_proxy.topic_factory_lookup_topicdescription("topic")
                as DdsResult<Topic>)
                .is_err()
        );
    }

    #[test]
    fn topic_factory_lookup_topic_with_two_types() {
        type TopicFoo = TopicProxy<Foo, MockRtps>;
        type TopicBar = TopicProxy<Bar, MockRtps>;

        let domain_participant = DdsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([1; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));
        let domain_participant_proxy = DomainParticipantProxy::new(domain_participant.downgrade());

        let topic_foo = domain_participant_proxy
            .topic_factory_create_topic("topic", None, None, 0)
            .unwrap() as TopicFoo;
        let topic_bar = domain_participant_proxy
            .topic_factory_create_topic("topic", None, None, 0)
            .unwrap() as TopicBar;

        assert!(
            (domain_participant_proxy.topic_factory_lookup_topicdescription("topic")
                as DdsResult<TopicFoo>)
                .unwrap()
                .as_ref()
                .upgrade()
                .unwrap()
                == topic_foo.as_ref().upgrade().unwrap()
        );

        assert!(
            (domain_participant_proxy.topic_factory_lookup_topicdescription("topic")
                as DdsResult<TopicBar>)
                .unwrap()
                .as_ref()
                .upgrade()
                .unwrap()
                == topic_bar.as_ref().upgrade().unwrap()
        );
    }

    #[test]
    fn topic_factory_lookup_topic_with_two_topics() {
        type Topic = TopicProxy<Foo, MockRtps>;

        let domain_participant = DdsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([1; 12]),
            DomainId::default(),
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));
        let domain_participant_proxy = DomainParticipantProxy::new(domain_participant.downgrade());

        let topic1 = domain_participant_proxy
            .topic_factory_create_topic("topic1", None, None, 0)
            .unwrap() as Topic;
        let topic2 = domain_participant_proxy
            .topic_factory_create_topic("topic2", None, None, 0)
            .unwrap() as Topic;

        assert!(
            (domain_participant_proxy.topic_factory_lookup_topicdescription("topic1")
                as DdsResult<Topic>)
                .unwrap()
                .as_ref()
                .upgrade()
                .unwrap()
                == topic1.as_ref().upgrade().unwrap()
        );

        assert!(
            (domain_participant_proxy.topic_factory_lookup_topicdescription("topic2")
                as DdsResult<Topic>)
                .unwrap()
                .as_ref()
                .upgrade()
                .unwrap()
                == topic2.as_ref().upgrade().unwrap()
        );
    }
}
