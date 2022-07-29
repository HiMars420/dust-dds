use std::{
    collections::HashMap,
    sync::atomic::{AtomicU8, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    dcps_psm::{Duration, DURATION_ZERO},
    dds_type::DdsType,
    implementation::rtps::{
        discovery_types::{BuiltinEndpointQos, BuiltinEndpointSet},
        stateful_reader::RtpsStatefulReaderImpl,
        stateful_writer::RtpsStatefulWriterImpl,
        stateless_reader::RtpsStatelessReaderImpl,
        types::{
            Count, EntityId, EntityKind, Guid, GuidPrefix, ProtocolVersion, ReliabilityKind,
            TopicKind, VendorId, BUILT_IN_READER_GROUP, BUILT_IN_READER_WITH_KEY,
            BUILT_IN_WRITER_GROUP, BUILT_IN_WRITER_WITH_KEY, ENTITYID_PARTICIPANT, PROTOCOLVERSION,
            USER_DEFINED_READER_GROUP, USER_DEFINED_WRITER_GROUP, VENDOR_ID_S2E,
        },
    },
    return_type::DdsResult,
    {
        builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
        dcps_psm::{
            BuiltInTopicKey, DomainId, InstanceHandle, StatusMask, Time, ANY_INSTANCE_STATE,
            ANY_SAMPLE_STATE, ANY_VIEW_STATE,
        },
        infrastructure::{
            entity::Entity,
            qos::{
                DataReaderQos, DataWriterQos, DomainParticipantQos, PublisherQos, SubscriberQos,
                TopicQos,
            },
            qos_policy::{HistoryQosPolicy, HistoryQosPolicyKind},
        },
    },
};
use crate::{
    implementation::{
        rtps::{
            group::RtpsGroupImpl,
            participant::RtpsParticipantImpl,
            stateless_writer::{RtpsReaderLocatorAttributesImpl, RtpsStatelessWriterImpl},
        },
        utils::rtps_communication_traits::SendRtpsMessage,
    },
    return_type::DdsError,
};
use dds_transport::types::Locator;

use crate::implementation::{
    data_representation_builtin_endpoints::{
        discovered_reader_data::{DiscoveredReaderData, DCPS_SUBSCRIPTION},
        discovered_topic_data::{DiscoveredTopicData, DCPS_TOPIC},
        discovered_writer_data::{DiscoveredWriterData, DCPS_PUBLICATION},
        spdp_discovered_participant_data::{
            ParticipantProxy, SpdpDiscoveredParticipantData, DCPS_PARTICIPANT,
        },
    },
    utils::{
        discovery_traits::{AddMatchedReader, AddMatchedWriter},
        shared_object::{DdsRwLock, DdsShared},
    },
};

use super::{
    data_reader_impl::{DataReaderImpl, RtpsReader},
    data_writer_impl::{DataWriterImpl, RtpsWriter},
    message_receiver::MessageReceiver,
    participant_discovery::ParticipantDiscovery,
    publisher_impl::{AddDataWriter, PublisherEmpty, PublisherImpl},
    subscriber_impl::{AddDataReader, SubscriberEmpty, SubscriberImpl},
    topic_impl::TopicImpl,
};

use crate::domain::domain_participant_listener::DomainParticipantListener;

use dds_transport::{TransportRead, TransportWrite};

pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER: EntityId =
    EntityId::new([0x00, 0x01, 0x00], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER: EntityId =
    EntityId::new([0x00, 0x01, 0x00], BUILT_IN_READER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER: EntityId =
    EntityId::new([0, 0, 0x02], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR: EntityId =
    EntityId::new([0, 0, 0x02], BUILT_IN_READER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER: EntityId =
    EntityId::new([0, 0, 0x03], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR: EntityId =
    EntityId::new([0, 0, 0x03], BUILT_IN_READER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER: EntityId =
    EntityId::new([0, 0, 0x04], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR: EntityId =
    EntityId::new([0, 0, 0x04], BUILT_IN_READER_WITH_KEY);

const DEFAULT_HEARTBEAT_PERIOD: Duration = Duration::new(2, 0);

const DEFAULT_NACK_RESPONSE_DELAY: Duration = Duration::new(0, 200);

const DEFAULT_NACK_SUPPRESSION_DURATION: Duration = DURATION_ZERO;

const DEFAULT_HEARTBEAT_RESPONSE_DELAY: Duration = Duration::new(0, 500);

const DEFAULT_HEARTBEAT_SUPPRESSION_DURATION: Duration = DURATION_ZERO;

pub const USER_DEFINED_TOPIC: EntityKind = 0x0a;

pub struct DomainParticipantImpl {
    rtps_participant: RtpsParticipantImpl,
    domain_id: DomainId,
    domain_tag: String,
    qos: DdsRwLock<DomainParticipantQos>,
    builtin_subscriber: DdsRwLock<Option<DdsShared<SubscriberImpl>>>,
    builtin_publisher: DdsRwLock<Option<DdsShared<PublisherImpl>>>,
    user_defined_subscriber_list: DdsRwLock<Vec<DdsShared<SubscriberImpl>>>,
    user_defined_subscriber_counter: AtomicU8,
    default_subscriber_qos: SubscriberQos,
    user_defined_publisher_list: DdsRwLock<Vec<DdsShared<PublisherImpl>>>,
    user_defined_publisher_counter: AtomicU8,
    default_publisher_qos: PublisherQos,
    topic_list: DdsRwLock<Vec<DdsShared<TopicImpl>>>,
    user_defined_topic_counter: AtomicU8,
    default_topic_qos: TopicQos,
    manual_liveliness_count: Count,
    lease_duration: Duration,
    metatraffic_unicast_locator_list: Vec<Locator>,
    metatraffic_multicast_locator_list: Vec<Locator>,
    discovered_participant_list: DdsRwLock<HashMap<InstanceHandle, ParticipantBuiltinTopicData>>,
    enabled: DdsRwLock<bool>,
}

impl DomainParticipantImpl {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        guid_prefix: GuidPrefix,
        domain_id: DomainId,
        domain_tag: String,
        domain_participant_qos: DomainParticipantQos,
        metatraffic_unicast_locator_list: Vec<Locator>,
        metatraffic_multicast_locator_list: Vec<Locator>,
        default_unicast_locator_list: Vec<Locator>,
        default_multicast_locator_list: Vec<Locator>,
    ) -> DdsShared<Self> {
        let lease_duration = Duration::new(100, 0);
        let protocol_version = PROTOCOLVERSION;
        let vendor_id = VENDOR_ID_S2E;
        let rtps_participant = RtpsParticipantImpl::new(
            Guid::new(guid_prefix, ENTITYID_PARTICIPANT),
            &default_unicast_locator_list,
            &default_multicast_locator_list,
            protocol_version,
            vendor_id,
        );

        DdsShared::new(DomainParticipantImpl {
            rtps_participant,
            domain_id,
            domain_tag,
            qos: DdsRwLock::new(domain_participant_qos),
            builtin_subscriber: DdsRwLock::new(None),
            builtin_publisher: DdsRwLock::new(None),
            user_defined_subscriber_list: DdsRwLock::new(Vec::new()),
            user_defined_subscriber_counter: AtomicU8::new(0),
            default_subscriber_qos: SubscriberQos::default(),
            user_defined_publisher_list: DdsRwLock::new(Vec::new()),
            user_defined_publisher_counter: AtomicU8::new(0),
            default_publisher_qos: PublisherQos::default(),
            topic_list: DdsRwLock::new(Vec::new()),
            user_defined_topic_counter: AtomicU8::new(0),
            default_topic_qos: TopicQos::default(),
            manual_liveliness_count: Count(0),
            lease_duration,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            discovered_participant_list: DdsRwLock::new(HashMap::new()),
            enabled: DdsRwLock::new(false),
        })
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read_lock()
    }
}

pub trait AnnounceTopic {
    fn announce_topic(&self, sedp_discovered_topic_data: DiscoveredTopicData);
}

impl AnnounceTopic for DdsShared<DomainParticipantImpl> {
    fn announce_topic(&self, sedp_discovered_topic_data: DiscoveredTopicData) {
        let builtin_publisher_option = self.builtin_publisher.read_lock().clone();
        if let Some(builtin_publisher) = builtin_publisher_option {
            if let Ok(topic_creation_topic) =
                self.lookup_topicdescription::<DiscoveredTopicData>(DCPS_TOPIC)
            {
                if let Ok(sedp_builtin_topic_announcer) = builtin_publisher
                    .lookup_datawriter::<DiscoveredTopicData>(&topic_creation_topic)
                {
                    sedp_builtin_topic_announcer
                        .write(&sedp_discovered_topic_data, None)
                        .unwrap();
                }
            }
        }
    }
}

impl DdsShared<DomainParticipantImpl> {
    pub fn create_publisher(
        &self,
        qos: Option<PublisherQos>,
        _a_listener: Option<<DdsShared<PublisherImpl> as Entity>::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<DdsShared<PublisherImpl>> {
        let publisher_qos = qos.unwrap_or_else(|| self.default_publisher_qos.clone());
        let publisher_counter = self
            .user_defined_publisher_counter
            .fetch_add(1, Ordering::Relaxed);
        let entity_id = EntityId::new([publisher_counter, 0, 0], USER_DEFINED_WRITER_GROUP);
        let guid = Guid::new(self.rtps_participant.guid().prefix(), entity_id);
        let rtps_group = RtpsGroupImpl::new(guid);
        let publisher_impl_shared = PublisherImpl::new(publisher_qos, rtps_group, self.downgrade());
        if *self.enabled.read_lock()
            && self
                .qos
                .read_lock()
                .entity_factory
                .autoenable_created_entities
        {
            publisher_impl_shared.enable()?;
        }

        self.user_defined_publisher_list
            .write_lock()
            .push(publisher_impl_shared.clone());

        Ok(publisher_impl_shared)
    }

    pub fn delete_publisher(&self, a_publisher: &DdsShared<PublisherImpl>) -> DdsResult<()> {
        if !DdsShared::ptr_eq(&a_publisher.get_participant()?, self) {
            return Err(DdsError::PreconditionNotMet(
                "Publisher can only be deleted from its parent participant".to_string(),
            ));
        }

        if !a_publisher.is_empty() {
            return Err(DdsError::PreconditionNotMet(
                "Publisher still contains data writers".to_string(),
            ));
        }

        self.user_defined_publisher_list
            .write_lock()
            .retain(|x| x != a_publisher);

        Ok(())
    }

    pub fn create_subscriber(
        &self,
        qos: Option<SubscriberQos>,
        _a_listener: Option<<DdsShared<SubscriberImpl> as Entity>::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<DdsShared<SubscriberImpl>> {
        let subscriber_qos = qos.unwrap_or_else(|| self.default_subscriber_qos.clone());
        let subcriber_counter = self
            .user_defined_subscriber_counter
            .fetch_add(1, Ordering::Relaxed);
        let entity_id = EntityId::new([subcriber_counter, 0, 0], USER_DEFINED_READER_GROUP);
        let guid = Guid::new(self.rtps_participant.guid().prefix(), entity_id);
        let rtps_group = RtpsGroupImpl::new(guid);
        let subscriber_shared = SubscriberImpl::new(subscriber_qos, rtps_group, self.downgrade());
        if *self.enabled.read_lock()
            && self
                .qos
                .read_lock()
                .entity_factory
                .autoenable_created_entities
        {
            subscriber_shared.enable()?;
        }

        self.user_defined_subscriber_list
            .write_lock()
            .push(subscriber_shared.clone());

        Ok(subscriber_shared)
    }

    pub fn delete_subscriber(&self, a_subscriber: &DdsShared<SubscriberImpl>) -> DdsResult<()> {
        if !DdsShared::ptr_eq(&a_subscriber.get_participant()?, self) {
            return Err(DdsError::PreconditionNotMet(
                "Subscriber can only be deleted from its parent participant".to_string(),
            ));
        }

        if !a_subscriber.is_empty() {
            return Err(DdsError::PreconditionNotMet(
                "Subscriber still contains data readers".to_string(),
            ));
        }

        self.user_defined_subscriber_list
            .write_lock()
            .retain(|x| x != a_subscriber);
        Ok(())
    }

    pub fn create_topic<Foo>(
        &self,
        topic_name: &str,
        qos: Option<TopicQos>,
        _a_listener: Option<<DdsShared<TopicImpl> as Entity>::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<DdsShared<TopicImpl>>
    where
        Foo: DdsType,
    {
        let topic_counter = self
            .user_defined_topic_counter
            .fetch_add(1, Ordering::Relaxed);
        let topic_guid = Guid::new(
            self.rtps_participant.guid().prefix(),
            EntityId {
                entity_key: [topic_counter, 0, 0],
                entity_kind: USER_DEFINED_TOPIC,
            },
        );
        let qos = qos.unwrap_or_else(|| self.default_topic_qos.clone());

        // /////// Create topic
        let topic_shared = TopicImpl::new(
            topic_guid,
            qos,
            Foo::type_name(),
            topic_name,
            self.downgrade(),
        );
        if *self.enabled.read_lock()
            && self
                .qos
                .read_lock()
                .entity_factory
                .autoenable_created_entities
        {
            topic_shared.enable()?;
        }

        self.topic_list.write_lock().push(topic_shared.clone());

        Ok(topic_shared)
    }

    pub fn delete_topic<Foo>(&self, a_topic: &DdsShared<TopicImpl>) -> DdsResult<()> {
        let mut topic_list = self.topic_list.write_lock();
        let topic_list_position = topic_list
            .iter()
            .position(|topic| topic == a_topic)
            .ok_or_else(|| {
                DdsError::PreconditionNotMet(
                    "Topic can only be deleted from its parent publisher".to_string(),
                )
            })?;

        // If topic is not attached to any reader or writer there must be no more than 2 strong counts
        // 1 strong stored in the list of the participant and 1 strong used to call this function
        if a_topic.strong_count() > 2 {
            return Err(DdsError::PreconditionNotMet(
                "Topic still attached to some data reader or data writer".to_string(),
            ));
        }

        topic_list.remove(topic_list_position);

        Ok(())
    }

    pub fn find_topic<Foo>(
        &self,
        topic_name: &str,
        _timeout: crate::dcps_psm::Duration,
    ) -> DdsResult<DdsShared<TopicImpl>>
    where
        Foo: DdsType,
    {
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
            .ok_or_else(|| DdsError::PreconditionNotMet("Not found".to_string()))
    }

    pub fn lookup_topicdescription<Foo>(&self, topic_name: &str) -> DdsResult<DdsShared<TopicImpl>>
    where
        Foo: DdsType,
    {
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
            .ok_or_else(|| DdsError::PreconditionNotMet("Not found".to_string()))
    }

    pub fn get_builtin_subscriber(&self) -> DdsResult<DdsShared<SubscriberImpl>> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        Ok(self
            .builtin_subscriber
            .read_lock()
            .as_ref()
            .unwrap()
            .clone())
    }

    pub fn ignore_participant(&self, _handle: InstanceHandle) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn ignore_topic(&self, _handle: InstanceHandle) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn ignore_publication(&self, _handle: InstanceHandle) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn ignore_subscription(&self, _handle: InstanceHandle) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn get_domain_id(&self) -> DdsResult<DomainId> {
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

    pub fn assert_liveliness(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn set_default_publisher_qos(&self, _qos: Option<PublisherQos>) -> DdsResult<()> {
        todo!()
    }

    pub fn get_default_publisher_qos(&self) -> DdsResult<PublisherQos> {
        todo!()
    }

    pub fn set_default_subscriber_qos(&self, _qos: Option<SubscriberQos>) -> DdsResult<()> {
        todo!()
    }

    pub fn get_default_subscriber_qos(&self) -> DdsResult<SubscriberQos> {
        todo!()
    }

    pub fn set_default_topic_qos(&self, _qos: Option<TopicQos>) -> DdsResult<()> {
        todo!()
    }

    pub fn get_default_topic_qos(&self) -> DdsResult<TopicQos> {
        Ok(self.default_topic_qos.clone())
    }

    pub fn get_discovered_participants(&self) -> DdsResult<Vec<InstanceHandle>> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        Ok(self
            .discovered_participant_list
            .read_lock()
            .iter()
            .map(|(&key, _)| key)
            .collect())
    }

    pub fn get_discovered_participant_data(
        &self,
        participant_handle: InstanceHandle,
    ) -> DdsResult<ParticipantBuiltinTopicData> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        self.discovered_participant_list
            .read_lock()
            .get(&participant_handle)
            .cloned()
            .ok_or(DdsError::BadParameter)
    }

    pub fn get_discovered_topics(&self, _topic_handles: &mut [InstanceHandle]) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn get_discovered_topic_data(
        &self,
        _topic_data: TopicBuiltinTopicData,
        _topic_handle: InstanceHandle,
    ) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn contains_entity(&self, _a_handle: InstanceHandle) -> DdsResult<bool> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    pub fn get_current_time(&self) -> DdsResult<crate::dcps_psm::Time> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

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

impl Entity for DdsShared<DomainParticipantImpl> {
    type Qos = DomainParticipantQos;
    type Listener = Box<dyn DomainParticipantListener>;

    fn set_qos(&self, qos: Option<Self::Qos>) -> DdsResult<()> {
        *self.qos.write_lock() = qos.unwrap_or_default();

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

    fn get_statuscondition(&self) -> DdsResult<crate::infrastructure::entity::StatusCondition> {
        todo!()
    }

    fn get_status_changes(&self) -> DdsResult<StatusMask> {
        todo!()
    }

    fn enable(&self) -> DdsResult<()> {
        *self.enabled.write_lock() = true;

        if self
            .qos
            .read_lock()
            .entity_factory
            .autoenable_created_entities
        {
            for publisher in self.user_defined_publisher_list.read_lock().iter() {
                publisher.enable()?;
            }

            for subscriber in self.user_defined_subscriber_list.read_lock().iter() {
                subscriber.enable()?;
            }

            for topic in self.topic_list.read_lock().iter() {
                topic.enable()?;
            }
        }

        Ok(())
    }

    fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        Ok(self.rtps_participant.guid().into())
    }
}

pub trait AnnounceParticipant {
    fn announce_participant(&self) -> DdsResult<()>;
}

impl AnnounceParticipant for DdsShared<DomainParticipantImpl> {
    fn announce_participant(&self) -> DdsResult<()> {
        let dcps_topic_participant =
            self.lookup_topicdescription::<SpdpDiscoveredParticipantData>(DCPS_PARTICIPANT)?;
        let builtin_publisher = self.builtin_publisher.read_lock();

        let spdp_participant_writer =
            builtin_publisher
                .as_ref()
                .unwrap()
                .lookup_datawriter::<SpdpDiscoveredParticipantData>(&dcps_topic_participant)?;

        let spdp_discovered_participant_data = SpdpDiscoveredParticipantData {
            dds_participant_data: ParticipantBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: self.rtps_participant.guid().into(),
                },
                user_data: self.qos.read_lock().user_data.clone(),
            },
            participant_proxy: ParticipantProxy {
                domain_id: self.domain_id,
                domain_tag: self.domain_tag.clone(),
                protocol_version: self.rtps_participant.protocol_version(),
                guid_prefix: self.rtps_participant.guid().prefix(),
                vendor_id: self.rtps_participant.vendor_id(),
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
            lease_duration: self.lease_duration.into(),
        };
        spdp_participant_writer.write(&spdp_discovered_participant_data, None)
    }
}

pub trait AddDiscoveredParticipant {
    fn add_discovered_participant(
        &self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    );
}

impl AddDiscoveredParticipant for DdsShared<DomainParticipantImpl> {
    fn add_discovered_participant(
        &self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        let dcps_publication_topic = self
            .lookup_topicdescription::<DiscoveredWriterData>(DCPS_PUBLICATION)
            .unwrap();
        let dcps_subscription_topic = self
            .lookup_topicdescription::<DiscoveredReaderData>(DCPS_SUBSCRIPTION)
            .unwrap();
        let dcps_topic_topic = self
            .lookup_topicdescription::<DiscoveredTopicData>(DCPS_TOPIC)
            .unwrap();

        if let Ok(participant_discovery) = ParticipantDiscovery::new(
            discovered_participant_data,
            self.domain_id as i32,
            &self.domain_tag,
        ) {
            let builtin_publisher_lock = self.builtin_publisher.read_lock();
            let builtin_subscriber_lock = self.builtin_subscriber.read_lock();
            let builtin_publisher = builtin_publisher_lock.as_ref().unwrap();
            let builtin_subscriber = builtin_subscriber_lock.as_ref().unwrap();
            let sedp_builtin_publication_writer_shared = builtin_publisher
                .lookup_datawriter::<DiscoveredWriterData>(&dcps_publication_topic)
                .unwrap();
            sedp_builtin_publication_writer_shared.add_matched_participant(&participant_discovery);

            let sedp_builtin_publication_reader_shared = builtin_subscriber
                .lookup_datareader::<DiscoveredWriterData>(&dcps_publication_topic)
                .unwrap();
            sedp_builtin_publication_reader_shared.add_matched_participant(&participant_discovery);

            let sedp_builtin_subscription_writer_shared = builtin_publisher
                .lookup_datawriter::<DiscoveredReaderData>(&dcps_subscription_topic)
                .unwrap();
            sedp_builtin_subscription_writer_shared.add_matched_participant(&participant_discovery);

            let sedp_builtin_subscription_reader_shared = builtin_subscriber
                .lookup_datareader::<DiscoveredReaderData>(&dcps_subscription_topic)
                .unwrap();
            sedp_builtin_subscription_reader_shared.add_matched_participant(&participant_discovery);

            let sedp_builtin_topic_writer_shared = builtin_publisher
                .lookup_datawriter::<DiscoveredTopicData>(&dcps_topic_topic)
                .unwrap();
            sedp_builtin_topic_writer_shared.add_matched_participant(&participant_discovery);

            let sedp_builtin_topic_reader_shared = builtin_subscriber
                .lookup_datareader::<DiscoveredTopicData>(&dcps_topic_topic)
                .unwrap();
            sedp_builtin_topic_reader_shared.add_matched_participant(&participant_discovery);

            self.discovered_participant_list.write_lock().insert(
                discovered_participant_data.dds_participant_data.key.value,
                discovered_participant_data.dds_participant_data.clone(),
            );
        }
    }
}

impl DdsShared<DomainParticipantImpl> {
    pub fn default_unicast_locator_list(&self) -> &[Locator] {
        self.rtps_participant.default_unicast_locator_list()
    }

    pub fn default_multicast_locator_list(&self) -> &[Locator] {
        self.rtps_participant.default_multicast_locator_list()
    }

    pub fn protocol_version(&self) -> ProtocolVersion {
        self.rtps_participant.protocol_version()
    }

    pub fn vendor_id(&self) -> VendorId {
        self.rtps_participant.vendor_id()
    }
}

pub trait DataWriterDiscovery {
    fn add_created_data_writer(&self, writer_data: &DiscoveredWriterData);
}

impl DataWriterDiscovery for DdsShared<DomainParticipantImpl> {
    fn add_created_data_writer(&self, writer_data: &DiscoveredWriterData) {
        let builtin_publisher = self.builtin_publisher.read_lock();
        if let Some(builtin_publisher) = builtin_publisher.as_ref() {
            if let Ok(publication_topic) =
                self.lookup_topicdescription::<DiscoveredWriterData>(DCPS_PUBLICATION)
            {
                if let Ok(sedp_builtin_publications_announcer) =
                    builtin_publisher.lookup_datawriter::<DiscoveredWriterData>(&publication_topic)
                {
                    sedp_builtin_publications_announcer
                        .write(writer_data, None)
                        .unwrap();
                }
            }
        }
    }
}

pub trait DataReaderDiscovery {
    fn add_created_data_reader(&self, reader_data: &DiscoveredReaderData);
}

impl DataReaderDiscovery for DdsShared<DomainParticipantImpl> {
    fn add_created_data_reader(&self, reader_data: &DiscoveredReaderData) {
        let builtin_publisher = self.builtin_publisher.read_lock();
        if let Some(builtin_publisher) = builtin_publisher.as_ref() {
            if let Ok(subscription_topic) =
                self.lookup_topicdescription::<DiscoveredReaderData>(DCPS_SUBSCRIPTION)
            {
                if let Ok(sedp_builtin_subscription_announcer) =
                    builtin_publisher.lookup_datawriter::<DiscoveredReaderData>(&subscription_topic)
                {
                    sedp_builtin_subscription_announcer
                        .write(reader_data, None)
                        .unwrap();
                }
            }
        }
    }
}

pub trait SendBuiltInData {
    fn send_built_in_data(&self, transport: &mut impl TransportWrite);
}

impl SendBuiltInData for DdsShared<DomainParticipantImpl> {
    fn send_built_in_data(&self, transport: &mut impl TransportWrite) {
        let builtin_publisher = self.builtin_publisher.read_lock();
        let builtin_subscriber = self.builtin_subscriber.read_lock();
        if let (Some(builtin_publisher), Some(builtin_subscriber)) =
            (builtin_publisher.as_ref(), builtin_subscriber.as_ref())
        {
            builtin_publisher.send_message(transport);
            builtin_subscriber.send_message(transport);
        } else {
            println!("/!\\ Participant doesn't have a builtin publisher and a builtin subscriber");
        }
    }
}

pub trait ReceiveBuiltInData {
    fn receive_built_in_data(&self, transport: &mut impl for<'a> TransportRead<'a>);
}

impl ReceiveBuiltInData for DdsShared<DomainParticipantImpl> {
    fn receive_built_in_data(&self, transport: &mut impl for<'a> TransportRead<'a>) {
        let publisher_list = self.builtin_publisher.read_lock();
        let subscriber_list = self.builtin_subscriber.read_lock();
        while let Some((source_locator, message)) = transport.read() {
            MessageReceiver::new().process_message(
                self.rtps_participant.guid().prefix,
                core::slice::from_ref(publisher_list.as_ref().unwrap()),
                core::slice::from_ref(subscriber_list.as_ref().unwrap()),
                source_locator,
                &message,
            );
        }
    }
}

pub trait CreateBuiltIns {
    fn create_builtins(&self) -> DdsResult<()>;
}

impl CreateBuiltIns for DdsShared<DomainParticipantImpl> {
    fn create_builtins(&self) -> DdsResult<()> {
        let guid_prefix = self.rtps_participant.guid().prefix;
        ///////// Create the built-in publisher and subcriber

        let builtin_subscriber = SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(Guid::new(
                guid_prefix,
                EntityId::new([0, 0, 0], BUILT_IN_READER_GROUP),
            )),
            self.downgrade(),
        );
        builtin_subscriber.enable()?;

        *self.builtin_subscriber.write_lock() = Some(builtin_subscriber);

        let builtin_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(Guid::new(
                guid_prefix,
                EntityId::new([0, 0, 0], BUILT_IN_WRITER_GROUP),
            )),
            self.downgrade(),
        );
        builtin_publisher.enable()?;

        *self.builtin_publisher.write_lock() = Some(builtin_publisher);

        ///////// Create built-in DDS data readers and data writers

        let builtin_reader_qos = DataReaderQos {
            history: HistoryQosPolicy {
                kind: HistoryQosPolicyKind::KeepAllHistoryQos,
                depth: 0,
            },
            ..Default::default()
        };

        ////////// SPDP built-in topic, reader and writer
        {
            let spdp_topic_participant = self.create_topic::<SpdpDiscoveredParticipantData>(
                DCPS_PARTICIPANT,
                Some(self.get_default_topic_qos()?),
                None,
                0,
            )?;
            spdp_topic_participant.enable()?;

            let spdp_builtin_participant_reader_guid =
                Guid::new(guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER);

            let unicast_locator_list = &[];
            let multicast_locator_list = &[];
            let spdp_builtin_participant_rtps_reader = RtpsStatelessReaderImpl::new(
                spdp_builtin_participant_reader_guid,
                TopicKind::WithKey,
                ReliabilityKind::BestEffort,
                unicast_locator_list,
                multicast_locator_list,
                DURATION_ZERO,
                DURATION_ZERO,
                false,
            );

            let spdp_builtin_participant_data_reader = DataReaderImpl::new(
                builtin_reader_qos.clone(),
                RtpsReader::Stateless(spdp_builtin_participant_rtps_reader),
                spdp_topic_participant.clone(),
                None,
                self.builtin_subscriber
                    .read_lock()
                    .clone()
                    .unwrap()
                    .downgrade(),
            );
            spdp_builtin_participant_data_reader.enable()?;
            self.builtin_subscriber
                .write_lock()
                .as_ref()
                .unwrap()
                .add_data_reader(spdp_builtin_participant_data_reader);

            let spdp_reader_locators: Vec<RtpsReaderLocatorAttributesImpl> = self
                .metatraffic_multicast_locator_list
                .iter()
                .map(|&locator| RtpsReaderLocatorAttributesImpl::new(locator, false))
                .collect();

            let spdp_builtin_participant_writer_guid =
                Guid::new(guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER);
            let unicast_locator_list = &[];
            let multicast_locator_list = &[];

            let mut spdp_builtin_participant_rtps_writer = RtpsStatelessWriterImpl::new(
                spdp_builtin_participant_writer_guid,
                TopicKind::WithKey,
                ReliabilityKind::BestEffort,
                unicast_locator_list,
                multicast_locator_list,
                true,
                DURATION_ZERO,
                DURATION_ZERO,
                DURATION_ZERO,
                None,
            );

            for reader_locator in spdp_reader_locators {
                spdp_builtin_participant_rtps_writer.reader_locator_add(reader_locator);
            }

            let spdp_builtin_participant_data_writer = DataWriterImpl::new(
                DataWriterQos::default(),
                RtpsWriter::Stateless(spdp_builtin_participant_rtps_writer),
                None,
                spdp_topic_participant,
                self.builtin_publisher
                    .read_lock()
                    .clone()
                    .unwrap()
                    .downgrade(),
            );
            spdp_builtin_participant_data_writer.enable()?;
            self.builtin_publisher
                .write_lock()
                .as_ref()
                .unwrap()
                .add_data_writer(spdp_builtin_participant_data_writer);
        }

        ////////// SEDP built-in publication topic, reader and writer
        {
            let sedp_topic_publication = self.create_topic::<DiscoveredWriterData>(
                DCPS_PUBLICATION,
                Some(self.get_default_topic_qos()?),
                None,
                0,
            )?;
            sedp_topic_publication.enable()?;

            let guid = Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR);
            let topic_kind = TopicKind::WithKey;
            let reliability_level = ReliabilityKind::Reliable;
            let heartbeat_response_delay = DEFAULT_HEARTBEAT_RESPONSE_DELAY;
            let heartbeat_suppression_duration = DEFAULT_HEARTBEAT_SUPPRESSION_DURATION;
            let expects_inline_qos = false;
            let unicast_locator_list = &[];
            let multicast_locator_list = &[];
            let sedp_builtin_publications_rtps_reader = RtpsStatefulReaderImpl::new(
                guid,
                topic_kind,
                reliability_level,
                unicast_locator_list,
                multicast_locator_list,
                heartbeat_response_delay,
                heartbeat_suppression_duration,
                expects_inline_qos,
            );

            let sedp_builtin_publications_data_reader = DataReaderImpl::new(
                builtin_reader_qos.clone(),
                RtpsReader::Stateful(sedp_builtin_publications_rtps_reader),
                sedp_topic_publication.clone(),
                None,
                self.builtin_subscriber
                    .read_lock()
                    .clone()
                    .unwrap()
                    .downgrade(),
            );
            sedp_builtin_publications_data_reader.enable()?;
            self.builtin_subscriber
                .write_lock()
                .as_ref()
                .unwrap()
                .add_data_reader(sedp_builtin_publications_data_reader);

            let guid = Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER);
            let topic_kind = TopicKind::WithKey;
            let reliability_level = ReliabilityKind::Reliable;
            let push_mode = true;
            let heartbeat_period = DEFAULT_HEARTBEAT_PERIOD;
            let nack_response_delay = DEFAULT_NACK_RESPONSE_DELAY;
            let nack_suppression_duration = DEFAULT_NACK_SUPPRESSION_DURATION;
            let data_max_size_serialized = None;
            let sedp_builtin_publications_rtps_writer = RtpsStatefulWriterImpl::new(
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
            );

            let sedp_builtin_publications_data_writer = DataWriterImpl::new(
                DataWriterQos::default(),
                RtpsWriter::Stateful(sedp_builtin_publications_rtps_writer),
                None,
                sedp_topic_publication,
                self.builtin_publisher
                    .read_lock()
                    .clone()
                    .unwrap()
                    .downgrade(),
            );
            sedp_builtin_publications_data_writer.enable()?;

            self.builtin_publisher
                .write_lock()
                .as_ref()
                .unwrap()
                .add_data_writer(sedp_builtin_publications_data_writer);
        }

        ////////// SEDP built-in subcriptions topic, reader and writer
        {
            let sedp_topic_subscription = self.create_topic::<DiscoveredReaderData>(
                DCPS_SUBSCRIPTION,
                Some(self.get_default_topic_qos()?),
                None,
                0,
            )?;
            sedp_topic_subscription.enable()?;

            let guid = Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
            let topic_kind = TopicKind::WithKey;
            let reliability_level = ReliabilityKind::Reliable;
            let heartbeat_response_delay = DEFAULT_HEARTBEAT_RESPONSE_DELAY;
            let heartbeat_suppression_duration = DEFAULT_HEARTBEAT_SUPPRESSION_DURATION;
            let expects_inline_qos = false;
            let unicast_locator_list = &[];
            let multicast_locator_list = &[];
            let sedp_builtin_subscriptions_rtps_reader = RtpsStatefulReaderImpl::new(
                guid,
                topic_kind,
                reliability_level,
                unicast_locator_list,
                multicast_locator_list,
                heartbeat_response_delay,
                heartbeat_suppression_duration,
                expects_inline_qos,
            );

            let sedp_builtin_subscriptions_data_reader = DataReaderImpl::new(
                builtin_reader_qos.clone(),
                RtpsReader::Stateful(sedp_builtin_subscriptions_rtps_reader),
                sedp_topic_subscription.clone(),
                None,
                self.builtin_subscriber
                    .read_lock()
                    .clone()
                    .unwrap()
                    .downgrade(),
            );
            sedp_builtin_subscriptions_data_reader.enable()?;
            self.builtin_subscriber
                .write_lock()
                .as_ref()
                .unwrap()
                .add_data_reader(sedp_builtin_subscriptions_data_reader);

            let guid = Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
            let topic_kind = TopicKind::WithKey;
            let reliability_level = ReliabilityKind::Reliable;
            let push_mode = true;
            let heartbeat_period = DEFAULT_HEARTBEAT_PERIOD;
            let nack_response_delay = DEFAULT_NACK_RESPONSE_DELAY;
            let nack_suppression_duration = DEFAULT_NACK_SUPPRESSION_DURATION;
            let data_max_size_serialized = None;
            let unicast_locator_list = &[];
            let multicast_locator_list = &[];
            let sedp_builtin_subscriptions_rtps_writer = RtpsStatefulWriterImpl::new(
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
            );
            let sedp_builtin_subscriptions_data_writer = DataWriterImpl::new(
                DataWriterQos::default(),
                RtpsWriter::Stateful(sedp_builtin_subscriptions_rtps_writer),
                None,
                sedp_topic_subscription,
                self.builtin_publisher
                    .read_lock()
                    .clone()
                    .unwrap()
                    .downgrade(),
            );
            sedp_builtin_subscriptions_data_writer.enable()?;
            self.builtin_publisher
                .write_lock()
                .as_ref()
                .unwrap()
                .add_data_writer(sedp_builtin_subscriptions_data_writer);
        }

        ////////// SEDP built-in topics topic, reader and writer
        {
            let sedp_topic_topic = self.create_topic::<DiscoveredTopicData>(
                DCPS_TOPIC,
                Some(self.get_default_topic_qos()?),
                None,
                0,
            )?;
            sedp_topic_topic.enable()?;

            let guid = Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR);
            let topic_kind = TopicKind::WithKey;
            let reliability_level = ReliabilityKind::Reliable;
            let heartbeat_response_delay = DEFAULT_HEARTBEAT_RESPONSE_DELAY;
            let heartbeat_suppression_duration = DEFAULT_HEARTBEAT_SUPPRESSION_DURATION;
            let expects_inline_qos = false;
            let unicast_locator_list = &[];
            let multicast_locator_list = &[];
            let sedp_builtin_topics_rtps_reader = RtpsStatefulReaderImpl::new(
                guid,
                topic_kind,
                reliability_level,
                unicast_locator_list,
                multicast_locator_list,
                heartbeat_response_delay,
                heartbeat_suppression_duration,
                expects_inline_qos,
            );

            let sedp_builtin_topics_data_reader = DataReaderImpl::new(
                builtin_reader_qos,
                RtpsReader::Stateful(sedp_builtin_topics_rtps_reader),
                sedp_topic_topic.clone(),
                None,
                self.builtin_subscriber
                    .read_lock()
                    .clone()
                    .unwrap()
                    .downgrade(),
            );
            sedp_builtin_topics_data_reader.enable()?;
            self.builtin_subscriber
                .write_lock()
                .as_ref()
                .unwrap()
                .add_data_reader(sedp_builtin_topics_data_reader);

            let guid = Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER);
            let topic_kind = TopicKind::WithKey;
            let reliability_level = ReliabilityKind::Reliable;
            let push_mode = true;
            let heartbeat_period = DEFAULT_HEARTBEAT_PERIOD;
            let nack_response_delay = DEFAULT_NACK_RESPONSE_DELAY;
            let nack_suppression_duration = DEFAULT_NACK_SUPPRESSION_DURATION;
            let data_max_size_serialized = None;
            let unicast_locator_list = &[];
            let multicast_locator_list = &[];
            let sedp_builtin_topics_rtps_writer = RtpsStatefulWriterImpl::new(
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
            );

            let sedp_builtin_topics_data_writer = DataWriterImpl::new(
                DataWriterQos::default(),
                RtpsWriter::Stateful(sedp_builtin_topics_rtps_writer),
                None,
                sedp_topic_topic,
                self.builtin_publisher
                    .read_lock()
                    .clone()
                    .unwrap()
                    .downgrade(),
            );
            sedp_builtin_topics_data_writer.enable()?;
            self.builtin_publisher
                .write_lock()
                .as_ref()
                .unwrap()
                .add_data_writer(sedp_builtin_topics_data_writer);
        }

        Ok(())
    }
}

pub trait SendUserDefinedData {
    fn send_user_defined_data(&self, transport: &mut impl TransportWrite);
}

impl SendUserDefinedData for DdsShared<DomainParticipantImpl> {
    fn send_user_defined_data(&self, transport: &mut impl TransportWrite) {
        let user_defined_publisher_list = self.user_defined_publisher_list.read_lock();
        let user_defined_subscriber_list = self.user_defined_subscriber_list.read_lock();

        for publisher in user_defined_publisher_list.iter() {
            publisher.send_message(transport)
        }

        for subscriber in user_defined_subscriber_list.iter() {
            subscriber.send_message(transport)
        }
    }
}

pub trait ReceiveUserDefinedData {
    fn receive_user_defined_data(&self, transport: &mut impl for<'a> TransportRead<'a>);
}

impl ReceiveUserDefinedData for DdsShared<DomainParticipantImpl> {
    fn receive_user_defined_data(&self, transport: &mut impl for<'a> TransportRead<'a>) {
        let user_defined_publisher_list = self.user_defined_publisher_list.read_lock();
        let user_defined_subscriber_list = self.user_defined_subscriber_list.read_lock();
        while let Some((source_locator, message)) = transport.read() {
            MessageReceiver::new().process_message(
                self.rtps_participant.guid().prefix,
                user_defined_publisher_list.as_slice(),
                user_defined_subscriber_list.as_slice(),
                source_locator,
                &message,
            );
        }
    }
}

pub trait SpdpParticipantDiscovery {
    fn discover_matched_participants(&self) -> DdsResult<()>;
}

impl SpdpParticipantDiscovery for DdsShared<DomainParticipantImpl> {
    fn discover_matched_participants(&self) -> DdsResult<()> {
        let builtin_subscriber = self.builtin_subscriber.read_lock();

        let dcps_participant_topic =
            self.lookup_topicdescription::<SpdpDiscoveredParticipantData>(DCPS_PARTICIPANT)?;

        let spdp_builtin_participant_data_reader =
            builtin_subscriber
                .as_ref()
                .unwrap()
                .lookup_datareader::<SpdpDiscoveredParticipantData>(&dcps_participant_topic)?;

        if let Ok(samples) = spdp_builtin_participant_data_reader.take(
            1,
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        ) {
            for discovered_participant_data_sample in samples.iter() {
                self.add_discovered_participant(
                    discovered_participant_data_sample.data.as_ref().unwrap(),
                )
            }
        }

        Ok(())
    }
}

pub trait SedpWriterDiscovery {
    fn discover_matched_writers(&self) -> DdsResult<()>;
}

impl SedpWriterDiscovery for DdsShared<DomainParticipantImpl> {
    fn discover_matched_writers(&self) -> DdsResult<()> {
        let user_defined_subscribers = self.user_defined_subscriber_list.read_lock();

        if user_defined_subscribers.is_empty() {
            return Ok(());
        }

        let builtin_subscriber = self.builtin_subscriber.read_lock();

        let dcps_publication_topic =
            self.lookup_topicdescription::<DiscoveredWriterData>(DCPS_PUBLICATION)?;
        let sedp_builtin_publication_reader =
            builtin_subscriber
                .as_ref()
                .unwrap()
                .lookup_datareader::<DiscoveredWriterData>(&dcps_publication_topic)?;

        let samples = sedp_builtin_publication_reader.take(
            1,
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        );

        for discovered_writer_data_sample in samples.unwrap_or_else(|_| vec![]).iter() {
            for subscriber in user_defined_subscribers.iter() {
                subscriber.add_matched_writer(discovered_writer_data_sample.data.as_ref().unwrap());
            }
        }

        Ok(())
    }
}

pub trait SedpReaderDiscovery {
    fn discover_matched_readers(&self) -> DdsResult<()>;
}

impl SedpReaderDiscovery for DdsShared<DomainParticipantImpl> {
    fn discover_matched_readers(&self) -> DdsResult<()> {
        let user_defined_publishers = self.user_defined_publisher_list.read_lock();

        if user_defined_publishers.is_empty() {
            return Ok(());
        }

        let builtin_subscriber = self.builtin_subscriber.read_lock();

        let dcps_subscription_topic =
            self.lookup_topicdescription::<DiscoveredReaderData>(DCPS_SUBSCRIPTION)?;
        let sedp_builtin_subscription_reader =
            builtin_subscriber
                .as_ref()
                .unwrap()
                .lookup_datareader::<DiscoveredReaderData>(&dcps_subscription_topic)?;

        let samples = sedp_builtin_subscription_reader.take(
            1,
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        );

        for discovered_reader_data_sample in samples.unwrap_or_else(|_| vec![]).iter() {
            for publisher in user_defined_publishers.iter() {
                publisher.add_matched_reader(discovered_reader_data_sample.data.as_ref().unwrap())
            }
        }

        Ok(())
    }
}
