use std::sync::Arc;

use crate::dds_infrastructure::entity::{Entity, StatusCondition};
use crate::dds_infrastructure::qos::{DomainParticipantQos, PublisherQos, SubscriberQos, TopicQos};
use crate::dds_infrastructure::domain_participant_listener::DomainParticipantListener;
use crate::dds_infrastructure::status::StatusMask;

use crate::types::{DDSType, DomainId, Duration, InstanceHandle, ReturnCode, Time};
use crate::builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData};

use crate::rtps::transport::udp::UdpTransport;
use crate::rtps::transport::Transport;

use crate::dds_rtps_implementation::rtps_object::RtpsObject;
use crate::dds_rtps_implementation::rtps_publisher::{RtpsPublisher, RtpsPublisherInner};
use crate::dds_rtps_implementation::rtps_subscriber::RtpsSubscriber;
use crate::dds_rtps_implementation::rtps_topic::RtpsTopic;

pub struct RtpsParticipant {
    userdata_transport: Box<dyn Transport>,
    metatraffic_transport: Box<dyn Transport>,
    publisher_list: Arc<[RtpsObject<RtpsPublisherInner>; 32]>,
}

impl RtpsParticipant {
    pub fn new(
        domain_id: DomainId,
        //     qos: DomainParticipantQos,
        //     a_listener: impl DomainParticipantListener,
        //     mask: StatusMask,
        //     enabled: bool,
    ) -> Option<Self> {
        let interface = "Ethernet";
        let userdata_transport =
            Box::new(UdpTransport::default_userdata_transport(domain_id, interface).unwrap());
        let metatraffic_transport =
            Box::new(UdpTransport::default_metatraffic_transport(domain_id, interface).unwrap());
        // let domain_tag = "".to_string();
        // let lease_duration = Duration {
        //     sec: 30,
        //     nanosec: 0,
        // };

        // let participant = RtpsParticipant::new(domain_id);

        // // if enabled {
        // //     new_participant.enable().ok()?;
        // // }

        Some(Self {
            userdata_transport,
            metatraffic_transport,
            publisher_list: Arc::new(Default::default()),
        })
    }

    pub fn create_publisher<'a>(&'a self, _qos: Option<&PublisherQos>) -> Option<RtpsPublisher<'a>> {
        todo!()
    }

    pub fn delete_publisher(&self, _a_publisher: &RtpsPublisher) -> ReturnCode<()> {
        todo!()
    }

    pub fn create_topic<T: DDSType>(&self, _topic_name: String, _qos: Option<&TopicQos>,) -> Option<RtpsTopic<T>> {
        todo!()
    }

    pub fn delete_topic<T: DDSType>(&self, _a_topic: &RtpsTopic<T>) -> ReturnCode<()> {
        todo!()
    }

    pub fn create_subscriber(&self, _qos: Option<&SubscriberQos>) -> Option<RtpsSubscriber> {
        todo!()
    }

    pub fn delete_subscriber(&self, _a_subscriber: &RtpsSubscriber) -> ReturnCode<()> {
        todo!()
    }

    pub fn find_topic<T: DDSType>(
        &self,
        _topic_name: String,
        _timeout: Duration,
    ) -> Option<RtpsTopic<T>> {
        todo!()
    }

    pub fn lookup_topicdescription<T: DDSType>(&self, _name: &str) -> Option<RtpsTopic<T>> {
        todo!()
    }

    pub fn get_builtin_subscriber(&self) -> RtpsSubscriber {
        todo!()
    }

    pub fn ignore_participant(&self, _handle: InstanceHandle) -> ReturnCode<()> {
        todo!()
    }

    pub fn ignore_topic(&self, _handle: InstanceHandle) -> ReturnCode<()> {
        todo!()
    }

    pub fn ignore_publication(&self, _handle: InstanceHandle) -> ReturnCode<()> {
        todo!()
    }

    pub fn ignore_subscription(&self, _handle: InstanceHandle) -> ReturnCode<()> {
        todo!()
    }

    pub fn get_domain_id(&self) -> DomainId {
        todo!()
    }

    pub fn delete_contained_entities(&self) -> ReturnCode<()> {
        todo!()
    }

    pub fn assert_liveliness(&self) -> ReturnCode<()> {
        todo!()
    }

    pub fn set_default_publisher_qos(&self, _qos: Option<PublisherQos>) -> ReturnCode<()> {
        todo!()
    }

    pub fn get_default_publisher_qos(&self) -> PublisherQos {
        todo!()
    }

    pub fn set_default_subscriber_qos(&self, _qos: Option<SubscriberQos>) -> ReturnCode<()> {
        todo!()
    }

    pub fn get_default_subscriber_qos(&self) -> SubscriberQos {
        todo!()
    }

    pub fn set_default_topic_qos(&self, _qos: Option<TopicQos>) -> ReturnCode<()> {
        todo!()
    }

    pub fn get_default_topic_qos(&self) -> TopicQos {
        todo!()
    }

    pub fn get_discovered_participants(
        &self,
        _participant_handles: &mut [InstanceHandle],
    ) -> ReturnCode<()> {
        todo!()
    }

    pub fn get_discovered_participant_data(
        &self,
        _participant_data: ParticipantBuiltinTopicData,
        _participant_handle: InstanceHandle,
    ) -> ReturnCode<()> {
        todo!()
    }

    pub fn get_discovered_topics(
        &self,
        _topic_handles: &mut [InstanceHandle]
    ) -> ReturnCode<()> {
        todo!()
    }

    pub fn get_discovered_topic_data(
        &self,
        _topic_data: TopicBuiltinTopicData,
        _topic_handle: InstanceHandle
    ) -> ReturnCode<()> {
        todo!()
    }

    pub fn contains_entity(
        &self,
        _a_handle: InstanceHandle
    ) -> bool {
        todo!()
    }

    pub fn get_current_time(&self) -> ReturnCode<Time> {
        todo!()
    }
}

impl Entity for RtpsParticipant {
    type Qos = DomainParticipantQos;
    type Listener = Box<dyn DomainParticipantListener>;

    fn set_qos(&self, _qos: Self::Qos) -> ReturnCode<()> {
        todo!()
    }

    fn get_qos(&self) -> ReturnCode<Self::Qos> {
        todo!()
    }

    fn set_listener(&self, _a_listener: Self::Listener, _mask: StatusMask) -> ReturnCode<()> {
        todo!()
    }

    fn get_listener(&self) -> &Self::Listener {
        todo!()
    }

    fn get_statuscondition(&self) -> StatusCondition {
        todo!()
    }

    fn get_status_changes(&self) -> StatusMask {
        todo!()
    }

    fn enable(&self) -> ReturnCode<()> {
        todo!()
    }

    fn get_instance_handle(&self) -> ReturnCode<InstanceHandle> {
        todo!()
    }
}