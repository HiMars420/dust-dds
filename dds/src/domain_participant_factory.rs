use std::{
    net::{Ipv4Addr, UdpSocket},
    str::FromStr,
};

use rust_dds_api::{
    dcps_psm::{DomainId, StatusMask},
    domain::domain_participant_listener::DomainParticipantListener,
    infrastructure::qos::{DataReaderQos, DataWriterQos, DomainParticipantQos},
};
use rust_dds_rtps_implementation::{
    data_representation_builtin_endpoints::{
        sedp_discovered_reader_data::SedpDiscoveredReaderData,
        sedp_discovered_topic_data::SedpDiscoveredTopicData,
        sedp_discovered_writer_data::SedpDiscoveredWriterData,
        spdp_discovered_participant_data::SpdpDiscoveredParticipantData,
    },
    dds_impl::{
        data_reader_impl::DataReaderImpl, data_writer_impl::DataWriterImpl,
        domain_participant_impl::DomainParticipantImpl,
    },
    rtps_impl::{
        rtps_stateful_writer_impl::RtpsStatefulWriterImpl,
        rtps_stateless_writer_impl::RtpsStatelessWriterImpl,
    },
    utils::clock::StdTimer,
};
use rust_rtps_pim::{
    behavior::writer::{
        reader_locator::RtpsReaderLocator, stateless_writer::RtpsStatelessWriterOperations,
    },
    discovery::{
        sedp::builtin_endpoints::{
            SedpBuiltinPublicationsReader, SedpBuiltinPublicationsWriter,
            SedpBuiltinSubscriptionsReader, SedpBuiltinSubscriptionsWriter,
            SedpBuiltinTopicsReader, SedpBuiltinTopicsWriter,
        },
        spdp::builtin_endpoints::{SpdpBuiltinParticipantReader, SpdpBuiltinParticipantWriter},
    },
    structure::types::{GuidPrefix, LOCATOR_KIND_UDPv4, Locator},
};

use crate::udp_transport::UdpTransport;

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
    ///
    /// Developer note: Ideally this method should return impl DomainParticipant. However because of the GAT workaround used there is no way
    /// to call,e.g. create_topic(), because we can't write impl DomainParticipant + for<'t, T> TopicGAT<'t, T> on the return. This issue will
    /// probably be solved once the GAT functionality is available on stable.
    pub fn create_participant(
        domain_id: DomainId,
        qos: Option<DomainParticipantQos>,
        _a_listener: Option<Box<dyn DomainParticipantListener>>,
        _mask: StatusMask,
    ) -> Option<DomainParticipantImpl> {
        let domain_participant_qos = qos.unwrap_or_default();

        // /////// Define guid prefix
        let guid_prefix = GuidPrefix([3; 12]);

        // /////// Define other configurations
        let domain_tag = "".to_string();
        let metatraffic_unicast_locator_list = vec![Locator::new(
            LOCATOR_KIND_UDPv4,
            7400,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 127, 0, 0, 1],
        )];
        let metatraffic_multicast_locator_list = vec![Locator::new(
            LOCATOR_KIND_UDPv4,
            7400,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239, 255, 0, 1],
        )];
        let default_unicast_locator_list = vec![Locator::new(
            LOCATOR_KIND_UDPv4,
            7410,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 127, 0, 0, 1],
        )];
        let default_multicast_locator_list = vec![];

        // /////// Create transports
        let socket = UdpSocket::bind("127.0.0.1:7400").unwrap();
        socket.set_nonblocking(true).unwrap();
        socket
            .join_multicast_v4(
                &Ipv4Addr::from_str("239.255.0.1").unwrap(),
                &Ipv4Addr::from_str("127.0.0.1").unwrap(),
            )
            .unwrap();
        socket.set_multicast_loop_v4(true).unwrap();
        let metatraffic_transport = UdpTransport::new(socket);

        let socket = UdpSocket::bind("127.0.0.1:7410").unwrap();
        socket.set_nonblocking(true).unwrap();
        let default_transport = UdpTransport::new(socket);

        // /////// Create SPDP and SEDP endpoints
        let spdp_builtin_participant_rtps_reader =
            SpdpBuiltinParticipantReader::create(guid_prefix, vec![], vec![]);
        let mut spdp_builtin_participant_rtps_writer = RtpsStatelessWriterImpl::new(
            SpdpBuiltinParticipantWriter::create(guid_prefix, vec![], vec![]),
        );
        let sedp_builtin_publications_rtps_reader =
            SedpBuiltinPublicationsReader::create(guid_prefix, vec![], vec![]);
        let sedp_builtin_publications_rtps_writer = RtpsStatefulWriterImpl::new(
            SedpBuiltinPublicationsWriter::create(guid_prefix, vec![], vec![]),
        );
        let sedp_builtin_subscriptions_rtps_reader =
            SedpBuiltinSubscriptionsReader::create(guid_prefix, vec![], vec![]);
        let sedp_builtin_subscriptions_rtps_writer = RtpsStatefulWriterImpl::new(
            SedpBuiltinSubscriptionsWriter::create(guid_prefix, vec![], vec![]),
        );
        let sedp_builtin_topics_rtps_reader =
            SedpBuiltinTopicsReader::create(guid_prefix, vec![], vec![]);
        let sedp_builtin_topics_rtps_writer = RtpsStatefulWriterImpl::new(
            SedpBuiltinTopicsWriter::create(guid_prefix, vec![], vec![]),
        );

        // ////////// Configure SPDP reader locator
        let spdp_discovery_locator = RtpsReaderLocator::new(
            Locator::new(
                LOCATOR_KIND_UDPv4,
                7400,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239, 255, 0, 1],
            ),
            false,
        );

        spdp_builtin_participant_rtps_writer.reader_locator_add(spdp_discovery_locator);

        // ///////// Create built-in DDS data readers and data writers
        let spdp_builtin_participant_dds_data_reader =
            DataReaderImpl::<SpdpDiscoveredParticipantData>::new(
                DataReaderQos::default(),
                spdp_builtin_participant_rtps_reader,
            );

        let spdp_builtin_participant_dds_data_writer =
            DataWriterImpl::<SpdpDiscoveredParticipantData, _, _>::new(
                DataWriterQos::default(),
                spdp_builtin_participant_rtps_writer,
                StdTimer::new(),
            );

        let sedp_builtin_publications_dds_data_reader =
            DataReaderImpl::<SedpDiscoveredWriterData>::new(
                DataReaderQos::default(),
                sedp_builtin_publications_rtps_reader,
            );

        let sedp_builtin_publications_dds_data_writer =
            DataWriterImpl::<SedpDiscoveredWriterData, _, _>::new(
                DataWriterQos::default(),
                sedp_builtin_publications_rtps_writer,
                StdTimer::new(),
            );

        let sedp_builtin_subscriptions_dds_data_reader =
            DataReaderImpl::<SedpDiscoveredReaderData>::new(
                DataReaderQos::default(),
                sedp_builtin_subscriptions_rtps_reader,
            );

        let sedp_builtin_subscriptions_dds_data_writer =
            DataWriterImpl::<SedpDiscoveredReaderData, _, _>::new(
                DataWriterQos::default(),
                sedp_builtin_subscriptions_rtps_writer,
                StdTimer::new(),
            );

        let sedp_builtin_topics_dds_data_reader = DataReaderImpl::<SedpDiscoveredTopicData>::new(
            DataReaderQos::default(),
            sedp_builtin_topics_rtps_reader,
        );

        let sedp_builtin_topics_dds_data_writer =
            DataWriterImpl::<SedpDiscoveredTopicData, _, _>::new(
                DataWriterQos::default(),
                sedp_builtin_topics_rtps_writer,
                StdTimer::new(),
            );

        let domain_participant = DomainParticipantImpl::new(
            guid_prefix,
            domain_id,
            domain_tag,
            domain_participant_qos,
            metatraffic_transport,
            default_transport,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            default_unicast_locator_list,
            default_multicast_locator_list,
            Some(spdp_builtin_participant_dds_data_reader),
            Some(spdp_builtin_participant_dds_data_writer),
            Some(sedp_builtin_publications_dds_data_reader),
            Some(sedp_builtin_publications_dds_data_writer),
            Some(sedp_builtin_subscriptions_dds_data_reader),
            Some(sedp_builtin_subscriptions_dds_data_writer),
            Some(sedp_builtin_topics_dds_data_reader),
            Some(sedp_builtin_topics_dds_data_writer),
        );

        Some(domain_participant)
    }
}
