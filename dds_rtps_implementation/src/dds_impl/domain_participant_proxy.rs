use std::{
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    thread::JoinHandle,
};

use rust_dds_api::{
    builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    dcps_psm::{DomainId, Duration, InstanceHandle, StatusMask, Time},
    domain::domain_participant_listener::DomainParticipantListener,
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::{DomainParticipantQos, PublisherQos, SubscriberQos, TopicQos},
    },
    publication::{publisher::Publisher, publisher_listener::PublisherListener},
    return_type::{DDSError, DDSResult},
    subscription::{subscriber::Subscriber, subscriber_listener::SubscriberListener},
    topic::{topic_description::TopicDescription, topic_listener::TopicListener},
};
use rust_rtps_pim::structure::{
    types::{EntityId, EntityKind, Guid},
    RtpsEntity,
};

use crate::{
    rtps_impl::{rtps_group_impl::RtpsGroupImpl,},
    utils::{
        shared_object::RtpsShared,
        transport::{TransportRead, TransportWrite},
    },
};

use super::{domain_participant_impl::DomainParticipantImpl, publisher_proxy::{PublisherProxy, PublisherImpl}, subscriber_proxy::{SubscriberProxy, SubscriberImpl}, topic_proxy::{TopicProxy, TopicImpl}};

pub trait Transport: TransportRead + TransportWrite + Send {}

impl<T> Transport for T where T: TransportRead + TransportWrite + Send {}

pub struct DomainParticipantProxy {
    is_enabled: Arc<AtomicBool>,
    domain_participant_storage: RtpsShared<DomainParticipantImpl>,
    _worker_threads: Vec<JoinHandle<()>>,
}

impl DomainParticipantProxy {
    pub fn new(domain_participant_storage: RtpsShared<DomainParticipantImpl>) -> Self {
        Self {
            is_enabled: Arc::new(AtomicBool::new(false)),
            domain_participant_storage,
            _worker_threads: Vec::new(),
        }
    }
}

impl<'p> rust_dds_api::domain::domain_participant::PublisherFactory<'p> for DomainParticipantProxy {
    type PublisherType = PublisherProxy<'p>;
    fn create_publisher(
        &'p self,
        qos: Option<PublisherQos>,
        _a_listener: Option<&'static dyn PublisherListener>,
        _mask: StatusMask,
    ) -> Option<Self::PublisherType> {
        let mut domain_participant_lock = self.domain_participant_storage.lock();
        let publisher_qos = qos.unwrap_or(domain_participant_lock.default_publisher_qos.clone());
        domain_participant_lock.user_defined_publisher_counter += 1;
        let entity_id = EntityId::new(
            [domain_participant_lock.user_defined_publisher_counter, 0, 0],
            EntityKind::UserDefinedWriterGroup,
        );
        let guid = Guid::new(
            *domain_participant_lock.rtps_participant.guid().prefix(),
            entity_id,
        );
        let rtps_group = RtpsGroupImpl::new(guid);
        let data_writer_storage_list = Vec::new();
        let publisher_storage =
            PublisherImpl::new(publisher_qos, rtps_group, data_writer_storage_list);
        let publisher_storage_shared = RtpsShared::new(publisher_storage);
        let publisher = PublisherProxy::new(self, publisher_storage_shared.downgrade());
        domain_participant_lock
            .user_defined_publisher_storage
            .push(publisher_storage_shared);
        Some(publisher)
    }

    fn delete_publisher(&self, a_publisher: &Self::PublisherType) -> DDSResult<()> {
        if std::ptr::eq(a_publisher.get_participant(), self) {
            let publisher_storage = a_publisher.publisher_storage().upgrade()?;
            let mut domain_participant_lock = self.domain_participant_storage.lock();
            domain_participant_lock
                .user_defined_publisher_storage
                .retain(|x| x != &publisher_storage);
            Ok(())
        } else {
            Err(DDSError::PreconditionNotMet(
                "Publisher can only be deleted from its parent participant",
            ))
        }
    }
}

impl<'s> rust_dds_api::domain::domain_participant::SubscriberFactory<'s> for DomainParticipantProxy {
    type SubscriberType = SubscriberProxy<'s>;

    fn create_subscriber(
        &'s self,
        qos: Option<SubscriberQos>,
        _a_listener: Option<&'static dyn SubscriberListener>,
        _mask: StatusMask,
    ) -> Option<Self::SubscriberType> {
        let mut domain_participant_lock = self.domain_participant_storage.lock();
        let subscriber_qos = qos.unwrap_or(domain_participant_lock.default_subscriber_qos.clone());
        domain_participant_lock.user_defined_subscriber_counter += 1;
        let entity_id = EntityId::new(
            [
                domain_participant_lock.user_defined_subscriber_counter,
                0,
                0,
            ],
            EntityKind::UserDefinedWriterGroup,
        );
        let guid = Guid::new(
            *domain_participant_lock.rtps_participant.guid().prefix(),
            entity_id,
        );
        let rtps_group = RtpsGroupImpl::new(guid);
        let data_reader_storage_list = Vec::new();
        let subscriber_storage =
            SubscriberImpl::new(subscriber_qos, rtps_group, data_reader_storage_list);
        let subscriber_storage_shared = RtpsShared::new(subscriber_storage);
        let subscriber = SubscriberProxy::new(self, subscriber_storage_shared.downgrade());
        domain_participant_lock
            .user_defined_subscriber_storage
            .push(subscriber_storage_shared);
        Some(subscriber)
    }

    fn delete_subscriber(&self, a_subscriber: &Self::SubscriberType) -> DDSResult<()> {
        if std::ptr::eq(a_subscriber.get_participant(), self) {
            let subscriber_storage = a_subscriber.subscriber_storage().upgrade()?;
            let mut domain_participant_lock = self.domain_participant_storage.lock();
            domain_participant_lock
                .user_defined_subscriber_storage
                .retain(|x| x != &subscriber_storage);
            Ok(())
        } else {
            Err(DDSError::PreconditionNotMet(
                "Subscriber can only be deleted from its parent participant",
            ))
        }
    }

    fn get_builtin_subscriber(&'s self) -> Self::SubscriberType {
        let domain_participant_lock = self.domain_participant_storage.lock();
        let subscriber_storage_shared =
            domain_participant_lock.builtin_subscriber_storage[0].clone();
        SubscriberProxy::new(self, subscriber_storage_shared.downgrade())
    }
}

impl<'t, T: 'static> rust_dds_api::domain::domain_participant::TopicFactory<'t, T>
    for DomainParticipantProxy
{
    type TopicType = TopicProxy<'t, T>;

    fn create_topic(
        &'t self,
        _topic_name: &str,
        qos: Option<TopicQos>,
        _a_listener: Option<&'static dyn TopicListener<DataPIM = T>>,
        _mask: StatusMask,
    ) -> Option<Self::TopicType> {
        let topic_qos = qos.unwrap_or(
            self.domain_participant_storage
                .lock()
                .default_topic_qos
                .clone(),
        );
        let topic_storage = TopicImpl::new(topic_qos);
        let topic_storage_shared = RtpsShared::new(topic_storage);
        let topic = TopicProxy::new(self, topic_storage_shared.downgrade());
        self.domain_participant_storage
            .lock()
            .topic_storage
            .push(topic_storage_shared);
        Some(topic)
    }

    fn delete_topic(&self, _a_topic: &Self::TopicType) -> DDSResult<()> {
        todo!()
    }

    fn find_topic(&self, _topic_name: &str, _timeout: Duration) -> Option<Self::TopicType> {
        todo!()
    }
}

impl rust_dds_api::domain::domain_participant::DomainParticipant for DomainParticipantProxy {
    fn lookup_topicdescription<'t, T>(
        &'t self,
        _name: &'t str,
    ) -> Option<&'t (dyn TopicDescription<T> + 't)> {
        todo!()
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

    fn set_default_publisher_qos(&self, qos: Option<PublisherQos>) -> DDSResult<()> {
        self.domain_participant_storage.lock().default_publisher_qos = qos.unwrap_or_default();
        Ok(())
    }

    fn get_default_publisher_qos(&self) -> PublisherQos {
        self.domain_participant_storage
            .lock()
            .default_publisher_qos
            .clone()
    }

    fn set_default_subscriber_qos(&self, qos: Option<SubscriberQos>) -> DDSResult<()> {
        self.domain_participant_storage
            .lock()
            .default_subscriber_qos = qos.unwrap_or_default();
        Ok(())
    }

    fn get_default_subscriber_qos(&self) -> SubscriberQos {
        self.domain_participant_storage
            .lock()
            .default_subscriber_qos
            .clone()
    }

    fn set_default_topic_qos(&self, qos: Option<TopicQos>) -> DDSResult<()> {
        let topic_qos = qos.unwrap_or_default();
        topic_qos.is_consistent()?;
        self.domain_participant_storage.lock().default_topic_qos = topic_qos;
        Ok(())
    }

    fn get_default_topic_qos(&self) -> TopicQos {
        self.domain_participant_storage
            .lock()
            .default_topic_qos
            .clone()
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

impl Entity for DomainParticipantProxy {
    type Qos = DomainParticipantQos;
    type Listener = &'static dyn DomainParticipantListener;

    fn set_qos(&self, qos: Option<Self::Qos>) -> DDSResult<()> {
        self.domain_participant_storage
            .lock()
            .domain_participant_qos = qos.unwrap_or_default();
        Ok(())
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        Ok(self
            .domain_participant_storage
            .lock()
            .domain_participant_qos
            .clone())
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

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle> {
        todo!()
        // Ok(crate::utils::instance_handle_from_guid(
        //     &self.rtps_participant_impl.lock().guid(),
        // ))
    }

    fn enable(&self) -> DDSResult<()> {
        self.is_enabled.store(true, atomic::Ordering::Release);
        let is_enabled = self.is_enabled.clone();
        let domain_participant_storage = self.domain_participant_storage.clone();
        std::thread::spawn(move || {
            while is_enabled.load(atomic::Ordering::Relaxed) {
                domain_participant_storage.lock().send_builtin_data();
                domain_participant_storage.lock().receive_builtin_data();
                domain_participant_storage.lock().send_user_defined_data();
                domain_participant_storage
                    .lock()
                    .receive_user_defined_data();
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{rtps_impl::rtps_participant_impl::RtpsParticipantImpl, utils::transport::RtpsMessageRead};

    use super::*;

    use rust_dds_api::domain::domain_participant::DomainParticipant;
    use rust_rtps_pim::structure::types::Locator;

    struct MockDDSType;

    struct MockTransport;

    impl TransportRead for MockTransport {
        fn read(&mut self) -> Option<(Locator, RtpsMessageRead)> {
        todo!()
    }
    }

    impl TransportWrite for MockTransport {
        fn write(
            &mut self,
            _message: &crate::utils::transport::RtpsMessageWrite<'_>,
            _destination_locator: &Locator,
        ) {
            todo!()
        }
    }

    #[test]
    fn set_default_publisher_qos_some_value() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant_storage = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let domain_participant_impl =
            DomainParticipantProxy::new(RtpsShared::new(domain_participant_storage));
        let mut qos = PublisherQos::default();
        qos.group_data.value = &[1, 2, 3, 4];
        domain_participant_impl
            .set_default_publisher_qos(Some(qos.clone()))
            .unwrap();
        assert!(domain_participant_impl.get_default_publisher_qos() == qos);
    }

    #[test]
    fn set_default_publisher_qos_none() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant_storage = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let domain_participant_impl =
            DomainParticipantProxy::new(RtpsShared::new(domain_participant_storage));
        let mut qos = PublisherQos::default();
        qos.group_data.value = &[1, 2, 3, 4];
        domain_participant_impl
            .set_default_publisher_qos(Some(qos.clone()))
            .unwrap();

        domain_participant_impl
            .set_default_publisher_qos(None)
            .unwrap();
        assert!(domain_participant_impl.get_default_publisher_qos() == PublisherQos::default());
    }

    #[test]
    fn set_default_subscriber_qos_some_value() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant_storage = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let domain_participant_impl =
            DomainParticipantProxy::new(RtpsShared::new(domain_participant_storage));
        let mut qos = SubscriberQos::default();
        qos.group_data.value = &[1, 2, 3, 4];
        domain_participant_impl
            .set_default_subscriber_qos(Some(qos.clone()))
            .unwrap();
        assert_eq!(domain_participant_impl.get_default_subscriber_qos(), qos);
    }

    #[test]
    fn set_default_subscriber_qos_none() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant_storage = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let domain_participant_impl =
            DomainParticipantProxy::new(RtpsShared::new(domain_participant_storage));
        let mut qos = SubscriberQos::default();
        qos.group_data.value = &[1, 2, 3, 4];
        domain_participant_impl
            .set_default_subscriber_qos(Some(qos.clone()))
            .unwrap();

        domain_participant_impl
            .set_default_subscriber_qos(None)
            .unwrap();
        assert_eq!(
            domain_participant_impl.get_default_subscriber_qos(),
            SubscriberQos::default()
        );
    }

    #[test]
    fn set_default_topic_qos_some_value() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant_storage = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let domain_participant_impl =
            DomainParticipantProxy::new(RtpsShared::new(domain_participant_storage));
        let mut qos = TopicQos::default();
        qos.topic_data.value = &[1, 2, 3, 4];
        domain_participant_impl
            .set_default_topic_qos(Some(qos.clone()))
            .unwrap();
        assert_eq!(domain_participant_impl.get_default_topic_qos(), qos);
    }

    #[test]
    fn set_default_topic_qos_inconsistent() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant_storage = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let domain_participant_impl =
            DomainParticipantProxy::new(RtpsShared::new(domain_participant_storage));
        let mut qos = TopicQos::default();
        qos.resource_limits.max_samples_per_instance = 2;
        qos.resource_limits.max_samples = 1;
        let set_default_topic_qos_result =
            domain_participant_impl.set_default_topic_qos(Some(qos.clone()));
        assert!(set_default_topic_qos_result == Err(DDSError::InconsistentPolicy));
    }

    #[test]
    fn set_default_topic_qos_none() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant_storage = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let domain_participant_impl =
            DomainParticipantProxy::new(RtpsShared::new(domain_participant_storage));
        let mut qos = TopicQos::default();
        qos.topic_data.value = &[1, 2, 3, 4];
        domain_participant_impl
            .set_default_topic_qos(Some(qos.clone()))
            .unwrap();

        domain_participant_impl.set_default_topic_qos(None).unwrap();
        assert_eq!(
            domain_participant_impl.get_default_topic_qos(),
            TopicQos::default()
        );
    }

    #[test]
    fn create_publisher() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant_storage = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let domain_participant_impl =
            DomainParticipantProxy::new(RtpsShared::new(domain_participant_storage));
        let publisher = domain_participant_impl.create_publisher(None, None, 0);

        assert!(publisher.is_some())
    }

    #[test]
    fn create_subscriber() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant_storage = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let domain_participant_impl =
            DomainParticipantProxy::new(RtpsShared::new(domain_participant_storage));
        let subscriber = domain_participant_impl.create_subscriber(None, None, 0);

        assert!(subscriber.is_some())
    }

    #[test]
    fn create_topic() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant_storage = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let domain_participant_impl =
            DomainParticipantProxy::new(RtpsShared::new(domain_participant_storage));
        let topic =
            domain_participant_impl.create_topic::<MockDDSType>("topic_name", None, None, 0);
        assert!(topic.is_some());
    }
}
