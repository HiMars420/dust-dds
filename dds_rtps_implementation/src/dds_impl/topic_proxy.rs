use std::marker::PhantomData;

use crate::utils::{rtps_structure::RtpsStructure, shared_object::RtpsWeak};
use rust_dds_api::{
    dcps_psm::{InconsistentTopicStatus, InstanceHandle, StatusMask},
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::TopicQos,
    },
    return_type::DDSResult,
    topic::{topic::Topic, topic_description::TopicDescription, topic_listener::TopicListener},
};

use super::domain_participant_proxy::{DomainParticipantAttributes, DomainParticipantProxy};

pub struct TopicAttributes<RTPS>
where
    RTPS: RtpsStructure,
{
    pub _qos: TopicQos,
    pub type_name: &'static str,
    pub topic_name: String,
    pub parent_participant: RtpsWeak<DomainParticipantAttributes<RTPS>>,
}

impl<RTPS> TopicAttributes<RTPS>
where
    RTPS: RtpsStructure,
{
    pub fn new(
        qos: TopicQos,
        type_name: &'static str,
        topic_name: &str,
        parent_participant: RtpsWeak<DomainParticipantAttributes<RTPS>>,
    ) -> Self {
        Self {
            _qos: qos,
            type_name,
            topic_name: topic_name.to_string(),
            parent_participant,
        }
    }
}

pub struct TopicProxy<Foo, RTPS>
where
    RTPS: RtpsStructure,
{
    topic_impl: RtpsWeak<TopicAttributes<RTPS>>,
    phantom: PhantomData<Foo>,
}

// Not automatically derived because in that case it is only available if Foo: Clone
impl<Foo, RTPS> Clone for TopicProxy<Foo, RTPS>
where
    RTPS: RtpsStructure,
{
    fn clone(&self) -> Self {
        Self {
            topic_impl: self.topic_impl.clone(),
            phantom: self.phantom.clone(),
        }
    }
}

impl<Foo, RTPS> TopicProxy<Foo, RTPS>
where
    RTPS: RtpsStructure,
{
    pub fn new(topic_impl: RtpsWeak<TopicAttributes<RTPS>>) -> Self {
        Self {
            topic_impl,
            phantom: PhantomData,
        }
    }
}

impl<Foo, RTPS> AsRef<RtpsWeak<TopicAttributes<RTPS>>> for TopicProxy<Foo, RTPS>
where
    RTPS: RtpsStructure,
{
    fn as_ref(&self) -> &RtpsWeak<TopicAttributes<RTPS>> {
        &self.topic_impl
    }
}

impl<Foo, RTPS> Topic for TopicProxy<Foo, RTPS>
where
    RTPS: RtpsStructure,
{
    fn get_inconsistent_topic_status(&self) -> DDSResult<InconsistentTopicStatus> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_inconsistent_topic_status()
        todo!()
    }
}

impl<Foo, RTPS> TopicDescription for TopicProxy<Foo, RTPS>
where
    RTPS: RtpsStructure,
{
    type DomainParticipant = DomainParticipantProxy<RTPS>;

    fn get_participant(&self) -> Self::DomainParticipant {
        todo!()
        // self.participant.clone()
    }

    fn get_type_name(&self) -> DDSResult<&'static str> {
        Ok(self.topic_impl.upgrade()?.read_lock().type_name)
    }

    fn get_name(&self) -> DDSResult<String> {
        Ok(self.topic_impl.upgrade()?.read_lock().topic_name.clone())
    }
}

impl<Foo, RTPS> Entity for TopicProxy<Foo, RTPS>
where
    RTPS: RtpsStructure,
{
    type Qos = TopicQos;
    type Listener = Box<dyn TopicListener>;

    fn set_qos(&mut self, _qos: Option<Self::Qos>) -> DDSResult<()> {
        // rtps_shared_write_lock(&rtps_weak_upgrade(&self.topic_impl)?).set_qos(qos)
        todo!()
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_qos()
        todo!()
    }

    fn set_listener(
        &self,
        _a_listener: Option<Self::Listener>,
        _mask: StatusMask,
    ) -> DDSResult<()> {
        // rtps_shared_write_lock(&rtps_weak_upgrade(&self.topic_impl)?).set_listener(a_listener, mask)
        todo!()
    }

    fn get_listener(&self) -> DDSResult<Option<Self::Listener>> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_listener()
        todo!()
    }

    fn get_statuscondition(&self) -> DDSResult<StatusCondition> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_statuscondition()
        todo!()
    }

    fn get_status_changes(&self) -> DDSResult<StatusMask> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_status_changes()
        todo!()
    }

    fn enable(&self) -> DDSResult<()> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).enable()
        todo!()
    }

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_instance_handle()
        todo!()
    }
}
