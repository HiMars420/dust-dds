use std::{
    cell::RefCell,
    sync::{atomic, Arc},
};

use crate::{
    dds::{
        implementation::{rtps_datawriter::RtpsDataWriter, rtps_participant::RtpsParticipant},
        infrastructure::{
            qos::{DataWriterQos, DomainParticipantQos},
            qos_policy::ReliabilityQosPolicyKind,
        },
    },
    discovery::types::SpdpDiscoveredParticipantData,
    rtps::{transport::udp::UdpTransport, types::Locator},
    types::DomainId,
};

use super::domain_participant::DomainParticipant;

/// The DomainParticipant object plays several roles:
/// - It acts as a container for all other Entity objects.
/// - It acts as factory for the Publisher, Subscriber, Topic, and MultiTopic Entity objects.
/// - It represents the participation of the application on a communication plane that isolates applications running on the
/// same set of physical computers from each other. A domain establishes a “virtual network” linking all applications that
/// share the same domainId and isolating them from applications running on different domains. In this way, several
/// independent distributed applications can coexist in the same physical network without interfering, or even being aware
/// of each other.
/// - It provides administration services in the domain, offering operations that allow the application to ‘ignore’ locally any
/// information about a given participant (ignore_participant), publication (ignore_publication), subscription
/// (ignore_subscription), or topic (ignore_topic).
///
/// The following sub clauses explain all the operations in detail.
/// The following operations may be called even if the DomainParticipant is not enabled. Other operations will have the value
/// NOT_ENABLED if called on a disabled DomainParticipant:
/// - Operations defined at the base-class level namely, set_qos, get_qos, set_listener, get_listener, and enable.
/// - Factory methods: create_topic, create_publisher, create_subscriber, delete_topic, delete_publisher,
/// delete_subscriber
/// - Operations that access the status: get_statuscondition
pub struct DomainParticipantFactory;

impl DomainParticipantFactory {
    /// This operation creates a new DomainParticipant object. The DomainParticipant signifies that the calling application intends
    /// to join the Domain identified by the domain_id argument.
    /// If the specified QoS policies are not consistent, the operation will fail and no DomainParticipant will be created.
    /// The special value PARTICIPANT_QOS_DEFAULT can be used to indicate that the DomainParticipant should be created
    /// with the default DomainParticipant QoS set in the factory. The use of this value is equivalent to the application obtaining the
    /// default DomainParticipant QoS by means of the operation get_default_participant_qos (2.2.2.2.2.6) and using the resulting
    /// QoS to create the DomainParticipant.
    /// In case of failure, the operation will return a ‘nil’ value (as specified by the platform).
    pub fn create_participant(
        domain_id: DomainId,
        qos: Option<DomainParticipantQos>,
        // a_listener: impl DomainParticipantListener,
        // mask: StatusMask,
        //     enabled: bool,
    ) -> Option<DomainParticipant> {
        let interface = "Wi-Fi";
        let userdata_transport =
            UdpTransport::default_userdata_transport(domain_id, interface).unwrap();
        let metatraffic_transport =
            UdpTransport::default_metatraffic_transport(domain_id, interface).unwrap();
        let qos = qos.unwrap_or_default();

        let rtps_participant =
            RtpsParticipant::new(domain_id, qos.clone(), userdata_transport, metatraffic_transport);

        // {
        //     let builtin_publisher = rtps_builtin_participant
        //         .create_builtin_publisher(None)
        //         .unwrap();
        //     let topic = rtps_builtin_participant
        //         .create_topic::<SpdpDiscoveredParticipantData>("BuildinTopic", None)
        //         .unwrap();
        //     let mut builtin_writer_qos = DataWriterQos::default();
        //     builtin_writer_qos.reliability.kind =
        //         ReliabilityQosPolicyKind::BestEffortReliabilityQos;
        //     let any_writer = builtin_publisher
        //         .get()
        //         .unwrap()
        //         .create_stateless_builtin_datawriter::<SpdpDiscoveredParticipantData>(
        //             topic.get().unwrap().clone(),
        //             Some(builtin_writer_qos),
        //         )
        //         .unwrap();

        //     let rtps_writer = any_writer
        //         .value_as::<RtpsDataWriter<SpdpDiscoveredParticipantData>>()
        //         .unwrap();

        //     let mut writer_flavor = rtps_writer.writer.lock().unwrap();
        //     if let Some(stateless_writer) = writer_flavor.try_get_stateless() {
        //         stateless_writer.reader_locator_add(Locator::new_udpv4(7400, [239, 255, 0, 0]));
        //     }
        // }
        // let builtin_topic = rtps_builtin_participant
        //     .create_topic::<SpdpDiscoveredParticipantData>("BuildinTopic", None)
        //     .unwrap();
        // let builtin_writer = builtin_publisher.value().unwrap().create_datawriter(a_topic, qos)

        // if enabled {
        //     new_participant.enable().ok()?;
        // }

        Some(DomainParticipant(rtps_participant))
    }

    pub fn delete_participant(_a_participant: DomainParticipant) {}
}
