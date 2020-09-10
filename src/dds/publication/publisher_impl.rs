use crate::dds::types::{StatusKind, ReturnCode, Duration};
use crate::dds::domain::domain_participant::DomainParticipant;
use crate::dds::topic::topic::Topic;
use crate::dds::publication::data_writer_listener::DataWriterListener;
use crate::dds::publication::data_writer::DataWriter;
use crate::dds::infrastructure::qos_policy::QosPolicy;
use crate::dds::infrastructure::entity::Entity;
use crate::dds::infrastructure::entity::DomainEntity;
use crate::dds::publication::publisher_listener::PublisherListener;
pub struct PublisherImpl{
    
}

impl PublisherImpl {
    pub fn create_datawriter(
        &self,
        _a_topic: Topic,
        _qos: &[&dyn QosPolicy],
        _a_listener: Box<dyn DataWriterListener>,
        _mask: &[StatusKind]
    ) -> DataWriter {
        todo!()
    }

    pub fn delete_datawriter(
        &self,
        _a_datawriter: DataWriter
    ) -> ReturnCode {
        todo!()
    }

    pub fn lookup_datawriter(
        &self,
        _topic_name: String,
    ) -> DataWriter {
        todo!()
    }

    pub fn suspend_publications(&self,) -> ReturnCode {
        todo!()
    }

    pub fn resume_publications(&self,) -> ReturnCode {
        todo!()
    }

    pub fn begin_coherent_changes(&self,) -> ReturnCode {
        todo!()
    }

    pub fn end_coherent_changes(&self,) -> ReturnCode {
        todo!()
    }

    pub fn wait_for_acknowledgments(
        &self,
        _max_wait: Duration
    ) -> ReturnCode {
        todo!()
    }

    pub fn get_participant(&self,) -> DomainParticipant {
        todo!()
    }

    pub fn delete_contained_entities(&self,) -> ReturnCode {
        todo!()
    }

    pub fn set_default_datawriter_qos(
        &self,
        _qos_list: &[&dyn QosPolicy],
    ) -> ReturnCode {
        todo!()
    }

    pub fn get_default_datawriter_qos (
        &self,
        _qos_list: &mut [&dyn QosPolicy],
    ) -> ReturnCode {
        todo!()
    }

    pub fn copy_from_topic_qos(
        &self,
        _a_datawriter_qos: &mut [&dyn QosPolicy],
        _a_topic_qos: &[&dyn QosPolicy],
    ) -> ReturnCode {
        todo!()
    }
}

impl Entity for PublisherImpl{
    type Listener = Box<dyn PublisherListener>;

    fn set_qos(&self, _qos_list: &[&dyn QosPolicy]) -> ReturnCode {
        todo!()
    }

    fn get_qos(&self, _qos_list: &mut [&dyn QosPolicy]) -> ReturnCode {
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
        todo!()
    }

    fn get_instance_handle(&self, ) -> crate::dds::types::InstanceHandle {
        todo!()
    }
}

impl DomainEntity for PublisherImpl{}