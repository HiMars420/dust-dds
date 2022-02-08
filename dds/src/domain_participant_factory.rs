use async_std::stream::StreamExt;
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    ops::Deref,
    str::FromStr,
    sync::{
        atomic::{self, AtomicBool},
        mpsc::{Receiver, SyncSender},
        Arc, Mutex,
    },
};

use rust_dds_api::{
    dcps_psm::{DomainId, StatusMask},
    domain::domain_participant_listener::DomainParticipantListener,
    infrastructure::qos::{
        DataReaderQos, DataWriterQos, DomainParticipantFactoryQos, DomainParticipantQos,
        PublisherQos, SubscriberQos, TopicQos,
    },
    return_type::DDSResult,
    subscription::data_reader::DataReader,
};
use rust_dds_rtps_implementation::{
    dds_impl::{
        data_reader_proxy::{DataReaderAttributes, RtpsReader, Samples},
        data_writer_proxy::{DataWriterAttributes, RtpsWriter},
        domain_participant_proxy::{DomainParticipantAttributes, DomainParticipantProxy},
        publisher_proxy::PublisherAttributes,
        subscriber_proxy::SubscriberAttributes,
        topic_proxy::TopicAttributes,
    },
    dds_type::DdsType,
    rtps_impl::{
        rtps_group_impl::RtpsGroupImpl, rtps_reader_locator_impl::RtpsReaderLocatorAttributesImpl,
        rtps_stateful_reader_impl::RtpsStatefulReaderImpl,
        rtps_stateful_writer_impl::RtpsStatefulWriterImpl,
        rtps_stateless_reader_impl::RtpsStatelessReaderImpl,
        rtps_stateless_writer_impl::RtpsStatelessWriterImpl,
        rtps_writer_proxy_impl::RtpsWriterProxyImpl,
    },
    utils::{rtps_structure::RtpsStructure, shared_object::RtpsShared},
};
use rust_rtps_pim::{
    behavior::{
        reader::{
            stateful_reader::RtpsStatefulReaderOperations, writer_proxy::RtpsWriterProxyConstructor,
        },
        writer::{
            reader_locator::RtpsReaderLocatorConstructor, reader_proxy::RtpsReaderProxyConstructor,
            stateful_writer::RtpsStatefulWriterOperations,
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

use crate::{
    communication::Communication,
    data_representation_builtin_endpoints::{
        sedp_discovered_reader_data::SedpDiscoveredReaderData,
        sedp_discovered_topic_data::SedpDiscoveredTopicData,
        sedp_discovered_writer_data::SedpDiscoveredWriterData,
        spdp_discovered_participant_data::SpdpDiscoveredParticipantData,
    },
    udp_transport::UdpTransport,
};

pub struct RtpsStructureImpl;

impl RtpsStructure for RtpsStructureImpl {
    type StatelessWriter = RtpsStatelessWriterImpl;
    type StatefulWriter = RtpsStatefulWriterImpl;
    type StatelessReader = RtpsStatelessReaderImpl;
    type StatefulReader = RtpsStatefulReaderImpl;
}

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
    pub fn new(task_sender: SyncSender<EnabledPeriodicTask>) -> Self {
        Self {
            task_sender,
            enabled: Arc::new(AtomicBool::new(false)),
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

fn _spdp_task_discovery<T>(
    spdp_builtin_participant_data_reader_arc: &RtpsShared<
        impl DataReader<SpdpDiscoveredParticipantData, Samples = T>,
    >,
    domain_id: u32,
    domain_tag: &str,
    sedp_builtin_publications_writer: &mut impl RtpsStatefulWriterOperations<
        ReaderProxyType = impl RtpsReaderProxyConstructor,
    >,
    sedp_builtin_publication_reader: &mut impl RtpsStatefulReaderOperations<
        WriterProxyType = impl RtpsWriterProxyConstructor,
    >,
    sedp_builtin_subscriptions_writer: &mut impl RtpsStatefulWriterOperations<
        ReaderProxyType = impl RtpsReaderProxyConstructor,
    >,
    sedp_builtin_subscriptions_reader: &mut impl RtpsStatefulReaderOperations<
        WriterProxyType = impl RtpsWriterProxyConstructor,
    >,
    sedp_builtin_topics_writer: &mut impl RtpsStatefulWriterOperations<
        ReaderProxyType = impl RtpsReaderProxyConstructor,
    >,
    sedp_builtin_topics_reader: &mut impl RtpsStatefulReaderOperations<
        WriterProxyType = impl RtpsWriterProxyConstructor,
    >,
) where
    T: Deref<Target = [SpdpDiscoveredParticipantData]>,
{
    let mut spdp_builtin_participant_data_reader_lock =
        spdp_builtin_participant_data_reader_arc.write_lock();
    if let Ok(samples) = spdp_builtin_participant_data_reader_lock.read(1, &[], &[], &[]) {
        for discovered_participant in samples.into_iter() {
            if let Ok(participant_discovery) = ParticipantDiscovery::new(
                &discovered_participant.participant_proxy,
                &(domain_id as u32),
                domain_tag,
            ) {
                participant_discovery.discovered_participant_add_publications_writer(
                    sedp_builtin_publications_writer,
                );

                participant_discovery.discovered_participant_add_publications_reader(
                    sedp_builtin_publication_reader,
                );

                participant_discovery.discovered_participant_add_subscriptions_writer(
                    sedp_builtin_subscriptions_writer,
                );

                participant_discovery.discovered_participant_add_subscriptions_reader(
                    sedp_builtin_subscriptions_reader,
                );

                participant_discovery
                    .discovered_participant_add_topics_writer(sedp_builtin_topics_writer);

                participant_discovery
                    .discovered_participant_add_topics_reader(sedp_builtin_topics_reader);
            }
        }
    }
}

fn _task_sedp_discovery(
    sedp_builtin_publications_data_reader: &RtpsShared<
        impl DataReader<SedpDiscoveredWriterData, Samples = Samples<SedpDiscoveredWriterData>>,
    >,
    subscriber_list: &RtpsShared<Vec<RtpsShared<SubscriberAttributes<RtpsStructureImpl>>>>,
) {
    let mut sedp_builtin_publications_data_reader_lock =
        sedp_builtin_publications_data_reader.write_lock();
    if let Ok(samples) = sedp_builtin_publications_data_reader_lock.read(1, &[], &[], &[]) {
        if let Some(sample) = samples.into_iter().next() {
            let topic_name = &sample.publication_builtin_topic_data.topic_name;
            let type_name = &sample.publication_builtin_topic_data.type_name;
            let subscriber_list_lock = subscriber_list.read_lock();
            for subscriber in subscriber_list_lock.iter() {
                let subscriber_lock = subscriber.read_lock();
                for data_reader in subscriber_lock.data_reader_list.iter() {
                    let mut data_reader_lock = data_reader.write().unwrap();
                    let reader_topic_name = &data_reader_lock.topic.read_lock().topic_name.clone();
                    let reader_type_name = data_reader_lock.topic.read_lock().type_name;
                    if topic_name == reader_topic_name && type_name == reader_type_name {
                        let writer_proxy = RtpsWriterProxyImpl::new(
                            sample.writer_proxy.remote_writer_guid,
                            sample.writer_proxy.unicast_locator_list.as_ref(),
                            sample.writer_proxy.multicast_locator_list.as_ref(),
                            sample.writer_proxy.data_max_size_serialized,
                            sample.writer_proxy.remote_group_entity_id,
                        );
                        match &mut data_reader_lock.rtps_reader {
                            RtpsReader::Stateless(_) => (),
                            RtpsReader::Stateful(rtps_stateful_reader) => {
                                rtps_stateful_reader.matched_writer_add(writer_proxy)
                            }
                        };
                    }
                }
            }
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

pub struct DomainParticipantFactory {
    participant_list: Mutex<Vec<RtpsShared<DomainParticipantAttributes<RtpsStructureImpl>>>>,
}

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
        &self,
        domain_id: DomainId,
        qos: Option<DomainParticipantQos>,
        _a_listener: Option<Box<dyn DomainParticipantListener>>,
        _mask: StatusMask,
    ) -> Option<DomainParticipantProxy<RtpsStructureImpl>> {
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
        let _default_transport = UdpTransport::new(socket);

        // //////// Create the domain participant
        let domain_participant = RtpsShared::new(DomainParticipantAttributes::new(
            guid_prefix,
            domain_id,
            domain_tag,
            domain_participant_qos,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            default_unicast_locator_list,
            default_multicast_locator_list,
        ));

        // ///////// Create the built-in publisher and subcriber
        let builtin_subscriber = RtpsShared::new(SubscriberAttributes::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(Guid::new(
                guid_prefix,
                EntityId::new([0, 0, 0], BUILT_IN_READER_GROUP),
            )),
            domain_participant.downgrade(),
        ));
        domain_participant
            .write()
            .unwrap()
            .builtin_subscriber_list
            .push(builtin_subscriber.clone());

        let builtin_publisher = RtpsShared::new(PublisherAttributes::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(Guid::new(
                guid_prefix,
                EntityId::new([0, 0, 0], BUILT_IN_WRITER_GROUP),
            )),
            None,
            domain_participant.downgrade(),
        ));
        domain_participant
            .write()
            .unwrap()
            .builtin_subscriber_list
            .push(builtin_subscriber.clone());

        // ///////// Create built-in DDS data readers and data writers

        // ////////// SPDP built-in topic, reader and writer
        let spdp_discovered_participant_topic = RtpsShared::new(TopicAttributes::new(
            TopicQos::default(),
            SpdpDiscoveredParticipantData::type_name(),
            "DCPSParticipant",
            domain_participant.downgrade(),
        ));

        let spdp_builtin_participant_rtps_reader =
            SpdpBuiltinParticipantReader::create::<RtpsStatelessReaderImpl>(guid_prefix, &[], &[]);

        let _spdp_builtin_participant_data_reader = RtpsShared::new(DataReaderAttributes::new(
            DataReaderQos::default(),
            RtpsReader::Stateless(spdp_builtin_participant_rtps_reader),
            spdp_discovered_participant_topic.clone(),
            builtin_subscriber.downgrade(),
        ));

        let mut spdp_builtin_participant_rtps_writer =
            SpdpBuiltinParticipantWriter::create::<RtpsStatelessWriterImpl>(guid_prefix, &[], &[]);

        let spdp_discovery_locator = RtpsReaderLocatorAttributesImpl::new(
            Locator::new(
                LOCATOR_KIND_UDPv4,
                7400,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239, 255, 0, 1],
            ),
            false,
        );

        spdp_builtin_participant_rtps_writer.reader_locator_add(spdp_discovery_locator);

        let _spdp_builtin_participant_data_writer = RtpsShared::new(DataWriterAttributes::new(
            DataWriterQos::default(),
            RtpsWriter::Stateless(spdp_builtin_participant_rtps_writer),
            spdp_discovered_participant_topic.clone(),
            builtin_publisher.downgrade(),
        ));

        // ////////// SEDP built-in publication topic, reader and writer
        let sedp_builtin_publications_topic = RtpsShared::new(TopicAttributes::new(
            TopicQos::default(),
            SedpDiscoveredWriterData::type_name(),
            "DCPSPublication",
            domain_participant.downgrade(),
        ));

        let sedp_builtin_publications_rtps_reader =
            SedpBuiltinPublicationsReader::create::<RtpsStatefulReaderImpl>(guid_prefix, &[], &[]);
        let _sedp_builtin_publications_data_reader = RtpsShared::new(DataReaderAttributes::new(
            DataReaderQos::default(),
            RtpsReader::Stateful(sedp_builtin_publications_rtps_reader),
            sedp_builtin_publications_topic.clone(),
            builtin_subscriber.downgrade(),
        ));

        let sedp_builtin_publications_rtps_writer =
            SedpBuiltinPublicationsWriter::create::<RtpsStatefulWriterImpl>(guid_prefix, &[], &[]);
        let _sedp_builtin_publications_data_writer = RtpsShared::new(DataWriterAttributes::new(
            DataWriterQos::default(),
            RtpsWriter::Stateful(sedp_builtin_publications_rtps_writer),
            sedp_builtin_publications_topic.clone(),
            builtin_publisher.downgrade(),
        ));

        // ////////// SEDP built-in subcriptions topic, reader and writer
        let sedp_builtin_subscriptions_topic = RtpsShared::new(TopicAttributes::new(
            TopicQos::default(),
            SedpDiscoveredReaderData::type_name(),
            "DCPSSubscription",
            domain_participant.downgrade(),
        ));

        let sedp_builtin_subscriptions_rtps_reader =
            SedpBuiltinSubscriptionsReader::create::<RtpsStatefulReaderImpl>(guid_prefix, &[], &[]);
        let _sedp_builtin_subscriptions_data_reader = RtpsShared::new(DataReaderAttributes::new(
            DataReaderQos::default(),
            RtpsReader::Stateful(sedp_builtin_subscriptions_rtps_reader),
            sedp_builtin_subscriptions_topic.clone(),
            builtin_subscriber.downgrade(),
        ));

        let sedp_builtin_subscriptions_rtps_writer =
            SedpBuiltinSubscriptionsWriter::create::<RtpsStatefulWriterImpl>(guid_prefix, &[], &[]);
        let _sedp_builtin_subscriptions_dds_data_writer =
            RtpsShared::new(DataWriterAttributes::new(
                DataWriterQos::default(),
                RtpsWriter::Stateful(sedp_builtin_subscriptions_rtps_writer),
                sedp_builtin_subscriptions_topic.clone(),
                builtin_publisher.downgrade(),
            ));

        // ////////// SEDP built-in topics topic, reader and writer
        let sedp_builtin_topics_topic = RtpsShared::new(TopicAttributes::new(
            TopicQos::default(),
            SedpDiscoveredTopicData::type_name(),
            "DCPSTopic",
            domain_participant.downgrade(),
        ));

        let sedp_builtin_topics_rtps_reader =
            SedpBuiltinTopicsReader::create::<RtpsStatefulReaderImpl>(guid_prefix, &[], &[]);
        let _sedp_builtin_topics_data_reader = RtpsShared::new(DataReaderAttributes::new(
            DataReaderQos::default(),
            RtpsReader::Stateful(sedp_builtin_topics_rtps_reader),
            sedp_builtin_topics_topic.clone(),
            builtin_subscriber.downgrade(),
        ));

        let sedp_builtin_topics_rtps_writer =
            SedpBuiltinTopicsWriter::create::<RtpsStatefulWriterImpl>(guid_prefix, &[], &[]);
        let _sedp_builtin_topics_data_writer = RtpsShared::new(DataWriterAttributes::new(
            DataWriterQos::default(),
            RtpsWriter::Stateful(sedp_builtin_topics_rtps_writer),
            sedp_builtin_topics_topic.clone(),
            builtin_publisher.downgrade(),
        ));

        // ////////// Task creation
        let (_sender, receiver) = std::sync::mpsc::sync_channel(10);
        let executor = Executor { receiver };
        // let _spawner = Spawner::new(sender, domain_participant.read().unwrap().enabled.clone());

        let mut _communication = Communication {
            version: PROTOCOLVERSION,
            vendor_id: VENDOR_ID_S2E,
            guid_prefix,
            transport: metatraffic_transport,
        };
        // let builtin_publisher_arc = builtin_publisher.clone();
        // let builtin_subscriber_arc = builtin_subscriber.clone();
        // spawner.spawn_enabled_periodic_task(
        //     "builtin communication",
        //     move || {
        //         communication.send(core::slice::from_ref(&builtin_publisher_arc));
        //         communication.receive(core::slice::from_ref(&builtin_subscriber_arc));
        //     },
        //     std::time::Duration::from_millis(500),
        // );

        // let mut communication = Communication {
        //     version: PROTOCOLVERSION,
        //     vendor_id: VENDOR_ID_S2E,
        //     guid_prefix,
        //     transport: default_transport,
        // };
        // let user_defined_publisher_list_arc = user_defined_publisher_list.clone();
        // let user_defined_subscriber_list_arc = user_defined_subscriber_list.clone();
        // spawner.spawn_enabled_periodic_task(
        //     "user-defined communication",
        //     move || {
        //         communication.send(&rtps_shared_write_lock(&user_defined_publisher_list_arc));
        //         communication.receive(&rtps_shared_write_lock(&user_defined_subscriber_list_arc));
        //     },
        //     std::time::Duration::from_millis(500),
        // );
        // let spdp_builtin_participant_data_reader_arc =
        //     spdp_builtin_participant_dds_data_reader.clone();
        // let domain_tag_arc = domain_tag.clone();
        // let sedp_builtin_publications_dds_data_reader_arc =
        //     sedp_builtin_publications_dds_data_reader.clone();

        // spawner.spawn_enabled_periodic_task(
        //     "spdp discovery",
        //     move || {
        //         spdp_task_discovery(
        //             &spdp_builtin_participant_data_reader_arc,
        //             domain_id as u32,
        //             domain_tag_arc.as_ref(),
        //             rtps_shared_write_lock(&sedp_builtin_publications_dds_data_writer)
        //                 .as_mut()
        //                 .try_as_stateful_writer()
        //                 .unwrap(),
        //             rtps_shared_write_lock(&sedp_builtin_publications_dds_data_reader_arc)
        //                 .as_mut()
        //                 .try_as_stateful_reader()
        //                 .unwrap(),
        //             rtps_shared_write_lock(&sedp_builtin_subscriptions_dds_data_writer)
        //                 .as_mut()
        //                 .try_as_stateful_writer()
        //                 .unwrap(),
        //             rtps_shared_write_lock(&sedp_builtin_subscriptions_dds_data_reader)
        //                 .as_mut()
        //                 .try_as_stateful_reader()
        //                 .unwrap(),
        //             rtps_shared_write_lock(&sedp_builtin_topics_dds_data_writer)
        //                 .as_mut()
        //                 .try_as_stateful_writer()
        //                 .unwrap(),
        //             rtps_shared_write_lock(&sedp_builtin_topics_dds_data_reader)
        //                 .as_mut()
        //                 .try_as_stateful_reader()
        //                 .unwrap(),
        //         )
        //     },
        //     std::time::Duration::from_millis(500),
        // );

        // let user_defined_publisher_list_arc = user_defined_publisher_list.clone();
        // let _user_defined_subscriber_list_arc = user_defined_subscriber_list.clone();
        // spawner.spawn_enabled_periodic_task(
        //     "sedp discovery",
        //     move || {
        //         let user_defined_publisher_list_lock =
        //             rtps_shared_write_lock(&user_defined_publisher_list_arc);
        //         for user_defined_publisher in user_defined_publisher_list_lock.iter() {
        //             let _user_defined_publisher_lock =
        //                 rtps_shared_write_lock(&user_defined_publisher);
        //             // user_defined_publisher_lock.process_discovery();
        //         }
        //     },
        //     std::time::Duration::from_millis(500),
        // );

        // let user_defined_publisher_list_arc = user_defined_publisher_list.clone();
        // let user_defined_subscriber_list_arc = user_defined_subscriber_list.clone();
        // let sedp_builtin_publications_dds_data_reader_arc =
        // sedp_builtin_publications_dds_data_reader.clone();
        // spawner.spawn_enabled_periodic_task(
        //     "sedp discovery",
        //     move || {
        //         task_sedp_discovery(
        //             &sedp_builtin_publications_dds_data_reader_arc,
        //             &user_defined_subscriber_list_arc,
        //         )
        //     },
        //     std::time::Duration::from_millis(500),
        // );

        // let spdp_discovered_participant_data =
        //     domain_participant.as_spdp_discovered_participant_data();

        // rtps_shared_write_lock(&spdp_builtin_participant_dds_data_writer)
        //     .write_w_timestamp(
        //         &spdp_discovered_participant_data,
        //         None,
        //         Time { sec: 0, nanosec: 0 },
        //     )
        //     .unwrap();

        executor.run();

        self.participant_list
            .lock()
            .unwrap()
            .push(domain_participant.clone());

        Some(DomainParticipantProxy::new(domain_participant.downgrade()))
    }

    /// This operation deletes an existing DomainParticipant. This operation can only be invoked if all domain entities belonging to
    /// the participant have already been deleted. Otherwise the error PRECONDITION_NOT_MET is returned.
    /// Possible error codes returned in addition to the standard ones: PRECONDITION_NOT_MET.
    pub fn delete_participant(
        &self,
        _a_participant: DomainParticipantProxy<RtpsStructureImpl>,
    ) -> DDSResult<()> {
        todo!()
    }

    /// This operation returns the DomainParticipantFactory singleton. The operation is idempotent, that is, it can be called multiple
    /// times without side-effects and it will return the same DomainParticipantFactory instance.
    /// The get_instance operation is a static operation implemented using the syntax of the native language and can therefore not be
    /// expressed in the IDL PSM.
    /// The pre-defined value TheParticipantFactory can also be used as an alias for the singleton factory returned by the operation
    /// get_instance.
    pub fn get_instance() -> Self {
        Self {
            participant_list: Mutex::new(Vec::new()),
        }
    }

    /// This operation retrieves a previously created DomainParticipant belonging to specified domain_id. If no such
    /// DomainParticipant exists, the operation will return a ‘nil’ value.
    /// If multiple DomainParticipant entities belonging to that domain_id exist, then the operation will return one of them. It is not
    /// specified which one.
    pub fn lookup_participant(
        &self,
        _domain_id: DomainId,
    ) -> DomainParticipantProxy<RtpsStructureImpl> {
        todo!()
    }

    /// This operation sets a default value of the DomainParticipant QoS policies which will be used for newly created
    /// DomainParticipant entities in the case where the QoS policies are defaulted in the create_participant operation.
    /// This operation will check that the resulting policies are self consistent; if they are not, the operation will have no effect and
    /// return INCONSISTENT_POLICY.
    pub fn set_default_participant_qos(&self, _qos: DomainParticipantQos) -> DDSResult<()> {
        todo!()
    }

    /// This operation retrieves the default value of the DomainParticipant QoS, that is, the QoS policies which will be used for
    /// newly created DomainParticipant entities in the case where the QoS policies are defaulted in the create_participant
    /// operation.
    /// The values retrieved get_default_participant_qos will match the set of values specified on the last successful call to
    /// set_default_participant_qos, or else, if the call was never made, the default values listed in the QoS table in 2.2.3,
    /// Supported QoS.
    pub fn get_default_participant_qos(&self) -> DDSResult<DomainParticipantQos> {
        todo!()
    }

    /// This operation sets the value of the DomainParticipantFactory QoS policies. These policies control the behavior of the object
    /// a factory for entities.
    /// Note that despite having QoS, the DomainParticipantFactory is not an Entity.
    /// This operation will check that the resulting policies are self consistent; if they are not, the operation will have no effect and
    /// return INCONSISTENT_POLICY.
    pub fn set_qos(&self, _qos: DomainParticipantFactoryQos) -> DDSResult<()> {
        todo!()
    }

    /// This operation returns the value of the DomainParticipantFactory QoS policies.
    pub fn get_qos(&self) -> DomainParticipantFactoryQos {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use mockall::{mock, predicate};
    use rust_dds_api::{
        builtin_topics::{ParticipantBuiltinTopicData, PublicationBuiltinTopicData},
        dcps_psm::{
            BuiltInTopicKey, Duration, InstanceHandle, InstanceStateKind, LivelinessChangedStatus,
            RequestedDeadlineMissedStatus, RequestedIncompatibleQosStatus, SampleLostStatus,
            SampleRejectedStatus, SampleStateKind, SubscriptionMatchedStatus, ViewStateKind,
        },
        infrastructure::{
            qos::SubscriberQos,
            qos_policy::{
                DeadlineQosPolicy, DestinationOrderQosPolicy, DurabilityQosPolicy,
                DurabilityServiceQosPolicy, GroupDataQosPolicy, LatencyBudgetQosPolicy,
                LifespanQosPolicy, LivelinessQosPolicy, OwnershipQosPolicy,
                OwnershipStrengthQosPolicy, PartitionQosPolicy, PresentationQosPolicy,
                ReliabilityQosPolicy, ReliabilityQosPolicyKind, TopicDataQosPolicy,
                UserDataQosPolicy,
            },
            read_condition::ReadCondition,
            sample_info::SampleInfo,
        },
        return_type::DDSResult,
        subscription::query_condition::QueryCondition,
    };
    use rust_dds_rtps_implementation::{
        rtps_impl::{
            rtps_group_impl::RtpsGroupImpl, rtps_reader_proxy_impl::RtpsReaderProxyAttributesImpl,
            rtps_writer_proxy_impl::RtpsWriterProxyImpl,
        },
        utils::shared_object::RtpsWeak,
    };
    use rust_rtps_pim::{
        behavior::{
            reader::writer_proxy::RtpsWriterProxyConstructor,
            writer::reader_proxy::RtpsReaderProxyConstructor,
        },
        discovery::{
            sedp::builtin_endpoints::{
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
                ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
            },
            types::{BuiltinEndpointQos, BuiltinEndpointSet},
        },
        messages::types::Count,
        structure::types::{EntityId, Guid, BUILT_IN_READER_GROUP, ENTITYID_UNKNOWN},
    };

    mock! {
        DdsDataReader<Foo: 'static>{}

        impl<Foo>  DataReader<Foo> for DdsDataReader<Foo>{
            type Samples = Samples<Foo>;
            type Subscriber = ();
            type TopicDescription = ();
            fn read(
                &mut self,
                max_samples: i32,
                sample_states: &[SampleStateKind],
                view_states: &[ViewStateKind],
                instance_states: &[InstanceStateKind],
            ) -> DDSResult<Samples<Foo>>;

            fn take(
                &self,
                data_values: &mut [Foo],
                sample_infos: &mut [SampleInfo],
                max_samples: i32,
                sample_states: &[SampleStateKind],
                view_states: &[ViewStateKind],
                instance_states: &[InstanceStateKind],
            ) -> DDSResult<()>;

            fn read_w_condition(
                &self,
                data_values: &mut [Foo],
                sample_infos: &mut [SampleInfo],
                max_samples: i32,
                a_condition: ReadCondition,
            ) -> DDSResult<()>;

            fn take_w_condition(
                &self,
                data_values: &mut [Foo],
                sample_infos: &mut [SampleInfo],
                max_samples: i32,
                a_condition: ReadCondition,
            ) -> DDSResult<()>;

            fn read_next_sample(
                &self,
                data_value: &mut [Foo],
                sample_info: &mut [SampleInfo],
            ) -> DDSResult<()>;

            fn take_next_sample(
                &self,
                data_value: &mut [Foo],
                sample_info: &mut [SampleInfo],
            ) -> DDSResult<()>;

            fn read_instance(
                &self,
                data_values: &mut [Foo],
                sample_infos: &mut [SampleInfo],
                max_samples: i32,
                a_handle: InstanceHandle,
                sample_states: &[SampleStateKind],
                view_states: &[ViewStateKind],
                instance_states: &[InstanceStateKind],
            ) -> DDSResult<()>;

            fn take_instance(
                &self,
                data_values: &mut [Foo],
                sample_infos: &mut [SampleInfo],
                max_samples: i32,
                a_handle: InstanceHandle,
                sample_states: &[SampleStateKind],
                view_states: &[ViewStateKind],
                instance_states: &[InstanceStateKind],
            ) -> DDSResult<()>;

            fn read_next_instance(
                &self,
                data_values: &mut [Foo],
                sample_infos: &mut [SampleInfo],
                max_samples: i32,
                previous_handle: InstanceHandle,
                sample_states: &[SampleStateKind],
                view_states: &[ViewStateKind],
                instance_states: &[InstanceStateKind],
            ) -> DDSResult<()>;

            fn take_next_instance(
                &self,
                data_values: &mut [Foo],
                sample_infos: &mut [SampleInfo],
                max_samples: i32,
                previous_handle: InstanceHandle,
                sample_states: &[SampleStateKind],
                view_states: &[ViewStateKind],
                instance_states: &[InstanceStateKind],
            ) -> DDSResult<()>;

            fn read_next_instance_w_condition(
                &self,
                data_values: &mut [Foo],
                sample_infos: &mut [SampleInfo],
                max_samples: i32,
                previous_handle: InstanceHandle,
                a_condition: ReadCondition,
            ) -> DDSResult<()>;


            fn take_next_instance_w_condition(
                &self,
                data_values: &mut [Foo],
                sample_infos: &mut [SampleInfo],
                max_samples: i32,
                previous_handle: InstanceHandle,
                a_condition: ReadCondition,
            ) -> DDSResult<()>;


            fn return_loan(
                &self,
                data_values: &mut [Foo],
                sample_infos: &mut [SampleInfo],
            ) -> DDSResult<()>;

            fn get_key_value(&self, key_holder: &mut Foo, handle: InstanceHandle) -> DDSResult<()>;


            fn lookup_instance(&self, instance: &Foo) -> InstanceHandle;


            fn create_readcondition(
                &self,
                sample_states: &[SampleStateKind],
                view_states: &[ViewStateKind],
                instance_states: &[InstanceStateKind],
            ) -> ReadCondition;


            fn create_querycondition(
                &self,
                sample_states: &[SampleStateKind],
                view_states: &[ViewStateKind],
                instance_states: &[InstanceStateKind],
                query_expression: &'static str,
                query_parameters: &[&'static str],
            ) -> QueryCondition;


            fn delete_readcondition(&self, a_condition: ReadCondition) -> DDSResult<()>;

            fn get_liveliness_changed_status(&self, status: &mut LivelinessChangedStatus) -> DDSResult<()>;


            fn get_requested_deadline_missed_status(
                &self,
                status: &mut RequestedDeadlineMissedStatus,
            ) -> DDSResult<()>;


            fn get_requested_incompatible_qos_status(
                &self,
                status: &mut RequestedIncompatibleQosStatus,
            ) -> DDSResult<()>;

            fn get_sample_lost_status(&self, status: &mut SampleLostStatus) -> DDSResult<()>;


            fn get_sample_rejected_status(&self, status: &mut SampleRejectedStatus) -> DDSResult<()>;


            fn get_subscription_matched_status(
                &self,
                status: &mut SubscriptionMatchedStatus,
            ) -> DDSResult<()>;


            fn get_topicdescription(&self) -> DDSResult<()>;

            fn get_subscriber(&self) -> DDSResult<()>;


            fn delete_contained_entities(&self) -> DDSResult<()>;

            fn wait_for_historical_data(&self) -> DDSResult<()>;


            fn get_matched_publication_data(
                &self,
                publication_data: &mut PublicationBuiltinTopicData,
                publication_handle: InstanceHandle,
            ) -> DDSResult<()>;

            fn get_match_publication(&self, publication_handles: &mut [InstanceHandle]) -> DDSResult<()>;
        }

    }

    use crate::data_representation_builtin_endpoints::{
        sedp_discovered_writer_data::RtpsWriterProxy,
        spdp_discovered_participant_data::ParticipantProxy,
    };

    use super::*;
    mock! {
        StatefulReader {}

        impl RtpsStatefulReaderOperations for StatefulReader {
            type WriterProxyType = RtpsWriterProxyImpl;
            fn matched_writer_add(&mut self, a_writer_proxy: RtpsWriterProxyImpl);
            fn matched_writer_remove(&mut self, writer_proxy_guid: &Guid);
            fn matched_writer_lookup(&self, a_writer_guid: &Guid) -> Option<&'static RtpsWriterProxyImpl>;
        }
    }

    mock! {
        StatefulWriter {}

        impl RtpsStatefulWriterOperations for StatefulWriter {
            type ReaderProxyType = RtpsReaderProxyAttributesImpl;
            fn matched_reader_add(&mut self, a_reader_proxy: RtpsReaderProxyAttributesImpl);
            fn matched_reader_remove(&mut self, reader_proxy_guid: &Guid);
            fn matched_reader_lookup(&self, a_reader_guid: &Guid) -> Option<&'static RtpsReaderProxyAttributesImpl>;
            fn is_acked_by_all(&self) -> bool;
        }
    }

    #[test]
    fn discovery_task_all_sedp_endpoints() {
        let mut mock_spdp_data_reader = MockDdsDataReader::new();
        mock_spdp_data_reader.expect_read().returning(|_, _, _, _| {
            Ok(Samples {
                samples: vec![SpdpDiscoveredParticipantData {
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
                }],
            })
        });

        let mut mock_builtin_publications_writer = MockStatefulWriter::new();
        mock_builtin_publications_writer
            .expect_matched_reader_add()
            .with(predicate::eq(RtpsReaderProxyAttributesImpl::new(
                Guid::new(
                    GuidPrefix([5; 12]),
                    ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
                ),
                ENTITYID_UNKNOWN,
                &[],
                &[],
                false,
                true,
            )))
            .once()
            .return_const(());

        let mut mock_builtin_publications_reader = MockStatefulReader::new();
        mock_builtin_publications_reader
            .expect_matched_writer_add()
            .with(predicate::eq(RtpsWriterProxyImpl::new(
                Guid::new(
                    GuidPrefix([5; 12]),
                    ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
                ),
                &[],
                &[],
                None,
                ENTITYID_UNKNOWN,
            )))
            .once()
            .return_const(());

        let mut mock_builtin_subscriptions_writer = MockStatefulWriter::new();
        mock_builtin_subscriptions_writer
            .expect_matched_reader_add()
            .with(predicate::eq(RtpsReaderProxyAttributesImpl::new(
                Guid::new(
                    GuidPrefix([5; 12]),
                    ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
                ),
                ENTITYID_UNKNOWN,
                &[],
                &[],
                false,
                true,
            )))
            .once()
            .return_const(());

        let mut mock_builtin_subscriptions_reader = MockStatefulReader::new();
        mock_builtin_subscriptions_reader
            .expect_matched_writer_add()
            .with(predicate::eq(RtpsWriterProxyImpl::new(
                Guid::new(
                    GuidPrefix([5; 12]),
                    ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
                ),
                &[],
                &[],
                None,
                ENTITYID_UNKNOWN,
            )))
            .once()
            .return_const(());

        let mut mock_builtin_topics_writer = MockStatefulWriter::new();
        mock_builtin_topics_writer
            .expect_matched_reader_add()
            .with(predicate::eq(RtpsReaderProxyAttributesImpl::new(
                Guid::new(GuidPrefix([5; 12]), ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR),
                ENTITYID_UNKNOWN,
                &[],
                &[],
                false,
                true,
            )))
            .once()
            .return_const(());

        let mut mock_builtin_topics_reader = MockStatefulReader::new();
        mock_builtin_topics_reader
            .expect_matched_writer_add()
            .with(predicate::eq(RtpsWriterProxyImpl::new(
                Guid::new(GuidPrefix([5; 12]), ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER),
                &[],
                &[],
                None,
                ENTITYID_UNKNOWN,
            )))
            .once()
            .return_const(());

        _spdp_task_discovery(
            &RtpsShared::new(mock_spdp_data_reader),
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

    #[test]
    fn task_sedp_discovery_() {
        let mut mock_sedp_discovered_writer_data_reader = MockDdsDataReader::new();
        mock_sedp_discovered_writer_data_reader
            .expect_read()
            .returning(|_, _, _, _| {
                Ok(Samples {
                    samples: vec![SedpDiscoveredWriterData {
                        writer_proxy: RtpsWriterProxy {
                            remote_writer_guid: Guid::new(
                                GuidPrefix([1; 12]),
                                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
                            ),
                            unicast_locator_list: vec![],
                            multicast_locator_list: vec![],
                            data_max_size_serialized: None,
                            remote_group_entity_id: EntityId::new([0; 3], 0),
                        },
                        publication_builtin_topic_data: PublicationBuiltinTopicData {
                            key: BuiltInTopicKey { value: [1; 16] },
                            participant_key: BuiltInTopicKey { value: [1; 16] },
                            topic_name: "MyTopic".to_string(),
                            type_name: "MyType".to_string(),
                            durability: DurabilityQosPolicy::default(),
                            durability_service: DurabilityServiceQosPolicy::default(),
                            deadline: DeadlineQosPolicy::default(),
                            latency_budget: LatencyBudgetQosPolicy::default(),
                            liveliness: LivelinessQosPolicy::default(),
                            reliability: ReliabilityQosPolicy {
                                kind: ReliabilityQosPolicyKind::BestEffortReliabilityQos,
                                max_blocking_time: Duration::new(3, 0),
                            },
                            lifespan: LifespanQosPolicy::default(),
                            user_data: UserDataQosPolicy::default(),
                            ownership: OwnershipQosPolicy::default(),
                            ownership_strength: OwnershipStrengthQosPolicy::default(),
                            destination_order: DestinationOrderQosPolicy::default(),
                            presentation: PresentationQosPolicy::default(),
                            partition: PartitionQosPolicy::default(),
                            topic_data: TopicDataQosPolicy::default(),
                            group_data: GroupDataQosPolicy::default(),
                        },
                    }],
                })
            });

        let subscriber = SubscriberAttributes::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(Guid::new(
                GuidPrefix([0; 12]),
                EntityId::new([0, 0, 0], BUILT_IN_READER_GROUP),
            )),
            RtpsWeak::new(),
        );
        let subscriber_list = vec![RtpsShared::new(subscriber)];

        _task_sedp_discovery(
            &RtpsShared::new(mock_sedp_discovered_writer_data_reader),
            &RtpsShared::new(subscriber_list),
        );

        //todo: Add readers and chack that thet got configured with appropriate proxies as
        // the returned from read() from the MockReader
    }
}
