use dds_api::{
    dcps_psm::{InconsistentTopicStatus, InstanceHandle, StatusMask},
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::TopicQos,
    },
    return_type::DdsResult,
    topic::{topic::Topic, topic_description::TopicDescription, topic_listener::TopicListener},
};

use crate::utils::shared_object::{DdsShared, DdsWeak};

pub struct TopicAttributes<DP> {
    _qos: TopicQos,
    type_name: &'static str,
    topic_name: String,
    parent_participant: DdsWeak<DP>,
}

impl<DP> TopicAttributes<DP> {
    pub fn new(
        qos: TopicQos,
        type_name: &'static str,
        topic_name: &str,
        parent_participant: DdsWeak<DP>,
    ) -> DdsShared<Self> {
        DdsShared::new(Self {
            _qos: qos,
            type_name,
            topic_name: topic_name.to_string(),
            parent_participant,
        })
    }
}

impl<DP> Topic for DdsShared<TopicAttributes<DP>> {
    fn get_inconsistent_topic_status(&self) -> DdsResult<InconsistentTopicStatus> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_inconsistent_topic_status()
        todo!()
    }
}

impl<DP> TopicDescription for DdsShared<TopicAttributes<DP>> {
    type DomainParticipant = DdsShared<DP>;

    fn get_participant(&self) -> DdsResult<Self::DomainParticipant> {
        self.parent_participant.clone().upgrade()
    }

    fn get_type_name(&self) -> DdsResult<&'static str> {
        Ok(self.type_name)
    }

    fn get_name(&self) -> DdsResult<String> {
        Ok(self.topic_name.clone())
    }
}

impl<DP> Entity for DdsShared<TopicAttributes<DP>> {
    type Qos = TopicQos;
    type Listener = Box<dyn TopicListener>;

    fn set_qos(&self, _qos: Option<Self::Qos>) -> DdsResult<()> {
        // rtps_shared_write_lock(&rtps_weak_upgrade(&self.topic_impl)?).set_qos(qos)
        todo!()
    }

    fn get_qos(&self) -> DdsResult<Self::Qos> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_qos()
        todo!()
    }

    fn set_listener(
        &self,
        _a_listener: Option<Self::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<()> {
        // rtps_shared_write_lock(&rtps_weak_upgrade(&self.topic_impl)?).set_listener(a_listener, mask)
        todo!()
    }

    fn get_listener(&self) -> DdsResult<Option<Self::Listener>> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_listener()
        todo!()
    }

    fn get_statuscondition(&self) -> DdsResult<StatusCondition> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_statuscondition()
        todo!()
    }

    fn get_status_changes(&self) -> DdsResult<StatusMask> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_status_changes()
        todo!()
    }

    fn enable(&self) -> DdsResult<()> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).enable()
        todo!()
    }

    fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        // rtps_shared_read_lock(&rtps_weak_upgrade(&self.topic_impl)?).get_instance_handle()
        todo!()
    }
}
