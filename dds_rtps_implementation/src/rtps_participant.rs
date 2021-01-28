use std::{
    cell::RefCell,
    marker::PhantomData,
    sync::{atomic, Arc, Mutex, Once},
    thread::JoinHandle,
};

use rust_dds_api::{
    builtin_topics::{ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    domain::{
        domain_participant::{DomainParticipant, DomainParticipantChildNode},
        domain_participant_listener::DomainParticipantListener,
    },
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::{DataWriterQos, DomainParticipantQos, PublisherQos, SubscriberQos, TopicQos},
        status::StatusMask,
    },
    publication::publisher_listener::PublisherListener,
    subscription::subscriber_listener::SubscriberListener,
    topic::{topic::Topic, topic_description::TopicDescription, topic_listener::TopicListener},
};
use rust_rtps::{
    structure::Participant,
    transport::Transport,
    types::{
        constants::{
            ENTITY_KIND_BUILT_IN_READER_GROUP, ENTITY_KIND_BUILT_IN_WRITER_GROUP,
            PROTOCOL_VERSION_2_4, VENDOR_ID,
        },
        Locator,
    },
};

use rust_dds_types::{DDSType, DomainId, Duration, InstanceHandle, ReturnCode, Time};

use crate::{
    rtps_publisher::{RtpsPublisherNode, RtpsPublisherRef},
    rtps_subscriber::{RtpsSubscriberNode, RtpsSubscriberRef},
    rtps_topic::RtpsTopicNode,
};

use super::rtps_participant_entities::RtpsParticipantEntities;

pub struct RtpsParticipant {
    participant: Participant,
    qos: Mutex<DomainParticipantQos>,
    default_publisher_qos: Mutex<PublisherQos>,
    default_subscriber_qos: Mutex<SubscriberQos>,
    default_topic_qos: Mutex<TopicQos>,
    builtin_entities: Arc<RtpsParticipantEntities>,
    user_defined_entities: Arc<RtpsParticipantEntities>,
    enabled: Arc<atomic::AtomicBool>,
    enabled_function: Once,
    thread_list: RefCell<Vec<JoinHandle<()>>>,
    a_listener: Option<Box<dyn DomainParticipantListener>>,
    mask: StatusMask,
}

// impl Into<ParticipantProxy> for &RtpsParticipant {
//     fn into(self) -> ParticipantProxy {
//         ParticipantProxy {
//             domain_id: self.participant.domain_id,
//             domain_tag: "".to_string(),
//             protocol_version: self.participant.protocol_version,
//             guid_prefix: self.participant.entity.guid.prefix(),
//             vendor_id: self.participant.vendor_id,
//             expects_inline_qos: true,
//             available_built_in_endpoints: BuiltInEndpointSet { value: 9 },
//             // built_in_endpoint_qos:
//             metatraffic_unicast_locator_list: self
//                 .builtin_entities
//                 .transport
//                 .unicast_locator_list()
//                 .clone(),
//             metatraffic_multicast_locator_list: self
//                 .builtin_entities
//                 .transport
//                 .multicast_locator_list()
//                 .clone(),
//             default_unicast_locator_list: vec![],
//             default_multicast_locator_list: vec![],
//             manual_liveliness_count: 8,
//         }
//     }
// }

impl RtpsParticipant {
    pub fn new(
        domain_id: DomainId,
        qos: DomainParticipantQos,
        userdata_transport: impl Transport,
        metatraffic_transport: impl Transport,
        a_listener: Option<Box<dyn DomainParticipantListener>>,
        mask: StatusMask,
    ) -> Self {
        let guid_prefix = [1; 12];
        let participant = Participant::new(guid_prefix, domain_id, PROTOCOL_VERSION_2_4, VENDOR_ID);

        let builtin_entities = Arc::new(RtpsParticipantEntities::new_builtin(
            guid_prefix,
            metatraffic_transport,
        ));
        let user_defined_entities = Arc::new(RtpsParticipantEntities::new_user_defined(
            guid_prefix,
            userdata_transport,
        ));

        RtpsParticipant {
            participant,
            qos: Mutex::new(qos),
            default_publisher_qos: Mutex::new(PublisherQos::default()),
            default_subscriber_qos: Mutex::new(SubscriberQos::default()),
            default_topic_qos: Mutex::new(TopicQos::default()),
            builtin_entities,
            user_defined_entities,
            enabled: Arc::new(atomic::AtomicBool::new(false)),
            enabled_function: Once::new(),
            thread_list: RefCell::new(Vec::new()),
            a_listener,
            mask,
        }
    }

    // pub fn create_publisher(
    //     &self,
    //     qos: Option<PublisherQos>,
    //     // listener: Option<impl PublisherListener>,
    //     // status_mask: StatusMask,
    // ) -> Option<RtpsPublisherRef> {
    //     let qos = qos.unwrap_or(self.get_default_publisher_qos());
    //     let publisher_ref = self.user_defined_entities.create_publisher(qos)?;
    //     Some(MaybeValidNode::new(self, publisher_ref))
    // }

    // pub fn delete_publisher(&self, a_publisher: &RtpsPublisherRef) -> ReturnCode<()> {
    //     self.user_defined_entities.delete_publisher(a_publisher)
    // }

    // pub fn create_subscriber(
    //     &self,
    //     qos: Option<SubscriberQos>,
    //     // _a_listener: impl SubscriberListener,
    //     // _mask: StatusMask
    // ) -> Option<RtpsSubscriberRef> {
    //     let qos = qos.unwrap_or(self.get_default_subscriber_qos());
    //     self.user_defined_entities.create_subscriber(qos)
    // }

    // pub fn delete_subscriber(&self, a_subscriber: &RtpsSubscriberRef) -> ReturnCode<()> {
    //     self.user_defined_entities.delete_subscriber(a_subscriber)
    // }

    // pub fn create_topic<T: DDSType>(
    //     &self,
    //     topic_name: &str,
    //     qos: Option<TopicQos>,
    //     // _a_listener: impl TopicListener<T>,
    //     // _mask: StatusMask
    // ) -> Option<RtpsAnyTopicRef> {
    //     let qos = qos.unwrap_or(self.get_default_topic_qos());
    //     qos.is_consistent().ok()?;
    //     self.user_defined_entities
    //         .create_topic::<T>(topic_name, qos)
    // }

    // pub fn delete_topic<T: DDSType>(&self, a_topic: &RtpsAnyTopicRef) -> ReturnCode<()> {
    //     self.user_defined_entities.delete_topic::<T>(a_topic)
    // }

    // pub fn set_default_publisher_qos(&self, qos: Option<PublisherQos>) -> ReturnCode<()> {
    //     let qos = qos.unwrap_or_default();
    //     *self.default_publisher_qos.lock().unwrap() = qos;
    //     Ok(())
    // }

    // pub fn get_default_publisher_qos(&self) -> PublisherQos {
    //     self.default_publisher_qos.lock().unwrap().clone()
    // }

    // pub fn set_default_subscriber_qos(&self, qos: Option<SubscriberQos>) -> ReturnCode<()> {
    //     let qos = qos.unwrap_or_default();
    //     *self.default_subscriber_qos.lock().unwrap() = qos;
    //     Ok(())
    // }

    // pub fn get_default_subscriber_qos(&self) -> SubscriberQos {
    //     self.default_subscriber_qos.lock().unwrap().clone()
    // }

    // pub fn set_default_topic_qos(&self, qos: Option<TopicQos>) -> ReturnCode<()> {
    //     let qos = qos.unwrap_or_default();
    //     qos.is_consistent()?;
    //     *self.default_topic_qos.lock().unwrap() = qos;
    //     Ok(())
    // }

    // pub fn get_default_topic_qos(&self) -> TopicQos {
    //     self.default_topic_qos.lock().unwrap().clone()
    // }

    // pub fn set_qos(&self, qos: Option<DomainParticipantQos>) -> ReturnCode<()> {
    //     let qos = qos.unwrap_or_default();
    //     *self.qos.lock().unwrap() = qos;
    //     Ok(())
    // }

    // pub fn get_qos(&self) -> ReturnCode<DomainParticipantQos> {
    //     Ok(self.qos.lock().unwrap().clone())
    // }

    // pub fn get_domain_id(&self) -> DomainId {
    //     self.participant.domain_id
    // }

    // pub fn enable(&self) -> ReturnCode<()> {
    //     self.enabled_function.call_once(|| {
    //         // let builtin_publisher_ref = self
    //         //     .builtin_entities
    //         //     .create_publisher(PublisherQos::default())
    //         //     .expect("Error creating built-in publisher");
    //         // let builtin_publisher = builtin_publisher_ref.get().expect("Error retrieving built-in publisher");

    //         // let spdp_topic_qos = TopicQos::default();
    //         // let spdp_topic = self.builtin_entities
    //         //     .create_topic::<SpdpDiscoveredParticipantData>("SPDP", spdp_topic_qos)
    //         //     .expect("Error creating SPDP topic");

    //         // let mut spdp_announcer_qos = DataWriterQos::default();
    //         // spdp_announcer_qos.reliability.kind = crate::dds::infrastructure::qos_policy::ReliabilityQosPolicyKind::BestEffortReliabilityQos;
    //         // let _spdp_announcer_anywriter_ref = builtin_publisher
    //         //     .create_stateless_builtin_datawriter::<SpdpDiscoveredParticipantData>(
    //         //         &spdp_topic,
    //         //         Some(spdp_announcer_qos),
    //         //     )
    //         //     .expect("Error creating SPDP built-in writer");

    //         // let _spdp_locator = Locator::new_udpv4(7400, [239, 255, 0, 0]);
    //         // let spdp_announcer = spdp_announcer_anywriter_ref.get().expect("Error retrieving SPDP announcer");
    //         // spdp_announcer
    //         //     .writer()
    //         //     .try_get_stateless()
    //         //     .unwrap()
    //         //     .reader_locator_add(spdp_locator);

    //         // let key = BuiltInTopicKey([1, 2, 3]);
    //         // let user_data = UserDataQosPolicy { value: vec![] };
    //         // let dds_participant_data = ParticipantBuiltinTopicData { key, user_data };
    //         // let participant_proxy = self.into();
    //         // let lease_duration = DURATION_INFINITE;

    //         // let data = SpdpDiscoveredParticipantData {
    //         //     dds_participant_data,
    //         //     participant_proxy,
    //         //     lease_duration,
    //         // };

    //         // spdp_announcer_anywriter_ref
    //         //     .get_as::<SpdpDiscoveredParticipantData>()
    //         //     .unwrap()
    //         //     .write_w_timestamp(data, None, TIME_INVALID)
    //         //     .ok();

    //         let mut thread_list = self.thread_list.borrow_mut();
    //         let enabled = self.enabled.clone();
    //         let builtin_entities = self.builtin_entities.clone();
    //         self.enabled.store(true, atomic::Ordering::Release);
    //         thread_list.push(std::thread::spawn(move || {
    //             while enabled.load(atomic::Ordering::Acquire) {
    //                 builtin_entities.send_data();
    //                 std::thread::sleep(std::time::Duration::from_secs(1));
    //             }
    //         }));
    //     });

    //     Ok(())
    // }

    // pub fn get_builtin_publisher(&self) -> Option<RtpsPublisherRef> {
    //     // let publisher_ref = self.builtin_entities
    //     //     .publisher_list()
    //     //     .into_iter()
    //     //     .find(|x| {
    //     //         if let Some(publisher) = x.get().ok() {
    //     //             publisher.group.entity.guid.entity_id().entity_kind()
    //     //                 == ENTITY_KIND_BUILT_IN_WRITER_GROUP
    //     //         } else {
    //     //             false
    //     //         }
    //     //     })?;
    //     todo!()
    // }

    // pub fn get_builtin_subscriber(&self) -> Option<RtpsSubscriberRef> {
    //     self.builtin_entities
    //         .subscriber_list()
    //         .into_iter()
    //         .find(|x| {
    //             if let Some(subscriber) = x.get().ok() {
    //                 subscriber.group.entity.guid.entity_id().entity_kind()
    //                     == ENTITY_KIND_BUILT_IN_READER_GROUP
    //             } else {
    //                 false
    //             }
    //         })
    // }
}

impl<'a> DomainParticipant<'a> for RtpsParticipant {
    type PublisherType = RtpsPublisherNode<'a>;
    type SubscriberType = RtpsSubscriberNode<'a>;

    fn create_publisher(
        &'a self,
        qos: Option<PublisherQos>,
        a_listener: Option<Box<dyn PublisherListener>>,
        mask: StatusMask,
    ) -> Option<Self::PublisherType> {
        let qos = qos.unwrap_or(self.get_default_publisher_qos());
        let publisher_ref = self
            .builtin_entities
            .create_publisher(qos, a_listener, mask)?;
        Some(RtpsPublisherNode::new(self, publisher_ref))
    }

    fn delete_publisher(&self, _a_publisher: &Self::PublisherType) -> ReturnCode<()> {
        todo!()
    }

    fn create_subscriber(
        &'a self,
        qos: Option<SubscriberQos>,
        a_listener: Option<Box<dyn SubscriberListener>>,
        mask: StatusMask,
    ) -> Option<Self::SubscriberType> {
        let qos = qos.unwrap_or(self.get_default_subscriber_qos());
        let subscriber_ref = self
            .builtin_entities
            .create_subscriber(qos, a_listener, mask)?;
        Some(RtpsSubscriberNode::new(self, subscriber_ref))
    }

    fn delete_subscriber(&self, _a_subscriber: &Self::SubscriberType) -> ReturnCode<()> {
        todo!()
    }

    fn create_topic<T: DDSType>(
        &'a self,
        topic_name: &str,
        qos: Option<TopicQos>,
        a_listener: Option<Box<dyn TopicListener<T>>>,
        mask: StatusMask,
    ) -> Option<Box<dyn Topic<'a, T> + 'a>> {
        todo!()
        // let qos = qos.unwrap_or(self.get_default_topic_qos());
        // qos.is_consistent().ok()?;
        // let topic_ref = self
        //     .builtin_entities
        //     .create_topic(topic_name, qos, a_listener, mask)?;
        // Some(Arc::new(RtpsTopicNode::new(self, topic_ref)))
    }

    fn delete_topic<T: DDSType>(&'a self, _a_topic: &'a Box<dyn Topic<T> + 'a>) -> ReturnCode<()> {
        todo!()
    }

    fn find_topic<T: DDSType>(
        &self,
        _topic_name: &str,
        _timeout: Duration,
    ) -> Option<Box<dyn Topic<T>>> {
        todo!()
    }

    fn lookup_topicdescription<T: DDSType>(
        &self,
        _name: &str,
    ) -> Option<Box<dyn TopicDescription<T>>> {
        todo!()
    }

    fn get_builtin_subscriber(&self) -> Self::SubscriberType {
        todo!()
    }

    fn ignore_participant(&self, _handle: InstanceHandle) -> ReturnCode<()> {
        todo!()
    }

    fn ignore_topic(&self, _handle: InstanceHandle) -> ReturnCode<()> {
        todo!()
    }

    fn ignore_publication(&self, _handle: InstanceHandle) -> ReturnCode<()> {
        todo!()
    }

    fn ignore_subscription(&self, _handle: InstanceHandle) -> ReturnCode<()> {
        todo!()
    }

    fn get_domain_id(&self) -> DomainId {
        todo!()
    }

    fn delete_contained_entities(&self) -> ReturnCode<()> {
        todo!()
    }

    fn assert_liveliness(&self) -> ReturnCode<()> {
        todo!()
    }

    fn set_default_publisher_qos(&self, qos: Option<PublisherQos>) -> ReturnCode<()> {
        let qos = qos.unwrap_or_default();
        *self.default_publisher_qos.lock().unwrap() = qos;
        Ok(())
    }

    fn get_default_publisher_qos(&self) -> PublisherQos {
        self.default_publisher_qos.lock().unwrap().clone()
    }

    fn set_default_subscriber_qos(&self, _qos: Option<SubscriberQos>) -> ReturnCode<()> {
        todo!()
    }

    fn get_default_subscriber_qos(&self) -> SubscriberQos {
        todo!()
    }

    fn set_default_topic_qos(&self, _qos: Option<TopicQos>) -> ReturnCode<()> {
        todo!()
    }

    fn get_default_topic_qos(&self) -> TopicQos {
        todo!()
    }

    fn get_discovered_participants(
        &self,
        _participant_handles: &mut [InstanceHandle],
    ) -> ReturnCode<()> {
        todo!()
    }

    fn get_discovered_participant_data(
        &self,
        _participant_data: ParticipantBuiltinTopicData,
        _participant_handle: InstanceHandle,
    ) -> ReturnCode<()> {
        todo!()
    }

    fn get_discovered_topics(&self, _topic_handles: &mut [InstanceHandle]) -> ReturnCode<()> {
        todo!()
    }

    fn get_discovered_topic_data(
        &self,
        _topic_data: TopicBuiltinTopicData,
        _topic_handle: InstanceHandle,
    ) -> ReturnCode<()> {
        todo!()
    }

    fn contains_entity(&self, _a_handle: InstanceHandle) -> bool {
        todo!()
    }

    fn get_current_time(&self) -> ReturnCode<Time> {
        todo!()
    }
}

impl Entity for RtpsParticipant {
    type Qos = DomainParticipantQos;
    type Listener = Box<dyn DomainParticipantListener>;

    fn set_qos(&self, _qos: Option<Self::Qos>) -> ReturnCode<()> {
        todo!()
    }

    fn get_qos(&self) -> ReturnCode<Self::Qos> {
        todo!()
    }

    fn set_listener(&self, _a_listener: Self::Listener, _maskk: StatusMask) -> ReturnCode<()> {
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

impl<'a> Drop for RtpsParticipant {
    fn drop(&mut self) {
        self.enabled.store(false, atomic::Ordering::Release);
        for thread in self.thread_list.borrow_mut().drain(..) {
            thread.join().ok();
        }
    }
}
