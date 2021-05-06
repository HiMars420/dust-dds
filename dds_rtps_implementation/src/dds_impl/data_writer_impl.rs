use std::sync::{Mutex, Weak};

use rust_dds_api::{
    builtin_topics::SubscriptionBuiltinTopicData,
    dcps_psm::{
        Duration, InstanceHandle, LivelinessLostStatus, OfferedDeadlineMissedStatus,
        OfferedIncompatibleQosStatus, PublicationMatchedStatus, StatusMask, Time,
    },
    infrastructure::{entity::StatusCondition, qos::DataWriterQos},
    publication::data_writer_listener::DataWriterListener,
    return_type::{DDSError, DDSResult},
};
use rust_rtps_pim::behavior::RTPSWriter;

use crate::rtps_impl::rtps_writer_impl::RTPSWriterImpl;

use super::{publisher_impl::PublisherImpl, topic_impl::TopicImpl};

pub struct DataWriterImpl<
    'datawriter,
    'publisher: 'datawriter,
    'topic: 'datawriter,
    'participant: 'publisher,
    T: 'topic,
    PSM: rust_rtps_pim::PIM,
> {
    pub(crate) parent: &'datawriter PublisherImpl<'publisher, 'participant, PSM>,
    pub(crate) topic: &'datawriter TopicImpl<'topic, 'participant, T, PSM>,
    pub(crate) rtps_writer: Weak<Mutex<RTPSWriterImpl<PSM>>>,
}

impl<
        'datawriter,
        'publisher: 'datawriter,
        'topic: 'datawriter,
        'participant: 'publisher,
        T: 'topic,
        PSM: rust_rtps_pim::PIM,
    > DataWriterImpl<'datawriter, 'publisher, 'topic, 'participant, T, PSM>
{
    pub fn new(
        parent: &'datawriter PublisherImpl<'publisher, 'participant, PSM>,
        topic: &'datawriter TopicImpl<'topic, 'participant, T, PSM>,
        rtps_writer: Weak<Mutex<RTPSWriterImpl<PSM>>>,
    ) -> Self {
        Self {
            parent,
            rtps_writer,
            topic,
        }
    }
}

impl<
        'datawriter,
        'publisher: 'datawriter,
        'topic: 'datawriter,
        'participant: 'publisher,
        T: 'topic,
        PSM: rust_rtps_pim::PIM,
    >
    rust_dds_api::publication::data_writer::DataWriter<
        'datawriter,
        'publisher,
        'topic,
        'participant,
        T,
    > for DataWriterImpl<'datawriter, 'publisher, 'topic, 'participant, T, PSM>
{
    fn register_instance(&self, _instance: T) -> DDSResult<Option<InstanceHandle>> {
        todo!()
        // let timestamp = self.parent.0.parent.get_current_time()?;
        // self.register_instance_w_timestamp(instance, timestamp)
    }

    fn register_instance_w_timestamp(
        &self,
        _instance: T,
        _timestamp: Time,
    ) -> DDSResult<Option<InstanceHandle>> {
        let writer = self.rtps_writer.upgrade().ok_or(DDSError::AlreadyDeleted)?;
        let writer_guard = writer.lock().unwrap();
        let _c = writer_guard.writer_cache();
        todo!()
    }

    fn unregister_instance(&self, _instance: T, _handle: Option<InstanceHandle>) -> DDSResult<()> {
        todo!()
    }

    fn unregister_instance_w_timestamp(
        &self,
        _instance: T,
        _handle: Option<InstanceHandle>,
        _timestamp: Time,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_key_value(&self, _key_holder: &mut T, _handle: InstanceHandle) -> DDSResult<()> {
        todo!()
    }

    fn lookup_instance(&self, _instance: &T) -> DDSResult<Option<InstanceHandle>> {
        todo!()
    }

    fn write(&self, _data: T, _handle: Option<InstanceHandle>) -> DDSResult<()> {
        todo!()
    }

    fn write_w_timestamp(
        &self,
        _data: T,
        _handle: Option<InstanceHandle>,
        _timestamp: Time,
    ) -> DDSResult<()> {
        // let writer = self.rtps_writer.upgrade().ok_or(DDSError::AlreadyDeleted)?;
        // let mut writer_guard = writer.lock().unwrap();
        // let cc = writer_guard.new_change(ChangeKind::Alive, vec![0, 1, 2, 3], vec![], 0);
        // writer_guard.writer_cache_mut().add_change(cc);
        Ok(())
    }

    fn dispose(&self, _data: T, _handle: Option<InstanceHandle>) -> DDSResult<()> {
        todo!()
    }

    fn dispose_w_timestamp(
        &self,
        _data: T,
        _handle: Option<InstanceHandle>,
        _timestamp: Time,
    ) -> DDSResult<()> {
        todo!()
    }

    fn wait_for_acknowledgments(&self, _max_wait: Duration) -> DDSResult<()> {
        todo!()
    }

    fn get_liveliness_lost_status(&self, _status: &mut LivelinessLostStatus) -> DDSResult<()> {
        todo!()
    }

    fn get_offered_deadline_missed_status(
        &self,
        _status: &mut OfferedDeadlineMissedStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_offered_incompatible_qos_status(
        &self,
        _status: &mut OfferedIncompatibleQosStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_publication_matched_status(
        &self,
        _status: &mut PublicationMatchedStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn assert_liveliness(&self) -> DDSResult<()> {
        todo!()
    }

    fn get_matched_subscription_data(
        &self,
        _subscription_data: SubscriptionBuiltinTopicData,
        _subscription_handle: InstanceHandle,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_topic(&self) -> &dyn rust_dds_api::topic::topic::Topic<T> {
        todo!()
    }

    fn get_publisher(&self) -> &dyn rust_dds_api::publication::publisher::Publisher {
        todo!()
    }

    fn get_matched_subscriptions(
        &self,
        _subscription_handles: &mut [InstanceHandle],
    ) -> DDSResult<()> {
        todo!()
    }


}

impl<
        'datawriter,
        'publisher: 'datawriter,
        'topic: 'datawriter,
        'participant: 'publisher,
        T: 'topic,
        PSM: rust_rtps_pim::PIM,
    > rust_dds_api::infrastructure::entity::Entity
    for DataWriterImpl<'datawriter, 'publisher, 'topic, 'participant, T, PSM>
{
    type Qos = DataWriterQos<'datawriter>;
    type Listener = &'datawriter (dyn DataWriterListener<DataType = T> + 'datawriter);

    fn set_qos(&self, _qos: Option<Self::Qos>) -> DDSResult<()> {
        todo!()
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        todo!()
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

    fn enable(&self) -> DDSResult<()> {
        todo!()
    }

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle> {
        todo!()
    }
}

impl<
        'datawriter,
        'publisher: 'datawriter,
        'topic: 'datawriter,
        'participant: 'publisher,
        T: 'topic,
        PSM: rust_rtps_pim::PIM,
    > rust_dds_api::publication::data_writer::AnyDataWriter
    for DataWriterImpl<'datawriter, 'publisher, 'topic, 'participant, T, PSM>
{
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::{
    //     dds_impl::domain_participant_impl::DomainParticipantImpl,
    //     rtps_impl::rtps_participant_impl::RTPSParticipantImpl,
    // };
    // use rust_dds_api::{
    //     domain::domain_participant::DomainParticipant,
    //     publication::{data_writer::DataWriter, publisher::Publisher},
    // };
    // use rust_rtps_udp_psm::RtpsUdpPsm;

    struct MockData;

    // impl DDSType for MockData {
    //     fn type_name() -> &'static str {
    //         todo!()
    //     }

    //     fn has_key() -> bool {
    //         todo!()
    //     }

    //     fn key(&self) -> Vec<u8> {
    //         todo!()
    //     }

    //     fn serialize(&self) -> Vec<u8> {
    //         todo!()
    //     }

    //     fn deserialize(_data: Vec<u8>) -> Self {
    //         todo!()
    //     }
    // }

    #[test]
    fn write_w_timestamp() {
        // let domain_participant: DomainParticipantImpl<RtpsUdpPsm> =
        //     DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
        // let publisher = domain_participant.create_publisher(None, None, 0).unwrap();
        // let a_topic = domain_participant
        //     .create_topic::<MockData>("Test", None, None, 0)
        //     .unwrap();

        // let data_writer = publisher
        //     .create_datawriter(&a_topic, None, None, 0)
        //     .unwrap();

        // data_writer
        //     .write_w_timestamp(MockData, None, Time { sec: 0, nanosec: 0 })
        //     .unwrap();

        // assert!(data_writer
        //     .rtps_writer
        //     .upgrade()
        //     .unwrap()
        //     .lock()
        //     .unwrap()
        //     .writer_cache()
        //     .get_change(&(1i64.into()))
        //     .is_some());
    }
}
