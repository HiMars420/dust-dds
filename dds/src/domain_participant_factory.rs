use async_std::stream::StreamExt;
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    str::FromStr,
    sync::{
        atomic::{self, AtomicBool},
        mpsc::{Receiver, SyncSender},
        Arc,
    },
};

use rust_dds_api::{
    dcps_psm::{DomainId, StatusMask, Time},
    domain::domain_participant_listener::DomainParticipantListener,
    infrastructure::qos::{
        DataReaderQos, DataWriterQos, DomainParticipantQos, PublisherQos, SubscriberQos,
    },
    publication::data_writer::DataWriter,
    subscription::data_reader::DataReaderBorrowedSamples,
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
        domain_participant_proxy::DomainParticipantProxy, publisher_impl::PublisherImpl,
        subscriber_impl::SubscriberImpl,
    },
    rtps_impl::{
        rtps_stateful_reader_impl::RtpsStatefulReaderImpl,
        rtps_stateful_writer_impl::RtpsStatefulWriterImpl,
        rtps_stateless_reader_impl::RtpsStatelessReaderImpl,
        rtps_stateless_writer_impl::RtpsStatelessWriterImpl, rtps_group_impl::RtpsGroupImpl,
    },
    utils::{
        clock::StdTimer,
        shared_object::{rtps_shared_new, rtps_shared_write_lock, RtpsShared},
    },
};
use rust_rtps_pim::{
    behavior::{
        reader::stateful_reader::RtpsStatefulReaderOperations,
        writer::{
            reader_locator::RtpsReaderLocator, stateful_writer::RtpsStatefulWriterOperations,
            stateless_writer::RtpsStatelessWriterOperations,
        },
    },
    discovery::{
        participant_discovery::ParticipantDiscovery,
        sedp::builtin_endpoints::{
            SedpBuiltinPublicationsReader, SedpBuiltinPublicationsWriter,
            SedpBuiltinSubscriptionsReader, SedpBuiltinSubscriptionsWriter,
            SedpBuiltinTopicsReader, SedpBuiltinTopicsWriter,
        },
        spdp::builtin_endpoints::{SpdpBuiltinParticipantReader, SpdpBuiltinParticipantWriter},
    },
    structure::types::{
        EntityId, Guid, GuidPrefix, LOCATOR_KIND_UDPv4, Locator, BUILT_IN_READER_GROUP,
        BUILT_IN_WRITER_GROUP, PROTOCOLVERSION, VENDOR_ID_S2E,
    },
};

use crate::{communication::Communication, udp_transport::UdpTransport};

pub struct Executor {
    receiver: Receiver<EnabledPeriodicTask>,
}

impl Executor {
    pub fn run(&self) {
        while let Ok(mut enabled_periodic_task) = self.receiver.try_recv() {
            async_std::task::spawn(async move {
                let mut interval = async_std::stream::interval(enabled_periodic_task.period);
                loop {
                    if enabled_periodic_task.enabled.load(atomic::Ordering::SeqCst) {
                        (enabled_periodic_task.task)();
                    } else {
                        println!("Not enabled");
                    }
                    interval.next().await;
                }
            });
        }
    }
}

#[derive(Clone)]
pub struct Spawner {
    task_sender: SyncSender<EnabledPeriodicTask>,
    enabled: Arc<AtomicBool>,
}

impl Spawner {
    pub fn new(task_sender: SyncSender<EnabledPeriodicTask>, enabled: Arc<AtomicBool>) -> Self {
        Self {
            task_sender,
            enabled,
        }
    }

    pub fn spawn_enabled_periodic_task(
        &self,
        name: &'static str,
        task: impl FnMut() -> () + Send + Sync + 'static,
        period: std::time::Duration,
    ) {
        self.task_sender
            .send(EnabledPeriodicTask {
                name,
                task: Box::new(task),
                period,
                enabled: self.enabled.clone(),
            })
            .unwrap();
    }

    pub fn enable_tasks(&self) {
        self.enabled.store(true, atomic::Ordering::SeqCst);
    }

    pub fn disable_tasks(&self) {
        self.enabled.store(false, atomic::Ordering::SeqCst);
    }
}

pub struct EnabledPeriodicTask {
    pub name: &'static str,
    pub task: Box<dyn FnMut() -> () + Send + Sync>,
    pub period: std::time::Duration,
    pub enabled: Arc<AtomicBool>,
}

fn task_discovery(
    spdp_builtin_participant_data_reader_arc: &RtpsShared<
        impl for<'a> DataReaderBorrowedSamples<'a, Samples = Vec<&'a SpdpDiscoveredParticipantData>>,
    >,
    domain_id: u32,
    domain_tag: &str,
    sedp_builtin_publications_writer: &mut impl RtpsStatefulWriterOperations<Vec<Locator>>,
    sedp_builtin_publication_reader: &mut impl RtpsStatefulReaderOperations<Vec<Locator>>,
    sedp_builtin_subscriptions_writer: &mut impl RtpsStatefulWriterOperations<Vec<Locator>>,
    sedp_builtin_subscriptions_reader: &mut impl RtpsStatefulReaderOperations<Vec<Locator>>,
    sedp_builtin_topics_writer: &mut impl RtpsStatefulWriterOperations<Vec<Locator>>,
    sedp_builtin_topics_reader: &mut impl RtpsStatefulReaderOperations<Vec<Locator>>,
) {
    let spdp_builtin_participant_data_reader_lock =
        rtps_shared_write_lock(&spdp_builtin_participant_data_reader_arc);
    let samples = spdp_builtin_participant_data_reader_lock
        .read_borrowed_samples(1, &[], &[], &[])
        .unwrap_or(vec![]);
    for discovered_participant in samples {
        if let Ok(participant_discovery) = ParticipantDiscovery::new(
            &discovered_participant.participant_proxy,
            domain_id as u32,
            domain_tag,
        ) {
            participant_discovery
                .discovered_participant_add_publications_writer(sedp_builtin_publications_writer);

            participant_discovery
                .discovered_participant_add_publications_reader(sedp_builtin_publication_reader);

            participant_discovery
                .discovered_participant_add_subscriptions_writer(sedp_builtin_subscriptions_writer);

            participant_discovery
                .discovered_participant_add_subscriptions_reader(sedp_builtin_subscriptions_reader);

            participant_discovery
                .discovered_participant_add_topics_writer(sedp_builtin_topics_writer);

            participant_discovery
                .discovered_participant_add_topics_reader(sedp_builtin_topics_reader);
        }
    }
}

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

const PB: u16 = 7400;
const DG: u16 = 250;
const PG: u16 = 2;
#[allow(non_upper_case_globals)]
const d0: u16 = 0;
#[allow(non_upper_case_globals)]
const _d1: u16 = 10;
#[allow(non_upper_case_globals)]
const _d2: u16 = 1;
#[allow(non_upper_case_globals)]
const d3: u16 = 11;

fn get_builtin_udp_socket(domain_id: u16) -> Option<UdpSocket> {
    for _participant_id in 0..120 {
        let socket_addr = SocketAddr::from(([127, 0, 0, 1], PB + DG * domain_id + d0));
        if let Ok(socket) = UdpSocket::bind(socket_addr) {
            return Some(socket);
        }
    }
    None
}

fn get_user_defined_udp_socket(domain_id: u16) -> Option<UdpSocket> {
    for participant_id in 0..120 {
        let socket_addr = SocketAddr::from((
            [127, 0, 0, 1],
            PB + DG * domain_id + d3 + PG * participant_id,
        ));
        if let Ok(socket) = UdpSocket::bind(socket_addr) {
            return Some(socket);
        }
    }
    None
}

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
    ) -> Option<DomainParticipantProxy> {
        let domain_participant_qos = qos.unwrap_or_default();

        // /////// Define guid prefix
        let guid_prefix = GuidPrefix([3; 12]);

        // /////// Define other configurations
        let domain_tag = Arc::new("".to_string());
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
        let socket = get_builtin_udp_socket(domain_id as u16).unwrap();
        socket.set_nonblocking(true).unwrap();
        socket
            .join_multicast_v4(
                &Ipv4Addr::from_str("239.255.0.1").unwrap(),
                &Ipv4Addr::from_str("127.0.0.1").unwrap(),
            )
            .unwrap();
        socket.set_multicast_loop_v4(true).unwrap();
        let metatraffic_transport = UdpTransport::new(socket);

        let socket = get_user_defined_udp_socket(domain_id as u16).unwrap();
        socket.set_nonblocking(true).unwrap();
        let default_transport = UdpTransport::new(socket);

        // /////// Create SPDP and SEDP endpoints
        let spdp_builtin_participant_rtps_reader = SpdpBuiltinParticipantReader::create::<
            RtpsStatelessReaderImpl<SpdpDiscoveredParticipantData>,
        >(guid_prefix, &[], &[]);
        let mut spdp_builtin_participant_rtps_writer =
            SpdpBuiltinParticipantWriter::create::<RtpsStatelessWriterImpl>(guid_prefix, &[], &[]);
        let sedp_builtin_publications_rtps_reader = SedpBuiltinPublicationsReader::create::<
            RtpsStatefulReaderImpl<SedpDiscoveredWriterData>,
        >(guid_prefix, &[], &[]);
        let sedp_builtin_publications_rtps_writer =
            SedpBuiltinPublicationsWriter::create::<RtpsStatefulWriterImpl>(guid_prefix, &[], &[]);
        let sedp_builtin_subscriptions_rtps_reader = SedpBuiltinSubscriptionsReader::create::<
            RtpsStatefulReaderImpl<SedpDiscoveredReaderData>,
        >(guid_prefix, &[], &[]);
        let sedp_builtin_subscriptions_rtps_writer =
            SedpBuiltinSubscriptionsWriter::create::<RtpsStatefulWriterImpl>(guid_prefix, &[], &[]);
        let sedp_builtin_topics_rtps_reader = SedpBuiltinTopicsReader::create::<
            RtpsStatefulReaderImpl<SedpDiscoveredTopicData>,
        >(guid_prefix, &[], &[]);
        let sedp_builtin_topics_rtps_writer =
            SedpBuiltinTopicsWriter::create::<RtpsStatefulWriterImpl>(guid_prefix, &[], &[]);

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
            rtps_shared_new(DataReaderImpl::<SpdpDiscoveredParticipantData, _>::new(
                DataReaderQos::default(),
                spdp_builtin_participant_rtps_reader,
            ));

        let spdp_builtin_participant_dds_data_writer =
            rtps_shared_new(DataWriterImpl::<SpdpDiscoveredParticipantData, _, _>::new(
                DataWriterQos::default(),
                spdp_builtin_participant_rtps_writer,
                StdTimer::new(),
            ));

        let sedp_builtin_publications_dds_data_reader =
            rtps_shared_new(DataReaderImpl::<SedpDiscoveredWriterData, _>::new(
                DataReaderQos::default(),
                sedp_builtin_publications_rtps_reader,
            ));

        let sedp_builtin_publications_dds_data_writer =
            rtps_shared_new(DataWriterImpl::<SedpDiscoveredWriterData, _, _>::new(
                DataWriterQos::default(),
                sedp_builtin_publications_rtps_writer,
                StdTimer::new(),
            ));

        let sedp_builtin_subscriptions_dds_data_reader =
            rtps_shared_new(DataReaderImpl::<SedpDiscoveredReaderData, _>::new(
                DataReaderQos::default(),
                sedp_builtin_subscriptions_rtps_reader,
            ));

        let sedp_builtin_subscriptions_dds_data_writer =
            rtps_shared_new(DataWriterImpl::<SedpDiscoveredReaderData, _, _>::new(
                DataWriterQos::default(),
                sedp_builtin_subscriptions_rtps_writer,
                StdTimer::new(),
            ));

        let sedp_builtin_topics_dds_data_reader =
            rtps_shared_new(DataReaderImpl::<SedpDiscoveredTopicData, _>::new(
                DataReaderQos::default(),
                sedp_builtin_topics_rtps_reader,
            ));

        let sedp_builtin_topics_dds_data_writer =
            rtps_shared_new(DataWriterImpl::<SedpDiscoveredTopicData, _, _>::new(
                DataWriterQos::default(),
                sedp_builtin_topics_rtps_writer,
                StdTimer::new(),
            ));

        let builtin_subscriber = rtps_shared_new(SubscriberImpl::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(Guid::new(
                guid_prefix,
                EntityId::new([0, 0, 0], BUILT_IN_READER_GROUP),
            )),
            vec![spdp_builtin_participant_dds_data_reader.clone()],
            vec![
                sedp_builtin_publications_dds_data_reader.clone(),
                sedp_builtin_subscriptions_dds_data_reader.clone(),
                sedp_builtin_topics_dds_data_reader.clone(),
            ],
        ));

        let builtin_publisher = rtps_shared_new(PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(Guid::new(
                guid_prefix,
                EntityId::new([0, 0, 0], BUILT_IN_WRITER_GROUP),
            )),
            vec![spdp_builtin_participant_dds_data_writer.clone()],
            vec![
                sedp_builtin_publications_dds_data_writer.clone(),
                sedp_builtin_subscriptions_dds_data_writer.clone(),
                sedp_builtin_topics_dds_data_writer.clone(),
            ],
            None,
            None,
        ));

        let user_defined_subscriber_list = rtps_shared_new(Vec::new());
        let user_defined_publisher_list = rtps_shared_new(Vec::new());
        let enabled = Arc::new(AtomicBool::new(false));

        let (sender, receiver) = std::sync::mpsc::sync_channel(10);
        let executor = Executor { receiver };
        let spawner = Spawner::new(sender, enabled.clone());

        let mut communication = Communication {
            version: PROTOCOLVERSION,
            vendor_id: VENDOR_ID_S2E,
            guid_prefix,
            transport: metatraffic_transport,
        };
        let builtin_publisher_arc = builtin_publisher.clone();
        let builtin_subscriber_arc = builtin_subscriber.clone();
        spawner.spawn_enabled_periodic_task(
            "builtin communication",
            move || {
                communication.send(core::slice::from_ref(&builtin_publisher_arc));
                communication.receive(core::slice::from_ref(&builtin_subscriber_arc));
            },
            std::time::Duration::from_millis(500),
        );

        let mut communication = Communication {
            version: PROTOCOLVERSION,
            vendor_id: VENDOR_ID_S2E,
            guid_prefix,
            transport: default_transport,
        };
        let user_defined_publisher_list_arc = user_defined_publisher_list.clone();
        let user_defined_subscriber_list_arc = user_defined_subscriber_list.clone();
        spawner.spawn_enabled_periodic_task(
            "user-defined communication",
            move || {
                communication.send(&rtps_shared_write_lock(&user_defined_publisher_list_arc));
                communication.receive(&rtps_shared_write_lock(&user_defined_subscriber_list_arc));
            },
            std::time::Duration::from_millis(500),
        );
        let spdp_builtin_participant_data_reader_arc =
            spdp_builtin_participant_dds_data_reader.clone();
        let domain_tag_arc = domain_tag.clone();

        spawner.spawn_enabled_periodic_task(
            "spdp discovery",
            move || {
                task_discovery(
                    &spdp_builtin_participant_data_reader_arc,
                    domain_id as u32,
                    domain_tag_arc.as_ref(),
                    rtps_shared_write_lock(&sedp_builtin_publications_dds_data_writer).as_mut(),
                    rtps_shared_write_lock(&sedp_builtin_publications_dds_data_reader).as_mut(),
                    rtps_shared_write_lock(&sedp_builtin_subscriptions_dds_data_writer).as_mut(),
                    rtps_shared_write_lock(&sedp_builtin_subscriptions_dds_data_reader).as_mut(),
                    rtps_shared_write_lock(&sedp_builtin_topics_dds_data_writer).as_mut(),
                    rtps_shared_write_lock(&sedp_builtin_topics_dds_data_reader).as_mut(),
                )
            },
            std::time::Duration::from_millis(500),
        );

        let user_defined_publisher_list_arc = user_defined_publisher_list.clone();
        let _user_defined_subscriber_list_arc = user_defined_subscriber_list.clone();
        spawner.spawn_enabled_periodic_task(
            "sedp discovery",
            move || {
                let user_defined_publisher_list_lock =
                    rtps_shared_write_lock(&user_defined_publisher_list_arc);
                for user_defined_publisher in user_defined_publisher_list_lock.iter() {
                    let user_defined_publisher_lock =
                        rtps_shared_write_lock(&user_defined_publisher);
                    user_defined_publisher_lock.process_discovery();
                }
            },
            std::time::Duration::from_millis(500),
        );

        let domain_participant = DomainParticipantImpl::new(
            guid_prefix,
            domain_id,
            domain_tag,
            domain_participant_qos,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            default_unicast_locator_list,
            default_multicast_locator_list,
            builtin_subscriber,
            builtin_publisher,
            user_defined_subscriber_list,
            user_defined_publisher_list,
            enabled,
        );

        let spdp_discovered_participant_data =
            domain_participant.as_spdp_discovered_participant_data();

        rtps_shared_write_lock(&spdp_builtin_participant_dds_data_writer)
            .write_w_timestamp(
                &spdp_discovered_participant_data,
                None,
                Time { sec: 0, nanosec: 0 },
            )
            .unwrap();

        executor.run();

        Some(DomainParticipantProxy::new(domain_participant))
    }
}

#[cfg(test)]
mod tests {
    use mockall::{mock, predicate};
    use rust_dds_api::{
        builtin_topics::ParticipantBuiltinTopicData,
        dcps_psm::{BuiltInTopicKey, InstanceStateKind, SampleStateKind, ViewStateKind},
        return_type::DDSResult,
    };
    use rust_rtps_pim::{
        behavior::{reader::writer_proxy::RtpsWriterProxy, writer::reader_proxy::RtpsReaderProxy},
        discovery::{
            sedp::builtin_endpoints::{
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
                ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
            },
            spdp::participant_proxy::ParticipantProxy,
            types::{BuiltinEndpointQos, BuiltinEndpointSet},
        },
        messages::types::Count,
        structure::types::ENTITYID_UNKNOWN,
    };

    mock! {
        DdsDataReader<T: 'static>{}

        impl<'a, T>  DataReaderBorrowedSamples<'a> for DdsDataReader<T>{
            type Samples = Vec<&'a T>;

            fn read_borrowed_samples(
                &'a self,
                max_samples: i32,
                sample_states: &[SampleStateKind],
                view_states: &[ViewStateKind],
                instance_states: &[InstanceStateKind],
            ) -> DDSResult<Vec<&'static T>>;
        }

    }

    use super::*;
    mock! {
        StatefulReader {}

        impl RtpsStatefulReaderOperations<Vec<Locator>> for StatefulReader {
            type WriterProxyType = ();
            fn matched_writer_add(&mut self, a_writer_proxy: RtpsWriterProxy<Vec<Locator>>);
            fn matched_writer_remove(&mut self, writer_proxy_guid: &Guid);
            fn matched_writer_lookup(&self, a_writer_guid: &Guid) -> Option<&'static ()>;
        }
    }

    mock! {
        StatefulWriter {}

        impl RtpsStatefulWriterOperations<Vec<Locator>> for StatefulWriter {
            type ReaderProxyType = ();
            fn matched_reader_add(&mut self, a_reader_proxy: RtpsReaderProxy<Vec<Locator>>);
            fn matched_reader_remove(&mut self, reader_proxy_guid: &Guid);
            fn matched_reader_lookup(&self, a_reader_guid: &Guid) -> Option<&'static ()>;
            fn is_acked_by_all(&self) -> bool;
        }
    }

    #[test]
    fn discovery_task_all_sedp_endpoints() {
        static RETURN_SPDP_DATA: SpdpDiscoveredParticipantData = SpdpDiscoveredParticipantData {
            dds_participant_data: ParticipantBuiltinTopicData {
                key: BuiltInTopicKey { value: [5; 16] },
                user_data: rust_dds_api::infrastructure::qos_policy::UserDataQosPolicy {
                    value: vec![],
                },
            },
            participant_proxy: ParticipantProxy {
                domain_id: 1,
                domain_tag: String::new(),
                protocol_version: PROTOCOLVERSION,
                guid_prefix: GuidPrefix([5; 12]),
                vendor_id: VENDOR_ID_S2E,
                expects_inline_qos: false,
                metatraffic_unicast_locator_list: vec![],
                metatraffic_multicast_locator_list: vec![],
                default_unicast_locator_list: vec![],
                default_multicast_locator_list: vec![],
                available_builtin_endpoints: BuiltinEndpointSet(
                    BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_ANNOUNCER
                        | BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR
                        | BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER
                        | BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR
                        | BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER
                        | BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR
                        | BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_ANNOUNCER
                        | BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_DETECTOR,
                ),
                manual_liveliness_count: Count(1),
                builtin_endpoint_qos: BuiltinEndpointQos(0),
            },
            lease_duration: rust_rtps_pim::behavior::types::Duration {
                seconds: 100,
                fraction: 0,
            },
        };

        let mut mock_spdp_data_reader = MockDdsDataReader::new();
        mock_spdp_data_reader
            .expect_read_borrowed_samples()
            .returning(|_, _, _, _| Ok(vec![&RETURN_SPDP_DATA]));

        let mut mock_builtin_publications_writer = MockStatefulWriter::new();
        mock_builtin_publications_writer
            .expect_matched_reader_add()
            .with(predicate::eq(RtpsReaderProxy {
                remote_reader_guid: Guid::new(
                    GuidPrefix([5; 12]),
                    ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
                ),
                remote_group_entity_id: ENTITYID_UNKNOWN,
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
                expects_inline_qos: false,
            }))
            .once()
            .return_const(());

        let mut mock_builtin_publications_reader = MockStatefulReader::new();
        mock_builtin_publications_reader
            .expect_matched_writer_add()
            .with(predicate::eq(RtpsWriterProxy {
                remote_writer_guid: Guid::new(
                    GuidPrefix([5; 12]),
                    ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
                ),
                remote_group_entity_id: ENTITYID_UNKNOWN,
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
                data_max_size_serialized: None,
            }))
            .once()
            .return_const(());

        let mut mock_builtin_subscriptions_writer = MockStatefulWriter::new();
        mock_builtin_subscriptions_writer
            .expect_matched_reader_add()
            .with(predicate::eq(RtpsReaderProxy {
                remote_reader_guid: Guid::new(
                    GuidPrefix([5; 12]),
                    ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
                ),
                remote_group_entity_id: ENTITYID_UNKNOWN,
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
                expects_inline_qos: false,
            }))
            .once()
            .return_const(());

        let mut mock_builtin_subscriptions_reader = MockStatefulReader::new();
        mock_builtin_subscriptions_reader
            .expect_matched_writer_add()
            .with(predicate::eq(RtpsWriterProxy {
                remote_writer_guid: Guid::new(
                    GuidPrefix([5; 12]),
                    ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
                ),
                remote_group_entity_id: ENTITYID_UNKNOWN,
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
                data_max_size_serialized: None,
            }))
            .once()
            .return_const(());

        let mut mock_builtin_topics_writer = MockStatefulWriter::new();
        mock_builtin_topics_writer
            .expect_matched_reader_add()
            .with(predicate::eq(RtpsReaderProxy {
                remote_reader_guid: Guid::new(
                    GuidPrefix([5; 12]),
                    ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
                ),
                remote_group_entity_id: ENTITYID_UNKNOWN,
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
                expects_inline_qos: false,
            }))
            .once()
            .return_const(());

        let mut mock_builtin_topics_reader = MockStatefulReader::new();
        mock_builtin_topics_reader
            .expect_matched_writer_add()
            .with(predicate::eq(RtpsWriterProxy {
                remote_writer_guid: Guid::new(
                    GuidPrefix([5; 12]),
                    ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
                ),
                remote_group_entity_id: ENTITYID_UNKNOWN,
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
                data_max_size_serialized: None,
            }))
            .once()
            .return_const(());

        task_discovery(
            &rtps_shared_new(mock_spdp_data_reader),
            1,
            "",
            &mut mock_builtin_publications_writer,
            &mut mock_builtin_publications_reader,
            &mut mock_builtin_subscriptions_writer,
            &mut mock_builtin_subscriptions_reader,
            &mut mock_builtin_topics_writer,
            &mut mock_builtin_topics_reader,
        );
    }
}
