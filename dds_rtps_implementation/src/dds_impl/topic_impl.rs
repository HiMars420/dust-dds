use std::marker::PhantomData;

use rust_dds_api::{
    dcps_psm::{InconsistentTopicStatus, InstanceHandle, StatusMask},
    domain::domain_participant::DomainParticipant,
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::TopicQos,
    },
    return_type::DDSResult,
    topic::{topic_description::TopicDescription, topic_listener::TopicListener},
};

use crate::utils::shared_object::RtpsWeak;

pub struct TopicImpl {
    qos: TopicQos,
}

impl TopicImpl {
    pub fn new(qos: TopicQos) -> Self {
        Self { qos }
    }
}

pub struct TopicProxy<'t, T> {
    participant: &'t dyn DomainParticipant,
    topic_storage: RtpsWeak<TopicImpl>,
    phantom: PhantomData<&'t T>,
}

impl<'t, T> TopicProxy<'t, T> {
    pub fn new(
        participant: &'t dyn DomainParticipant,
        topic_storage: RtpsWeak<TopicImpl>,
    ) -> Self {
        Self {
            participant,
            topic_storage,
            phantom: PhantomData,
        }
    }
}

impl<'t, T: 'static> rust_dds_api::topic::topic::Topic<T> for TopicProxy<'t, T> {
    fn get_inconsistent_topic_status(
        &self,
        _status: &mut InconsistentTopicStatus,
    ) -> DDSResult<()> {
        todo!()
    }
}

impl<'t, T: 'static> TopicDescription<T> for TopicProxy<'t, T> {
    fn get_participant(&self) -> &dyn rust_dds_api::domain::domain_participant::DomainParticipant {
        self.participant
    }

    fn get_type_name(&self) -> DDSResult<&'static str> {
        // Ok(self
        //     .impl_ref
        //     .upgrade()
        //     .ok_or(DDSError::AlreadyDeleted)?
        //     .lock()
        //     .unwrap()
        //     .get_type_name())
        todo!()
    }

    fn get_name(&self) -> DDSResult<&'t str> {
        // Ok(self
        //     .impl_ref
        //     .upgrade()
        //     .ok_or(DDSError::AlreadyDeleted)?
        //     .lock()
        //     .unwrap()
        //     .get_name())
        todo!()
    }
}

impl<'t, T: 'static> Entity for TopicProxy<'t, T> {
    type Qos = TopicQos;
    type Listener = &'static dyn TopicListener<DataPIM = T>;

    fn set_qos(&self, qos: Option<Self::Qos>) -> DDSResult<()> {
        let qos = qos.unwrap_or_default();
        qos.is_consistent()?;
        let topic_storage = self.topic_storage.upgrade()?;
        let mut topic_storage_lock = topic_storage.lock();
        topic_storage_lock.qos = qos;
        Ok(())
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        Ok(self.topic_storage.upgrade()?.lock().qos.clone())
    }

    fn set_listener(
        &self,
        _a_listener: Option<Self::Listener>,
        _mask: StatusMask,
    ) -> DDSResult<()> {
        // Ok(self
        //     .impl_ref
        //     .upgrade()
        //     .ok_or(DDSError::AlreadyDeleted)?
        //     .lock()
        //     .unwrap()
        //     .set_listener(a_listener, mask))
        todo!()
    }

    fn get_listener(&self) -> DDSResult<Option<Self::Listener>> {
        // Ok(self
        //     .impl_ref
        //     .upgrade()
        //     .ok_or(DDSError::AlreadyDeleted)?
        //     .lock()
        //     .unwrap()
        //     .get_listener())
        todo!()
    }

    fn get_statuscondition(&self) -> StatusCondition {
        todo!()
    }

    fn get_status_changes(&self) -> StatusMask {
        todo!()
    }

    fn enable(&self) -> DDSResult<()> {
        todo!()
    }

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle> {
        todo!()
    }
}
