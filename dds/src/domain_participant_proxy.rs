use dds_api::{
    builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    dcps_psm::{DomainId, Duration, InstanceHandle, StatusMask, Time},
    domain::{
        domain_participant::{DomainParticipant, DomainParticipantTopicFactory},
        domain_participant_listener::DomainParticipantListener,
    },
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::{DomainParticipantQos, PublisherQos, SubscriberQos, TopicQos},
    },
    publication::publisher_listener::PublisherListener,
    return_type::DdsResult,
    subscription::subscriber_listener::SubscriberListener,
    topic::topic_listener::TopicListener,
};
use dds_implementation::utils::shared_object::{DdsShared, DdsWeak};

use crate::{
    publisher_proxy::PublisherProxy, subscriber_proxy::SubscriberProxy, topic_proxy::TopicProxy,
};

pub struct DomainParticipantProxy<I> {
    domain_participant_attributes: DdsWeak<I>,
}

impl<I> Clone for DomainParticipantProxy<I> {
    fn clone(&self) -> Self {
        Self {
            domain_participant_attributes: self.domain_participant_attributes.clone(),
        }
    }
}

impl<I> DomainParticipantProxy<I> {
    pub fn new(domain_participant_attributes: DdsWeak<I>) -> Self {
        Self {
            domain_participant_attributes,
        }
    }
}

impl<I> PartialEq for DomainParticipantProxy<I> {
    fn eq(&self, other: &Self) -> bool {
        self.domain_participant_attributes
            .ptr_eq(&other.domain_participant_attributes)
    }
}

impl<Foo, I, T> DomainParticipantTopicFactory<Foo> for DomainParticipantProxy<I>
where
    DdsShared<I>: DomainParticipantTopicFactory<Foo, TopicType = DdsShared<T>>,
    DdsShared<T>: Entity<Qos = TopicQos, Listener = Box<dyn TopicListener>>,
{
    type TopicType = TopicProxy<Foo, T>;

    fn topic_factory_create_topic(
        &self,
        topic_name: &str,
        qos: Option<TopicQos>,
        a_listener: Option<<Self::TopicType as Entity>::Listener>,
        mask: StatusMask,
    ) -> DdsResult<Self::TopicType> {
        DomainParticipantTopicFactory::<Foo>::topic_factory_create_topic(
            &self.domain_participant_attributes.upgrade()?,
            topic_name,
            qos,
            a_listener,
            mask,
        )
        .map(|x| TopicProxy::new(x.downgrade()))
    }

    fn topic_factory_delete_topic(&self, a_topic: &Self::TopicType) -> DdsResult<()> {
        DomainParticipantTopicFactory::<Foo>::topic_factory_delete_topic(
            &self.domain_participant_attributes.upgrade()?,
            &a_topic.as_ref().upgrade()?,
        )
    }

    fn topic_factory_find_topic(
        &self,
        topic_name: &str,
        timeout: Duration,
    ) -> DdsResult<Self::TopicType> {
        DomainParticipantTopicFactory::<Foo>::topic_factory_find_topic(
            &self.domain_participant_attributes.upgrade()?,
            topic_name,
            timeout,
        )
        .map(|x| TopicProxy::new(x.downgrade()))
    }

    fn topic_factory_lookup_topicdescription(
        &self,
        topic_name: &str,
    ) -> DdsResult<Self::TopicType> {
        DomainParticipantTopicFactory::<Foo>::topic_factory_lookup_topicdescription(
            &self.domain_participant_attributes.upgrade()?,
            topic_name,
        )
        .map(|x| TopicProxy::new(x.downgrade()))
    }
}

impl<I, P, S> DomainParticipant for DomainParticipantProxy<I>
where
    DdsShared<I>: DomainParticipant<PublisherType = DdsShared<P>, SubscriberType = DdsShared<S>>,
    DdsShared<P>: Entity<Qos = PublisherQos, Listener = Box<dyn PublisherListener>>,
    DdsShared<S>: Entity<Qos = SubscriberQos, Listener = Box<dyn SubscriberListener>>,
{
    type PublisherType = PublisherProxy<P>;
    type SubscriberType = SubscriberProxy<S>;

    fn create_publisher(
        &self,
        qos: Option<PublisherQos>,
        a_listener: Option<<Self::PublisherType as Entity>::Listener>,
        mask: StatusMask,
    ) -> DdsResult<Self::PublisherType> {
        self.domain_participant_attributes
            .upgrade()?
            .create_publisher(qos, a_listener, mask)
            .map(|x| PublisherProxy::new(x.downgrade()))
    }

    fn delete_publisher(&self, a_publisher: &Self::PublisherType) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .delete_publisher(&a_publisher.as_ref().upgrade()?)
    }

    fn create_subscriber(
        &self,
        qos: Option<SubscriberQos>,
        a_listener: Option<<Self::SubscriberType as Entity>::Listener>,
        mask: StatusMask,
    ) -> DdsResult<Self::SubscriberType> {
        self.domain_participant_attributes
            .upgrade()?
            .create_subscriber(qos, a_listener, mask)
            .map(|x| SubscriberProxy::new(x.downgrade()))
    }

    fn delete_subscriber(&self, a_subscriber: &Self::SubscriberType) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .delete_subscriber(&a_subscriber.as_ref().upgrade()?)
    }

    fn get_builtin_subscriber(&self) -> DdsResult<Self::SubscriberType> {
        self.domain_participant_attributes
            .upgrade()?
            .get_builtin_subscriber()
            .map(|x| SubscriberProxy::new(x.downgrade()))
    }

    fn ignore_participant(&self, handle: InstanceHandle) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .ignore_participant(handle)
    }

    fn ignore_topic(&self, handle: InstanceHandle) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .ignore_topic(handle)
    }

    fn ignore_publication(&self, handle: InstanceHandle) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .ignore_publication(handle)
    }

    fn ignore_subscription(&self, handle: InstanceHandle) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .ignore_subscription(handle)
    }

    fn get_domain_id(&self) -> DdsResult<DomainId> {
        self.domain_participant_attributes
            .upgrade()?
            .get_domain_id()
    }

    fn delete_contained_entities(&self) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .delete_contained_entities()
    }

    fn assert_liveliness(&self) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .assert_liveliness()
    }

    fn set_default_publisher_qos(&self, qos: Option<PublisherQos>) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .set_default_publisher_qos(qos)
    }

    fn get_default_publisher_qos(&self) -> DdsResult<PublisherQos> {
        self.domain_participant_attributes
            .upgrade()?
            .get_default_publisher_qos()
    }

    fn set_default_subscriber_qos(&self, qos: Option<SubscriberQos>) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .set_default_subscriber_qos(qos)
    }

    fn get_default_subscriber_qos(&self) -> DdsResult<SubscriberQos> {
        self.domain_participant_attributes
            .upgrade()?
            .get_default_subscriber_qos()
    }

    fn set_default_topic_qos(&self, qos: Option<TopicQos>) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .set_default_topic_qos(qos)
    }

    fn get_default_topic_qos(&self) -> DdsResult<TopicQos> {
        self.domain_participant_attributes
            .upgrade()?
            .get_default_topic_qos()
    }

    fn get_discovered_participants(&self) -> DdsResult<Vec<InstanceHandle>> {
        self.domain_participant_attributes
            .upgrade()?
            .get_discovered_participants()
    }

    fn get_discovered_participant_data(
        &self,
        participant_data: ParticipantBuiltinTopicData,
        participant_handle: InstanceHandle,
    ) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .get_discovered_participant_data(participant_data, participant_handle)
    }

    fn get_discovered_topics(&self, topic_handles: &mut [InstanceHandle]) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .get_discovered_topics(topic_handles)
    }

    fn get_discovered_topic_data(
        &self,
        topic_data: TopicBuiltinTopicData,
        topic_handle: InstanceHandle,
    ) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .get_discovered_topic_data(topic_data, topic_handle)
    }

    fn contains_entity(&self, a_handle: InstanceHandle) -> DdsResult<bool> {
        self.domain_participant_attributes
            .upgrade()?
            .contains_entity(a_handle)
    }

    fn get_current_time(&self) -> DdsResult<Time> {
        self.domain_participant_attributes
            .upgrade()?
            .get_current_time()
    }
}

impl<I> Entity for DomainParticipantProxy<I>
where
    DdsShared<I>: Entity<Qos = DomainParticipantQos, Listener = Box<dyn DomainParticipantListener>>,
{
    type Qos = DomainParticipantQos;
    type Listener = Box<dyn DomainParticipantListener>;

    fn set_qos(&self, qos: Option<Self::Qos>) -> DdsResult<()> {
        self.domain_participant_attributes.upgrade()?.set_qos(qos)
    }

    fn get_qos(&self) -> DdsResult<Self::Qos> {
        self.domain_participant_attributes.upgrade()?.get_qos()
    }

    fn set_listener(&self, a_listener: Option<Self::Listener>, mask: StatusMask) -> DdsResult<()> {
        self.domain_participant_attributes
            .upgrade()?
            .set_listener(a_listener, mask)
    }

    fn get_listener(&self) -> DdsResult<Option<Self::Listener>> {
        self.domain_participant_attributes.upgrade()?.get_listener()
    }

    fn get_statuscondition(&self) -> DdsResult<StatusCondition> {
        self.domain_participant_attributes
            .upgrade()?
            .get_statuscondition()
    }

    fn get_status_changes(&self) -> DdsResult<StatusMask> {
        self.domain_participant_attributes
            .upgrade()?
            .get_status_changes()
    }

    fn enable(&self) -> DdsResult<()> {
        self.domain_participant_attributes.upgrade()?.enable()
    }

    fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        self.domain_participant_attributes
            .upgrade()?
            .get_instance_handle()
    }
}
