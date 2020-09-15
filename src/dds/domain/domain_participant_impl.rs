use std::any::Any;
use std::sync::{Arc, Mutex};

use crate::dds::types::{StatusKind, StatusMask, ReturnCode, Duration, InstanceHandle, DomainId, Time};
use crate::dds::topic::topic::Topic;
use crate::dds::topic::qos::TopicQos;
use crate::dds::topic::topic_listener::TopicListener;
use crate::dds::topic::topic_description::TopicDescription;
use crate::dds::subscription::subscriber::Subscriber;
use crate::dds::subscription::subscriber_impl::SubscriberImpl;
use crate::dds::subscription::subscriber::qos::SubscriberQos;
use crate::dds::subscription::subscriber_listener::SubscriberListener;
use crate::dds::publication::publisher::Publisher;
use crate::dds::publication::publisher_impl::PublisherImpl;
use crate::dds::publication::publisher::qos::PublisherQos;
use crate::dds::publication::publisher_listener::PublisherListener;
use crate::dds::infrastructure::entity::Entity;
use crate::dds::domain::domain_participant_listener::DomainParticipantListener;
use crate::dds::infrastructure::listener::NoListener;
use crate::dds::builtin_topics::{TopicBuiltinTopicData, ParticipantBuiltinTopicData};

use super::domain_participant::qos::DomainParticipantQos;

pub struct DomainParticipantImpl{
    domain_id: DomainId,
    qos: DomainParticipantQos,
    a_listener: Box<dyn DomainParticipantListener>,
    mask: StatusMask,
    publisher_list: Mutex<Vec<Arc<PublisherImpl>>>,
    publisher_default_qos: Mutex<PublisherQos>,
    subscriber_list: Mutex<Vec<Arc<SubscriberImpl>>>,
    subscriber_default_qos: Mutex<SubscriberQos>,
}

impl DomainParticipantImpl{
    pub fn create_publisher(
        this: &Arc<DomainParticipantImpl>,
        _qos_list: PublisherQos,
        _a_listener: impl PublisherListener,
        _mask: StatusMask
    ) -> Publisher {
        let publisher_impl = Arc::new(PublisherImpl::new(Arc::downgrade(this)));
        let publisher = Publisher(Arc::downgrade(&publisher_impl));

        this.publisher_list.lock().unwrap().push(publisher_impl);

        publisher
    }

    pub fn delete_publisher(
        this: &Arc<DomainParticipantImpl>,
        a_publisher: &Publisher
    ) -> ReturnCode {
        // TODO: Shouldn't be deleted if it still contains entities but can't yet be done because the publisher is not implemented
        let mut publisher_list = this.publisher_list.lock().unwrap();
        let index = publisher_list.iter().position(|x| std::ptr::eq(x.as_ref(), a_publisher.0.upgrade().unwrap().as_ref())).unwrap();
        publisher_list.swap_remove(index);
        ReturnCode::Ok
    }

    pub fn create_subscriber(
        this: &Arc<DomainParticipantImpl>,
        _qos_list: SubscriberQos,
        _a_listener: impl SubscriberListener,
        _mask: StatusMask
    ) -> Subscriber {
        let subscriber_impl = Arc::new(SubscriberImpl::new(Arc::downgrade(this)));
        let subscriber = Subscriber(Arc::downgrade(&subscriber_impl));

        this.subscriber_list.lock().unwrap().push(subscriber_impl);

        subscriber
    }

    pub fn delete_subscriber(
        this: &Arc<DomainParticipantImpl>,
        a_subscriber: &Subscriber,
    ) -> ReturnCode {
        // TODO: Shouldn't be deleted if it still contains entities but can't yet be done because the publisher is not implemented
        let mut subscriber_list = this.subscriber_list.lock().unwrap();
        let index = subscriber_list.iter().position(|x| std::ptr::eq(x.as_ref(), a_subscriber.0.upgrade().unwrap().as_ref())).unwrap();
        subscriber_list.swap_remove(index);
        ReturnCode::Ok
    }

    pub fn create_topic(
        _this: &Arc<DomainParticipantImpl>,
        _topic_name: String,
        _type_name: String,
        _qos_list: TopicQos,
        _a_listener: Box<dyn TopicListener>,
        _mask: &[StatusKind]
    ) -> Topic {
        todo!()
    }

    pub fn delete_topic(
        _this: &Arc<DomainParticipantImpl>,
        _a_topic: Topic,
    ) -> ReturnCode {
        todo!()
    }

    pub fn find_topic(
        _this: &Arc<DomainParticipantImpl>,
        _topic_name: String,
        _timeout: Duration,
    ) -> Topic {
        todo!()
    }

    pub fn lookup_topicdescription(
        _this: &Arc<DomainParticipantImpl>,
        _name: String,
    ) -> &dyn TopicDescription {
        todo!()
    }

    pub fn get_builtin_subscriber(_this: &Arc<DomainParticipantImpl>,) -> Subscriber {
        todo!()
    }

    pub fn ignore_participant(
        _this: &Arc<DomainParticipantImpl>,
        _handle: InstanceHandle
    ) -> ReturnCode{
        todo!()
    }

    pub fn ignore_topic(
        _this: &Arc<DomainParticipantImpl>,
        _handle: InstanceHandle
    ) -> ReturnCode{
        todo!()
    }

    pub fn ignore_publication(
        _this: &Arc<DomainParticipantImpl>,
        _handle: InstanceHandle
    ) -> ReturnCode{
        todo!()
    }

    pub fn ignore_subscription(
        _this: &Arc<DomainParticipantImpl>,
        _handle: InstanceHandle
    ) -> ReturnCode{
        todo!()
    }

    pub fn get_domain_id(this: &Arc<DomainParticipantImpl>) -> DomainId {
        this.domain_id
    }

    pub fn delete_contained_entities(_this: &Arc<DomainParticipantImpl>) -> ReturnCode {
        todo!()   
    }

    pub fn assert_liveliness(_this: &Arc<DomainParticipantImpl>) -> ReturnCode {
        todo!()   
    }

    pub fn set_default_publisher_qos(
        this: &Arc<DomainParticipantImpl>,
        qos: PublisherQos,
    ) -> ReturnCode {
        *this.publisher_default_qos.lock().unwrap() = qos;
        ReturnCode::Ok
    }

    pub fn get_default_publisher_qos(
        this: &Arc<DomainParticipantImpl>,
        qos: &mut PublisherQos,
    ) -> ReturnCode {
        qos.clone_from(&this.publisher_default_qos.lock().unwrap());
        ReturnCode::Ok
    }

    pub fn set_default_subscriber_qos(
        _this: &Arc<DomainParticipantImpl>,
        _qos_list: SubscriberQos,
    ) -> ReturnCode {
        todo!()
    }

    pub fn get_default_subscriber_qos(
        _this: &Arc<DomainParticipantImpl>,
        _qos_list: &mut SubscriberQos,
    ) -> ReturnCode {
        todo!()
    }

    pub fn set_default_topic_qos(
        _this: &Arc<DomainParticipantImpl>,
        _qos_list: TopicQos,
    ) -> ReturnCode {
        todo!()
    }

    pub fn get_default_topic_qos(
        _this: &Arc<DomainParticipantImpl>,
        _qos_list: &mut TopicQos,
    ) -> ReturnCode {
        todo!()
    }

    pub fn get_discovered_participants(
        _this: &Arc<DomainParticipantImpl>,
        _participant_handles: &mut [InstanceHandle]
    ) -> ReturnCode {
        todo!()
    }

    pub fn get_discovered_participant_data(
        _this: &Arc<DomainParticipantImpl>,
        _participant_data: ParticipantBuiltinTopicData,
        _participant_handle: InstanceHandle
    ) -> ReturnCode {
        todo!()
    }

    pub fn get_discovered_topics(
        _this: &Arc<DomainParticipantImpl>,
        _topic_handles: &mut [InstanceHandle]
    ) -> ReturnCode {
        todo!()
    }

    pub fn get_discovered_topic_data(
        _this: &Arc<DomainParticipantImpl>,
        _topic_data: TopicBuiltinTopicData,
        _topic_handle: InstanceHandle
    ) -> ReturnCode {
        todo!()
    }

    pub fn contains_entity(
        _this: &Arc<DomainParticipantImpl>,
        _a_handle: InstanceHandle
    ) -> bool {
        todo!()
    }

    pub fn get_current_time(
        _this: &Arc<DomainParticipantImpl>,
        _current_time: Time,
    ) -> ReturnCode {
        todo!()
    }

    //////////////// From here on are the functions that do not belong to the standard API
    pub(crate) fn new(
        domain_id: DomainId,
        qos: DomainParticipantQos,
        a_listener: impl DomainParticipantListener,
        mask: StatusMask,
    ) -> Self {
        
        if !Any::is::<NoListener>(&a_listener) {
            println!("TODO: Use the real listener")
        }

        Self {
            domain_id,
            qos,
            a_listener: Box::new(a_listener),
            mask,
            publisher_list: Mutex::new(Vec::new()),
            publisher_default_qos: Mutex::new(PublisherQos::default()),
            subscriber_list: Mutex::new(Vec::new()),
            subscriber_default_qos: Mutex::new(SubscriberQos::default()),
        }
    }

}

impl Entity for DomainParticipantImpl
{
    type Qos = DomainParticipantQos;
    type Listener = Box<dyn DomainParticipantListener>;

    fn set_qos(&self, _qos_list: Self::Qos) -> ReturnCode {
        todo!()
    }

    fn get_qos(&self, _qos_list: &mut Self::Qos) -> ReturnCode {
        todo!()
    }

    fn set_listener(&self, _a_listener: Self::Listener, _mask: &[StatusKind]) -> ReturnCode {
        todo!()
    }

    fn get_listener(&self, ) -> Self::Listener {
        todo!()
    }

    fn get_statuscondition(&self, ) -> crate::dds::infrastructure::entity::StatusCondition {
        todo!()
    }

    fn get_status_changes(&self, ) -> StatusKind {
        todo!()
    }

    fn enable(&self, ) -> ReturnCode {
        //TODO: This is to prevent the ParticipantFactory test from panicking
        ReturnCode::Ok
    }

    fn get_instance_handle(&self, ) -> InstanceHandle {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dds::infrastructure::listener::NoListener;

    #[test]
    fn create_publisher() {
        let domain_participant_impl = Arc::new(DomainParticipantImpl::new(0, DomainParticipantQos::default(), NoListener, 0));

        {
            assert_eq!(domain_participant_impl.publisher_list.lock().unwrap().len(), 0);
            let _publisher = DomainParticipantImpl::create_publisher(&domain_participant_impl,PublisherQos::default(), NoListener, 0);
            assert_eq!(domain_participant_impl.publisher_list.lock().unwrap().len(), 1);
        }

        assert_eq!(domain_participant_impl.publisher_list.lock().unwrap().len(), 0);
    }

    #[test]
    fn create_subscriber() {
        let domain_participant_impl = Arc::new(DomainParticipantImpl::new(0, DomainParticipantQos::default(), NoListener, 0));

        {
            assert_eq!(domain_participant_impl.subscriber_list.lock().unwrap().len(), 0);
            let _subscriber = DomainParticipantImpl::create_subscriber(&domain_participant_impl,SubscriberQos::default(), NoListener, 0);
            assert_eq!(domain_participant_impl.subscriber_list.lock().unwrap().len(), 1);
        }

        assert_eq!(domain_participant_impl.subscriber_list.lock().unwrap().len(), 0);
    }
}