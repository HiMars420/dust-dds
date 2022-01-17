use rust_dds_api::{
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
    publication::{publisher::Publisher, publisher_listener::PublisherListener},
    return_type::{DDSError, DDSResult},
    subscription::{subscriber::Subscriber, subscriber_listener::SubscriberListener},
    topic::{topic_description::TopicDescription, topic_listener::TopicListener},
};

use crate::{
    dds_type::DdsType,
    utils::shared_object::{
        rtps_shared_downgrade, rtps_shared_read_lock, rtps_shared_write_lock, rtps_weak_upgrade,
        RtpsWeak,
    },
};

use super::{
    domain_participant_impl::DomainParticipantImpl, publisher_proxy::PublisherProxy,
    subscriber_proxy::SubscriberProxy, topic_proxy::TopicProxy,
};

#[derive(Clone)]
pub struct DomainParticipantProxy {
    domain_participant: RtpsWeak<DomainParticipantImpl>,
}

impl DomainParticipantProxy {
    pub fn new(domain_participant: RtpsWeak<DomainParticipantImpl>) -> Self {
        Self { domain_participant }
    }
}

impl<Foo> DomainParticipantTopicFactory<Foo> for DomainParticipantProxy
where
    Foo: DdsType + 'static,
{
    type TopicType = TopicProxy<Foo>;

    fn topic_factory_create_topic(
        &self,
        topic_name: &str,
        qos: Option<TopicQos>,
        a_listener: Option<Box<dyn TopicListener>>,
        mask: StatusMask,
    ) -> Option<Self::TopicType> {
        let domain_participant_lock = rtps_weak_upgrade(&self.domain_participant).ok()?;
        let topic_shared = DomainParticipantTopicFactory::<Foo>::topic_factory_create_topic(
            &*rtps_shared_read_lock(&domain_participant_lock),
            topic_name,
            qos,
            a_listener,
            mask,
        )?;
        let topic_weak = rtps_shared_downgrade(&topic_shared);
        Some(TopicProxy::new(self.clone(), topic_weak))
    }

    fn topic_factory_delete_topic(&self, a_topic: &Self::TopicType) -> DDSResult<()> {
        let domain_participant_lock = rtps_weak_upgrade(&self.domain_participant)?;
        let topic_shared = rtps_weak_upgrade(a_topic.as_ref())?;
        if std::ptr::eq(&a_topic.get_participant(), self) {
            // Explicit call with the complete function path otherwise the generic type can't be infered.
            // This happens because TopicImpl has no generic type information.
            DomainParticipantTopicFactory::<Foo>::topic_factory_delete_topic(
                &*rtps_shared_read_lock(&domain_participant_lock),
                &topic_shared,
            )
        } else {
            Err(DDSError::PreconditionNotMet(
                "Subscriber can only be deleted from its parent participant".to_string(),
            ))
        }
    }

    fn topic_factory_find_topic(
        &self,
        _topic_name: &str,
        _timeout: Duration,
    ) -> Option<Self::TopicType> {
        // Explicit call with the complete function path otherwise the generic type can't be infered.
        // This happens because TopicImpl has no generic type information.
        // let domain_participant = rtps_shared_read_lock(&domain_participant_lock)
        // let topic_shared = DomainParticipantTopicFactory::<'t, Foo>::topic_factory_find_topic(
        //     &*,
        //     topic_name,
        //     timeout,
        // )?;
        // let topic_weak = rtps_shared_downgrade(&topic_shared);
        // Some(TopicProxy::new(self, topic_weak))
        todo!()
    }
}

impl DomainParticipant for DomainParticipantProxy {
    type PublisherType = PublisherProxy;
    type SubscriberType = SubscriberProxy;

    fn create_publisher(
        &self,
        qos: Option<PublisherQos>,
        a_listener: Option<&'static dyn PublisherListener>,
        mask: StatusMask,
    ) -> Option<Self::PublisherType> {
        let domain_participant_lock = rtps_weak_upgrade(&self.domain_participant).ok()?;
        let publisher_shared = rtps_shared_read_lock(&domain_participant_lock)
            .create_publisher(qos, a_listener, mask)?;
        let publisher_weak = rtps_shared_downgrade(&publisher_shared);

        Some(PublisherProxy::new(self.clone(), publisher_weak))
    }

    fn delete_publisher(&self, a_publisher: &Self::PublisherType) -> DDSResult<()> {
        let domain_participant_lock = rtps_weak_upgrade(&self.domain_participant)?;
        let publisher_shared = rtps_weak_upgrade(a_publisher.as_ref())?;
        if std::ptr::eq(&a_publisher.get_participant(), self) {
            rtps_shared_read_lock(&domain_participant_lock).delete_publisher(&publisher_shared)
        } else {
            Err(DDSError::PreconditionNotMet(
                "Publisher can only be deleted from its parent participant".to_string(),
            ))
        }
    }

    fn create_subscriber(
        &self,
        qos: Option<SubscriberQos>,
        a_listener: Option<&'static dyn SubscriberListener>,
        mask: StatusMask,
    ) -> Option<Self::SubscriberType> {
        let domain_participant_lock = rtps_weak_upgrade(&self.domain_participant).ok()?;
        let subscriber_shared = rtps_shared_read_lock(&domain_participant_lock)
            .create_subscriber(qos, a_listener, mask)?;
        let subscriber_weak = rtps_shared_downgrade(&subscriber_shared);
        Some(SubscriberProxy::new(self.clone(), subscriber_weak))
    }

    fn delete_subscriber(&self, a_subscriber: &Self::SubscriberType) -> DDSResult<()> {
        let domain_participant_lock = rtps_weak_upgrade(&self.domain_participant)?;
        let subscriber_shared = rtps_weak_upgrade(a_subscriber.as_ref())?;
        if std::ptr::eq(&a_subscriber.get_participant(), self) {
            rtps_shared_read_lock(&domain_participant_lock).delete_subscriber(&subscriber_shared)
        } else {
            Err(DDSError::PreconditionNotMet(
                "Subscriber can only be deleted from its parent participant".to_string(),
            ))
        }
    }

    fn lookup_topicdescription<Foo>(
        &self,
        _name: &str,
    ) -> Option<&dyn TopicDescription<DomainParticipant = Self>>
    where
        Self: Sized,
    {
        todo!()
        // rtps_shared_read_lock(&domain_participant_lock).lookup_topicdescription(name)
    }

    fn get_builtin_subscriber(&self) -> DDSResult<Self::SubscriberType> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let subscriber_shared =
            rtps_shared_read_lock(&domain_participant_shared).get_builtin_subscriber()?;
        let subscriber_weak = rtps_shared_downgrade(&subscriber_shared);
        Ok(SubscriberProxy::new(self.clone(), subscriber_weak))
    }

    fn ignore_participant(&self, handle: InstanceHandle) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.ignore_participant(handle)
    }

    fn ignore_topic(&self, handle: InstanceHandle) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.ignore_topic(handle)
    }

    fn ignore_publication(&self, handle: InstanceHandle) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.ignore_publication(handle)
    }

    fn ignore_subscription(&self, handle: InstanceHandle) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.ignore_subscription(handle)
    }

    fn get_domain_id(&self) -> DDSResult<DomainId> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_domain_id()
    }

    fn delete_contained_entities(&self) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.delete_contained_entities()
    }

    fn assert_liveliness(&self) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.assert_liveliness()
    }

    fn set_default_publisher_qos(&mut self, qos: Option<PublisherQos>) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let mut domain_participant_lock = rtps_shared_write_lock(&domain_participant_shared);
        domain_participant_lock.set_default_publisher_qos(qos)
    }

    fn get_default_publisher_qos(&self) -> DDSResult<PublisherQos> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_default_publisher_qos()
    }

    fn set_default_subscriber_qos(&mut self, qos: Option<SubscriberQos>) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let mut domain_participant_lock = rtps_shared_write_lock(&domain_participant_shared);
        domain_participant_lock.set_default_subscriber_qos(qos)
    }

    fn get_default_subscriber_qos(&self) -> DDSResult<SubscriberQos> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_default_subscriber_qos()
    }

    fn set_default_topic_qos(&mut self, qos: Option<TopicQos>) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let mut domain_participant_lock = rtps_shared_write_lock(&domain_participant_shared);
        domain_participant_lock.set_default_topic_qos(qos)
    }

    fn get_default_topic_qos(&self) -> DDSResult<TopicQos> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_default_topic_qos()
    }

    fn get_discovered_participants(
        &self,
        participant_handles: &mut [InstanceHandle],
    ) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_discovered_participants(participant_handles)
    }

    fn get_discovered_participant_data(
        &self,
        participant_data: ParticipantBuiltinTopicData,
        participant_handle: InstanceHandle,
    ) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock
            .get_discovered_participant_data(participant_data, participant_handle)
    }

    fn get_discovered_topics(&self, topic_handles: &mut [InstanceHandle]) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_discovered_topics(topic_handles)
    }

    fn get_discovered_topic_data(
        &self,
        topic_data: TopicBuiltinTopicData,
        topic_handle: InstanceHandle,
    ) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_discovered_topic_data(topic_data, topic_handle)
    }

    fn contains_entity(&self, a_handle: InstanceHandle) -> DDSResult<bool> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.contains_entity(a_handle)
    }

    fn get_current_time(&self) -> DDSResult<Time> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_current_time()
    }
}

impl Entity for DomainParticipantProxy {
    type Qos = DomainParticipantQos;
    type Listener = &'static dyn DomainParticipantListener;

    fn set_qos(&mut self, qos: Option<Self::Qos>) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let mut domain_participant_lock = rtps_shared_write_lock(&domain_participant_shared);
        domain_participant_lock.set_qos(qos)
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_qos()
    }

    fn set_listener(&self, a_listener: Option<Self::Listener>, mask: StatusMask) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.set_listener(a_listener, mask)
    }

    fn get_listener(&self) -> DDSResult<Option<Self::Listener>> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_listener()
    }

    fn get_statuscondition(&self) -> DDSResult<StatusCondition> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_statuscondition()
    }

    fn get_status_changes(&self) -> DDSResult<StatusMask> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_status_changes()
    }

    fn enable(&self) -> DDSResult<()> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.enable()
    }

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle> {
        let domain_participant_shared = rtps_weak_upgrade(&self.domain_participant)?;
        let domain_participant_lock = rtps_shared_read_lock(&domain_participant_shared);
        domain_participant_lock.get_instance_handle()
    }
}
