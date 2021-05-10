use std::sync::Mutex;

use rust_dds_api::{
    builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    dcps_psm::{DomainId, InstanceHandle, StatusMask, Time},
    domain::domain_participant_listener::DomainParticipantListener,
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::{DomainParticipantQos, PublisherQos, SubscriberQos, TopicQos},
    },
    publication::{publisher::Publisher, publisher_listener::PublisherListener},
    return_type::{DDSError, DDSResult},
    topic::topic_description::TopicDescription,
};

use crate::{
    rtps_impl::rtps_participant_impl::RTPSParticipantImpl, utils::shared_object::RtpsShared,
};

use super::{publisher_impl::PublisherImpl, writer_group_factory::WriterGroupFactory};

pub struct DomainParticipantImpl<'dp, PSM: rust_rtps_pim::PIM> {
    writer_group_factory: Mutex<WriterGroupFactory<'dp, PSM>>,
    rtps_participant_impl: Mutex<RTPSParticipantImpl<'dp, PSM>>,
}

impl<'dp, PSM: rust_rtps_pim::PIM> DomainParticipantImpl<'dp, PSM> {
    pub fn new(guid_prefix: PSM::GuidPrefix) -> Self {
        Self {
            writer_group_factory: Mutex::new(WriterGroupFactory::new(guid_prefix)),
            rtps_participant_impl: Mutex::new(RTPSParticipantImpl::new()),
        }
    }
}

impl<'p, 'dp: 'p, PSM: rust_rtps_pim::PIM>
    rust_dds_api::domain::domain_participant::PublisherFactory<'p, 'dp>
    for DomainParticipantImpl<'dp, PSM>
{
    type PublisherType = PublisherImpl<'p, 'dp, PSM>;
    fn create_publisher(
        &'p self,
        qos: Option<PublisherQos<'dp>>,
        a_listener: Option<&'dp (dyn PublisherListener + 'dp)>,
        mask: StatusMask,
    ) -> Option<Self::PublisherType> {
        let writer_group = self
            .writer_group_factory
            .lock()
            .unwrap()
            .create_writer_group(qos, a_listener, mask)
            .ok()?;
        let writer_group_shared = RtpsShared::new(writer_group);
        let writer_group_weak = writer_group_shared.downgrade();
        self.rtps_participant_impl
            .lock()
            .unwrap()
            .add_writer_group(writer_group_shared);
        Some(PublisherImpl::new(self, writer_group_weak))
    }

    fn delete_publisher(&self, a_publisher: &Self::PublisherType) -> DDSResult<()> {
        if std::ptr::eq(a_publisher.get_participant(), self) {
            self.rtps_participant_impl
                .lock()
                .unwrap()
                .delete_writer_group(&a_publisher.get_instance_handle()?)
        } else {
            Err(DDSError::PreconditionNotMet(
                "Publisher can only be deleted from its parent participant",
            ))
        }
    }
}

impl<'dp, PSM: rust_rtps_pim::PIM> rust_dds_api::domain::domain_participant::DomainParticipant<'dp>
    for DomainParticipantImpl<'dp, PSM>
{
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

    fn set_default_publisher_qos(&self, _qos: Option<PublisherQos<'dp>>) -> DDSResult<()> {
        // *self.default_publisher_qos.lock().unwrap() = qos.unwrap_or_default();
        Ok(())
    }

    fn get_default_publisher_qos(&self) -> PublisherQos<'dp> {
        // self.default_publisher_qos.lock().unwrap().clone()
        todo!()
    }

    fn set_default_subscriber_qos(&self, _qos: Option<SubscriberQos<'dp>>) -> DDSResult<()> {
        // *self.default_subscriber_qos.lock().unwrap() = qos.unwrap_or_default();
        Ok(())
    }

    fn get_default_subscriber_qos(&self) -> SubscriberQos<'dp> {
        // self.default_subscriber_qos.lock().unwrap().clone()
        todo!()
    }

    fn set_default_topic_qos(&self, qos: Option<TopicQos<'dp>>) -> DDSResult<()> {
        let topic_qos = qos.unwrap_or_default();
        topic_qos.is_consistent()?;
        // *self.default_topic_qos.lock().unwrap() = topic_qos;
        Ok(())
    }

    fn get_default_topic_qos(&self) -> TopicQos<'dp> {
        // self.default_topic_qos.lock().unwrap().clone()
        todo!()
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

impl<'dp, PSM: rust_rtps_pim::PIM> Entity for DomainParticipantImpl<'dp, PSM> {
    type Qos = DomainParticipantQos<'dp>;
    type Listener = &'dp (dyn DomainParticipantListener + 'dp);

    fn set_qos(&self, _qos: Option<Self::Qos>) -> DDSResult<()> {
        // self.0.lock().unwrap().set_qos(qos)
        todo!()
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        // Ok(self.0.lock().unwrap().get_qos())
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
        // self.0.lock().unwrap().enable()
        todo!()
    }

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use rust_dds_api::domain::domain_participant::DomainParticipant;
    // use rust_dds_api::return_type::DDSError;
    use rust_rtps_udp_psm::RtpsUdpPsm;

    use super::*;

    struct MockDDSType;

    // #[test]
    // fn set_default_publisher_qos_some_value() {
    //     let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
    //         DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
    //     let mut qos = PublisherQos::default();
    //     qos.group_data.value = &[1, 2, 3, 4];
    //     domain_participant_impl
    //         .set_default_publisher_qos(Some(qos.clone()))
    //         .unwrap();
    //     assert!(
    //         *domain_participant_impl
    //             .default_publisher_qos
    //             .lock()
    //             .unwrap()
    //             == qos
    //     );
    // }

    // #[test]
    // fn set_default_publisher_qos_none() {
    //     let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
    //         DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
    //     let mut qos = PublisherQos::default();
    //     qos.group_data.value = &[1, 2, 3, 4];
    //     domain_participant_impl
    //         .set_default_publisher_qos(Some(qos.clone()))
    //         .unwrap();

    //     domain_participant_impl
    //         .set_default_publisher_qos(None)
    //         .unwrap();
    //     assert!(
    //         *domain_participant_impl
    //             .default_publisher_qos
    //             .lock()
    //             .unwrap()
    //             == PublisherQos::default()
    //     );
    // }

    // #[test]
    // fn get_default_publisher_qos() {
    //     let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
    //         DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
    //     let mut qos = PublisherQos::default();
    //     qos.group_data.value = &[1, 2, 3, 4];
    //     domain_participant_impl
    //         .set_default_publisher_qos(Some(qos.clone()))
    //         .unwrap();
    //     assert!(domain_participant_impl.get_default_publisher_qos() == qos);
    // }

    // #[test]
    // fn set_default_subscriber_qos_some_value() {
    //     let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
    //         DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
    //     let mut qos = SubscriberQos::default();
    //     qos.group_data.value = &[1, 2, 3, 4];
    //     domain_participant_impl
    //         .set_default_subscriber_qos(Some(qos.clone()))
    //         .unwrap();
    //     assert!(
    //         *domain_participant_impl
    //             .default_subscriber_qos
    //             .lock()
    //             .unwrap()
    //             == qos
    //     );
    // }

    // #[test]
    // fn set_default_subscriber_qos_none() {
    //     let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
    //         DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
    //     let mut qos = SubscriberQos::default();
    //     qos.group_data.value = &[1, 2, 3, 4];
    //     domain_participant_impl
    //         .set_default_subscriber_qos(Some(qos.clone()))
    //         .unwrap();

    //     domain_participant_impl
    //         .set_default_subscriber_qos(None)
    //         .unwrap();
    //     assert!(
    //         *domain_participant_impl
    //             .default_subscriber_qos
    //             .lock()
    //             .unwrap()
    //             == SubscriberQos::default()
    //     );
    // }

    // #[test]
    // fn get_default_subscriber_qos() {
    //     let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
    //         DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
    //     let mut qos = SubscriberQos::default();
    //     qos.group_data.value = &[1, 2, 3, 4];
    //     domain_participant_impl
    //         .set_default_subscriber_qos(Some(qos.clone()))
    //         .unwrap();
    //     assert!(domain_participant_impl.get_default_subscriber_qos() == qos);
    // }

    // #[test]
    // fn set_default_topic_qos_some_value() {
    //     let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
    //         DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
    //     let mut qos = TopicQos::default();
    //     qos.topic_data.value = &[1, 2, 3, 4];
    //     domain_participant_impl
    //         .set_default_topic_qos(Some(qos.clone()))
    //         .unwrap();
    //     assert!(*domain_participant_impl.default_topic_qos.lock().unwrap() == qos);
    // }

    // #[test]
    // fn set_default_topic_qos_inconsistent() {
    //     let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
    //         DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
    //     let mut qos = TopicQos::default();
    //     qos.resource_limits.max_samples_per_instance = 2;
    //     qos.resource_limits.max_samples = 1;
    //     let set_default_topic_qos_result =
    //         domain_participant_impl.set_default_topic_qos(Some(qos.clone()));
    //     assert!(set_default_topic_qos_result == Err(DDSError::InconsistentPolicy));
    // }

    // #[test]
    // fn set_default_topic_qos_none() {
    //     let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
    //         DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
    //     let mut qos = TopicQos::default();
    //     qos.topic_data.value = &[1, 2, 3, 4];
    //     domain_participant_impl
    //         .set_default_topic_qos(Some(qos.clone()))
    //         .unwrap();

    //     domain_participant_impl.set_default_topic_qos(None).unwrap();
    //     assert!(*domain_participant_impl.default_topic_qos.lock().unwrap() == TopicQos::default());
    // }

    // #[test]
    // fn get_default_topic_qos() {
    //     let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
    //         DomainParticipantImpl::new(RTPSParticipantImpl::new([1; 12]));
    //     let mut qos = TopicQos::default();
    //     qos.topic_data.value = &[1, 2, 3, 4];
    //     domain_participant_impl
    //         .set_default_topic_qos(Some(qos.clone()))
    //         .unwrap();
    //     assert!(domain_participant_impl.get_default_topic_qos() == qos);
    // }

    #[test]
    fn create_publisher() {
        let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
            DomainParticipantImpl::new([1; 12]);
        let publisher = domain_participant_impl.create_publisher(None, None, 0);

        assert!(publisher.is_some())
    }

    #[test]
    fn create_topic() {
        let domain_participant_impl: DomainParticipantImpl<RtpsUdpPsm> =
            DomainParticipantImpl::new([1; 12]);
        let topic =
            domain_participant_impl.create_topic::<MockDDSType>("topic_name", None, None, 0);
        assert!(topic.is_some());
    }
}
