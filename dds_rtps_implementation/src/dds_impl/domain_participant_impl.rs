use std::sync::{
    atomic::{self, AtomicBool, AtomicU8},
    Arc, Mutex,
};

use rust_dds_api::{
    builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    dcps_psm::{DomainId, Duration, InstanceHandle, StatusMask, Time},
    domain::{
        domain_participant::{DomainParticipant, PublisherGAT, SubscriberGAT, TopicGAT},
        domain_participant_listener::DomainParticipantListener,
    },
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::{DomainParticipantQos, PublisherQos, SubscriberQos, TopicQos},
    },
    publication::{publisher::Publisher, publisher_listener::PublisherListener},
    return_type::{DDSError, DDSResult},
    subscription::subscriber_listener::SubscriberListener,
    topic::{topic_description::TopicDescription, topic_listener::TopicListener},
};
use rust_rtps_pim::structure::{
    types::{EntityId, EntityKind, Guid},
    RtpsEntity,
};

use crate::{
    rtps_impl::{rtps_group_impl::RtpsGroupImpl, rtps_participant_impl::RtpsParticipantImpl},
    utils::{
        shared_object::RtpsShared,
        transport::{TransportRead, TransportWrite},
    },
};

use super::{
    publisher_impl::PublisherImpl, publisher_proxy::PublisherProxy,
    subscriber_impl::SubscriberImpl, subscriber_proxy::SubscriberProxy, topic_impl::TopicImpl,
    topic_proxy::TopicProxy,
};

pub trait Transport: TransportRead + TransportWrite + Send + Sync {}

impl<T> Transport for T where T: TransportRead + TransportWrite + Send + Sync {}

pub struct DomainParticipantImpl {
    rtps_participant: Arc<RtpsParticipantImpl>,
    _qos: DomainParticipantQos,
    builtin_subscriber_storage: Arc<Vec<RtpsShared<SubscriberImpl>>>,
    builtin_publisher_storage: Arc<Vec<RtpsShared<PublisherImpl>>>,
    user_defined_subscriber_storage: Arc<Mutex<Vec<RtpsShared<SubscriberImpl>>>>,
    _user_defined_subscriber_counter: u8,
    default_subscriber_qos: SubscriberQos,
    user_defined_publisher_storage: Arc<Mutex<Vec<RtpsShared<PublisherImpl>>>>,
    user_defined_publisher_counter: AtomicU8,
    default_publisher_qos: PublisherQos,
    _topic_storage: Vec<RtpsShared<TopicImpl>>,
    default_topic_qos: TopicQos,
    metatraffic_transport: Arc<Mutex<Box<dyn Transport>>>,
    default_transport: Arc<Mutex<Box<dyn Transport>>>,
    is_enabled: Arc<AtomicBool>,
}

impl DomainParticipantImpl {
    pub fn new(
        domain_participant_qos: DomainParticipantQos,
        rtps_participant: RtpsParticipantImpl,
        builtin_subscriber_storage: Vec<RtpsShared<SubscriberImpl>>,
        builtin_publisher_storage: Vec<RtpsShared<PublisherImpl>>,
        metatraffic_transport: Box<dyn Transport>,
        default_transport: Box<dyn Transport>,
    ) -> Self {
        Self {
            rtps_participant: Arc::new(rtps_participant),
            _qos: domain_participant_qos,
            builtin_subscriber_storage: Arc::new(builtin_subscriber_storage),
            builtin_publisher_storage: Arc::new(builtin_publisher_storage),
            metatraffic_transport: Arc::new(Mutex::new(metatraffic_transport)),
            default_transport: Arc::new(Mutex::new(default_transport)),
            user_defined_subscriber_storage: Arc::new(Mutex::new(Vec::new())),
            _user_defined_subscriber_counter: 0,
            default_subscriber_qos: SubscriberQos::default(),
            user_defined_publisher_storage: Arc::new(Mutex::new(Vec::new())),
            user_defined_publisher_counter: AtomicU8::new(0),
            default_publisher_qos: PublisherQos::default(),
            _topic_storage: Vec::new(),
            default_topic_qos: TopicQos::default(),
            is_enabled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<'p> PublisherGAT<'p> for DomainParticipantImpl {
    type PublisherType = PublisherProxy<'p, PublisherImpl>;
    fn create_publisher_gat(
        &'p self,
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
            EntityKind::UserDefinedWriterGroup,
        );
        let guid = Guid::new(*self.rtps_participant.guid().prefix(), entity_id);
        let rtps_group = RtpsGroupImpl::new(guid);
        let data_writer_impl_list = Vec::new();
        let publisher_impl = PublisherImpl::new(publisher_qos, rtps_group, data_writer_impl_list);
        let publisher_impl_shared = RtpsShared::new(publisher_impl);
        let publisher_impl_weak = publisher_impl_shared.downgrade();
        self.user_defined_publisher_storage
            .lock()
            .unwrap()
            .push(publisher_impl_shared);
        let publisher = PublisherProxy::new(self, publisher_impl_weak);

        Some(publisher)
    }

    fn delete_publisher_gat(&self, a_publisher: &Self::PublisherType) -> DDSResult<()> {
        // let publisher_storage = a_publisher.upgrade()?;

        if std::ptr::eq(a_publisher.get_participant(), self) {
            let publisher_impl_shared = a_publisher.publisher_impl().upgrade()?;
            self.user_defined_publisher_storage
                .lock()
                .unwrap()
                .retain(|x| x != &publisher_impl_shared);
            Ok(())
        } else {
            Err(DDSError::PreconditionNotMet(
                "Publisher can only be deleted from its parent participant",
            ))
        }
    }
}

impl<'s> SubscriberGAT<'s> for DomainParticipantImpl {
    type SubscriberType = SubscriberProxy<'s, SubscriberImpl>;

    fn create_subscriber_gat(
        &'s self,
        _qos: Option<SubscriberQos>,
        _a_listener: Option<&'static dyn SubscriberListener>,
        _mask: StatusMask,
    ) -> Option<Self::SubscriberType> {
        // let subscriber_qos = qos.unwrap_or(self.default_subscriber_qos.clone());
        // self.user_defined_subscriber_counter += 1;
        // let entity_id = EntityId::new(
        //     [self.user_defined_subscriber_counter, 0, 0],
        //     EntityKind::UserDefinedWriterGroup,
        // );
        // let guid = Guid::new(*self.rtps_participant.guid().prefix(), entity_id);
        // let rtps_group = RtpsGroupImpl::new(guid);
        // let data_reader_storage_list = Vec::new();
        // let subscriber_storage =
        //     SubscriberImpl::new(subscriber_qos, rtps_group, data_reader_storage_list);
        // let subscriber_storage_shared = RtpsShared::new(subscriber_storage);
        // let subscriber_storage_weak = subscriber_storage_shared.downgrade();
        // self.user_defined_subscriber_storage
        //     .push(subscriber_storage_shared);
        // Some(subscriber_storage_weak)

        // let subscriber_storage_weak = self
        //     .domain_participant_storage
        //     .lock()
        //     .create_subscriber(qos, a_listener, mask)?;
        // let subscriber = SubscriberProxy::new(self, subscriber_storage_weak);
        // Some(subscriber)
        todo!()
    }

    fn delete_subscriber_gat(&self, _a_subscriber: &Self::SubscriberType) -> DDSResult<()> {
        // let subscriber_storage = a_subscriber.upgrade()?;
        // self.user_defined_subscriber_storage
        //     .retain(|x| x != &subscriber_storage);
        // Ok(())

        // if std::ptr::eq(a_subscriber.get_participant(), self) {
        //     self.domain_participant_storage
        //         .lock()
        //         .delete_subscriber(a_subscriber.subscriber_storage())
        // } else {
        //     Err(DDSError::PreconditionNotMet(
        //         "Subscriber can only be deleted from its parent participant",
        //     ))
        // }
        todo!()
    }

    fn get_builtin_subscriber_gat(&'s self) -> Self::SubscriberType {
        // self.builtin_subscriber_storage[0].clone().downgrade()

        // let subscriber_storage_weak = self
        //     .domain_participant_storage
        //     .lock()
        //     .get_builtin_subscriber();
        // SubscriberProxy::new(self, subscriber_storage_weak)
        todo!()
    }
}

impl<'t, T: 'static> TopicGAT<'t, T> for DomainParticipantImpl {
    type TopicType = TopicProxy<'t, T, TopicImpl>;

    fn create_topic_gat(
        &'t self,
        _topic_name: &str,
        _qos: Option<TopicQos>,
        _a_listener: Option<&'static dyn TopicListener<DataPIM = T>>,
        _mask: StatusMask,
    ) -> Option<Self::TopicType> {
        // let topic_qos = qos.unwrap_or(self.default_topic_qos.clone());
        // let topic_storage = TopicImpl::new(topic_qos);
        // let topic_storage_shared = RtpsShared::new(topic_storage);
        // let topic_storage_weak = topic_storage_shared.downgrade();
        // self.topic_storage.push(topic_storage_shared);
        // Some(topic_storage_weak)

        // let topic_storage_weak = self
        //     .domain_participant_storage
        //     .lock()
        //     .create_topic(topic_name, qos, a_listener, mask)?;
        // let topic = TopicProxy::new(self, topic_storage_weak);
        // Some(topic)
        todo!()
    }

    fn delete_topic_gat(&self, _a_topic: &Self::TopicType) -> DDSResult<()> {
        todo!()
    }

    fn find_topic_gat(&self, _topic_name: &str, _timeout: Duration) -> Option<Self::TopicType> {
        todo!()
    }
}

impl DomainParticipant for DomainParticipantImpl {
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
        // self.is_enabled.store(true, atomic::Ordering::Release);
        let is_enabled = self.is_enabled.clone();
        let default_transport = self.default_transport.clone();
        let metatraffic_transport = self.metatraffic_transport.clone();
        let guid_prefix = *self.rtps_participant.guid().prefix();
        let rtps_participant = self.rtps_participant.clone();
        let builtin_subscriber_storage = self.builtin_subscriber_storage.clone();
        let builtin_publisher_storage = self.builtin_publisher_storage.clone();
        let user_defined_subscriber_storage = self.user_defined_subscriber_storage.clone();
        let user_defined_publisher_storage = self.user_defined_publisher_storage.clone();

        std::thread::spawn(move || {
            while is_enabled.load(atomic::Ordering::SeqCst) {
                // send_builtin_data();
                for builtin_publisher in builtin_publisher_storage.iter() {
                    builtin_publisher.read().send_data(
                        rtps_participant.as_ref(),
                        metatraffic_transport.lock().unwrap().as_mut(),
                    );
                }

                //receive_builtin_data();
                if let Some((source_locator, message)) =
                    metatraffic_transport.lock().unwrap().read()
                {
                    crate::utils::message_receiver::MessageReceiver::new().process_message(
                        guid_prefix,
                        &builtin_subscriber_storage,
                        source_locator,
                        &message,
                    );
                }

                // send_user_defined_data();
                for user_defined_publisher in user_defined_publisher_storage.lock().unwrap().iter() {
                    user_defined_publisher.read().send_data(
                        rtps_participant.as_ref(),
                        metatraffic_transport.lock().unwrap().as_mut(),
                    );
                }
                crate::utils::message_sender::send_data(
                    rtps_participant.as_ref(),
                    &user_defined_publisher_storage.lock().unwrap(),
                    default_transport.lock().unwrap().as_mut(),
                );

                //receive_user_defined_data();
                if let Some((source_locator, message)) = default_transport.lock().unwrap().read() {
                    crate::utils::message_receiver::MessageReceiver::new().process_message(
                        guid_prefix,
                        &user_defined_subscriber_storage.lock().unwrap(),
                        source_locator,
                        &message,
                    );
                }

                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });
        self.is_enabled.store(true, atomic::Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rtps_impl::rtps_participant_impl::RtpsParticipantImpl, utils::transport::RtpsMessageRead,
    };
    use rust_dds_api::return_type::DDSError;
    use rust_rtps_pim::structure::types::Locator;

    //     struct MockDDSType;

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
        let mut domain_participant = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let mut qos = PublisherQos::default();
        qos.group_data.value = &[1, 2, 3, 4];
        domain_participant
            .set_default_publisher_qos(Some(qos.clone()))
            .unwrap();
        assert!(domain_participant.get_default_publisher_qos() == qos);
    }

    #[test]
    fn set_default_publisher_qos_none() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let mut domain_participant = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let mut qos = PublisherQos::default();
        qos.group_data.value = &[1, 2, 3, 4];
        domain_participant
            .set_default_publisher_qos(Some(qos.clone()))
            .unwrap();

        domain_participant.set_default_publisher_qos(None).unwrap();
        assert!(domain_participant.get_default_publisher_qos() == PublisherQos::default());
    }

    #[test]
    fn set_default_subscriber_qos_some_value() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let mut domain_participant = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let mut qos = SubscriberQos::default();
        qos.group_data.value = &[1, 2, 3, 4];
        domain_participant
            .set_default_subscriber_qos(Some(qos.clone()))
            .unwrap();
        assert_eq!(domain_participant.get_default_subscriber_qos(), qos);
    }

    #[test]
    fn set_default_subscriber_qos_none() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let mut domain_participant = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let mut qos = SubscriberQos::default();
        qos.group_data.value = &[1, 2, 3, 4];
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
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let mut domain_participant = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let mut qos = TopicQos::default();
        qos.topic_data.value = &[1, 2, 3, 4];
        domain_participant
            .set_default_topic_qos(Some(qos.clone()))
            .unwrap();
        assert_eq!(domain_participant.get_default_topic_qos(), qos);
    }

    #[test]
    fn set_default_topic_qos_inconsistent() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let mut domain_participant = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
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
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let mut domain_participant = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let mut qos = TopicQos::default();
        qos.topic_data.value = &[1, 2, 3, 4];
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
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );

        let publisher_counter_before = domain_participant
            .user_defined_publisher_counter
            .load(atomic::Ordering::Relaxed);
        let publisher = domain_participant.create_publisher(None, None, 0);

        let publisher_counter_after = domain_participant
            .user_defined_publisher_counter
            .load(atomic::Ordering::Relaxed);

        assert_eq!(
            domain_participant
                .user_defined_publisher_storage
                .lock()
                .unwrap()
                .len(),
            1
        );

        assert_ne!(publisher_counter_before, publisher_counter_after);
        assert!(publisher.is_some());
    }

    #[test]
    fn delete_publisher() {
        let rtps_participant = RtpsParticipantImpl::new([1; 12]);
        let domain_participant = DomainParticipantImpl::new(
            DomainParticipantQos::default(),
            rtps_participant,
            vec![],
            vec![],
            Box::new(MockTransport),
            Box::new(MockTransport),
        );
        let a_publisher = domain_participant.create_publisher(None, None, 0).unwrap();

        domain_participant.delete_publisher(&a_publisher).unwrap();
        assert_eq!(
            domain_participant
                .user_defined_publisher_storage
                .lock()
                .unwrap()
                .len(),
            0
        );
    }
}
