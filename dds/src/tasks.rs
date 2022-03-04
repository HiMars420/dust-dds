use std::{
    ops::Deref,
    sync::{
        atomic::{self, AtomicBool},
        mpsc::{Receiver, SyncSender},
        Arc,
    },
};

use async_std::prelude::StreamExt;
use rust_dds_api::{
    dcps_psm::{PublicationMatchedStatus, SubscriptionMatchedStatus},
    subscription::data_reader::DataReader,
};
use rust_dds_rtps_implementation::{
    data_representation_builtin_endpoints::{
        sedp_discovered_reader_data::SedpDiscoveredReaderData,
        sedp_discovered_writer_data::SedpDiscoveredWriterData,
        spdp_discovered_participant_data::SpdpDiscoveredParticipantData,
    },
    dds_impl::{
        data_reader_proxy::{RtpsReader, Samples},
        data_writer_proxy::RtpsWriter,
        publisher_proxy::PublisherAttributes,
        subscriber_proxy::SubscriberAttributes,
    },
    rtps_impl::{
        rtps_reader_proxy_impl::RtpsReaderProxyAttributesImpl,
        rtps_writer_proxy_impl::RtpsWriterProxyImpl,
    },
    utils::shared_object::RtpsShared,
};
use rust_rtps_pim::{
    behavior::{
        reader::{
            stateful_reader::RtpsStatefulReaderOperations, writer_proxy::RtpsWriterProxyConstructor,
        },
        writer::{
            reader_proxy::RtpsReaderProxyConstructor, stateful_writer::RtpsStatefulWriterOperations,
        },
    },
    discovery::participant_discovery::ParticipantDiscovery,
};

use crate::domain_participant_factory::RtpsStructureImpl;

pub struct Executor {
    pub receiver: Receiver<EnabledPeriodicTask>,
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
                        println!("Task not enabled: {}", enabled_periodic_task.name);
                    }
                    interval.next().await;
                }
            });
        }
    }
}

#[derive(Clone)]
pub struct Spawner {
    pub task_sender: SyncSender<EnabledPeriodicTask>,
    pub enabled: Arc<AtomicBool>,
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

    pub fn _disable_tasks(&self) {
        self.enabled.store(false, atomic::Ordering::SeqCst);
    }
}

pub struct EnabledPeriodicTask {
    pub name: &'static str,
    pub task: Box<dyn FnMut() -> () + Send + Sync>,
    pub period: std::time::Duration,
    pub enabled: Arc<AtomicBool>,
}

pub fn task_spdp_discovery<T>(
    spdp_builtin_participant_data_reader: &mut impl DataReader<
        SpdpDiscoveredParticipantData,
        Samples = T,
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
    if let Ok(samples) = spdp_builtin_participant_data_reader.take(1, &[], &[], &[]) {
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

pub fn task_sedp_writer_discovery(
    sedp_builtin_publications_data_reader: &mut impl DataReader<
        SedpDiscoveredWriterData,
        Samples = Samples<SedpDiscoveredWriterData>,
    >,
    subscriber_list: &Vec<RtpsShared<SubscriberAttributes<RtpsStructureImpl>>>,
) {
    if subscriber_list.is_empty() {
        return;
    }

    if let Ok(samples) = sedp_builtin_publications_data_reader.take(1, &[], &[], &[]) {
        if let Some(sample) = samples.into_iter().next() {
            let topic_name = &sample.publication_builtin_topic_data.topic_name;
            let type_name = &sample.publication_builtin_topic_data.type_name;
            for subscriber in subscriber_list {
                let subscriber_lock = subscriber.read_lock();
                for data_reader in subscriber_lock.data_reader_list.iter() {
                    let mut data_reader_lock = data_reader.write_lock();
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
                                rtps_stateful_reader.matched_writer_add(writer_proxy);
                                let count = rtps_stateful_reader.matched_writers.len() as i32;
                                data_reader_lock.listener.as_ref().map(|l| {
                                    l.on_subscription_matched(SubscriptionMatchedStatus {
                                        total_count: count,         // ?
                                        total_count_change: 1,      // ?
                                        last_publication_handle: 0, // ????
                                        current_count: count,
                                        current_count_change: 1,
                                    })
                                });
                            }
                        };
                    }
                }
            }
        }
    }
}

pub fn task_sedp_reader_discovery(
    sedp_builtin_subscriptions_data_reader: &mut impl DataReader<
        SedpDiscoveredReaderData,
        Samples = Samples<SedpDiscoveredReaderData>,
    >,
    publisher_list: &Vec<RtpsShared<PublisherAttributes<RtpsStructureImpl>>>,
) {
    if publisher_list.is_empty() {
        return;
    }

    if let Ok(samples) = sedp_builtin_subscriptions_data_reader.take(1, &[], &[], &[]) {
        if let Some(sample) = samples.into_iter().next() {
            let topic_name = &sample.subscription_builtin_topic_data.topic_name;
            let type_name = &sample.subscription_builtin_topic_data.type_name;
            for publisher in publisher_list {
                let publisher_lock = publisher.read_lock();
                for data_writer in publisher_lock.data_writer_list.iter() {
                    let mut data_writer_lock = data_writer.write_lock();
                    let writer_topic_name = &data_writer_lock.topic.read_lock().topic_name.clone();
                    let writer_type_name = data_writer_lock.topic.read_lock().type_name;
                    if topic_name == writer_topic_name && type_name == writer_type_name {
                        let reader_proxy = RtpsReaderProxyAttributesImpl::new(
                            sample.reader_proxy.remote_reader_guid,
                            sample.reader_proxy.remote_group_entity_id,
                            sample.reader_proxy.unicast_locator_list.as_ref(),
                            sample.reader_proxy.multicast_locator_list.as_ref(),
                            sample.reader_proxy.expects_inline_qos,
                            true, // ???
                        );
                        match &mut data_writer_lock.rtps_writer {
                            RtpsWriter::Stateless(_) => (),
                            RtpsWriter::Stateful(rtps_stateful_writer) => {
                                rtps_stateful_writer.matched_reader_add(reader_proxy);
                                let count = rtps_stateful_writer.matched_readers.len() as i32;
                                data_writer_lock.listener.as_ref().map(|l| {
                                    l.on_publication_matched(PublicationMatchedStatus {
                                        total_count: count,          // ?
                                        total_count_change: 1,       // ?
                                        last_subscription_handle: 0, // ????
                                        current_count: count,
                                        current_count_change: 1,
                                    })
                                });
                            }
                        };
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mockall::{mock, predicate};
    use rust_dds_api::{
        builtin_topics::{
            ParticipantBuiltinTopicData, PublicationBuiltinTopicData, SubscriptionBuiltinTopicData,
        },
        dcps_psm::{
            BuiltInTopicKey, DomainId, Duration, InstanceHandle, InstanceStateKind,
            LivelinessChangedStatus, RequestedDeadlineMissedStatus, RequestedIncompatibleQosStatus,
            SampleLostStatus, SampleRejectedStatus, SampleStateKind, SubscriptionMatchedStatus,
            ViewStateKind,
        },
        infrastructure::{
            qos::{DataWriterQos, DomainParticipantQos, PublisherQos, SubscriberQos, TopicQos},
            qos_policy::{
                DeadlineQosPolicy, DestinationOrderQosPolicy, DurabilityQosPolicy,
                DurabilityServiceQosPolicy, GroupDataQosPolicy, LatencyBudgetQosPolicy,
                LifespanQosPolicy, LivelinessQosPolicy, OwnershipQosPolicy,
                OwnershipStrengthQosPolicy, PartitionQosPolicy, PresentationQosPolicy,
                ReliabilityQosPolicy, ReliabilityQosPolicyKind, TimeBasedFilterQosPolicy,
                TopicDataQosPolicy, UserDataQosPolicy,
            },
            read_condition::ReadCondition,
            sample_info::SampleInfo,
        },
        publication::publisher::PublisherDataWriterFactory,
        return_type::DDSResult,
        subscription::{
            data_reader::DataReader, query_condition::QueryCondition,
            subscriber::SubscriberDataReaderFactory,
        },
    };
    use rust_dds_rtps_implementation::{
        data_representation_builtin_endpoints::{
            sedp_discovered_reader_data::{
                RtpsReaderProxy, SedpDiscoveredReaderData, DCPS_SUBSCRIPTION,
            },
            sedp_discovered_writer_data::{
                RtpsWriterProxy, SedpDiscoveredWriterData, DCPS_PUBLICATION,
            },
            spdp_discovered_participant_data::{ParticipantProxy, SpdpDiscoveredParticipantData},
        },
        dds_impl::{
            data_reader_proxy::Samples,
            data_writer_proxy::{DataWriterAttributes, RtpsWriter},
            domain_participant_proxy::{DomainParticipantAttributes, DomainParticipantProxy},
            publisher_proxy::{PublisherAttributes, PublisherProxy},
            subscriber_proxy::{SubscriberAttributes, SubscriberProxy},
            topic_proxy::{TopicAttributes, TopicProxy},
        },
        dds_type::{DdsDeserialize, DdsSerialize, DdsType},
        rtps_impl::{
            rtps_group_impl::RtpsGroupImpl, rtps_reader_proxy_impl::RtpsReaderProxyAttributesImpl,
            rtps_writer_proxy_impl::RtpsWriterProxyImpl,
        },
        utils::shared_object::{RtpsShared, RtpsWeak},
    };
    use rust_rtps_pim::{
        behavior::{
            reader::{
                stateful_reader::RtpsStatefulReaderOperations,
                writer_proxy::RtpsWriterProxyConstructor,
            },
            writer::{
                reader_proxy::RtpsReaderProxyConstructor,
                stateful_writer::RtpsStatefulWriterOperations,
            },
        },
        discovery::{
            sedp::builtin_endpoints::{
                SedpBuiltinPublicationsWriter, SedpBuiltinSubscriptionsWriter,
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
                ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
            },
            types::{BuiltinEndpointQos, BuiltinEndpointSet},
        },
        messages::types::Count,
        structure::{
            group::RtpsGroupConstructor,
            types::{
                EntityId, Guid, GuidPrefix, BUILT_IN_READER_GROUP, ENTITYID_UNKNOWN, GUID_UNKNOWN,
                PROTOCOLVERSION, USER_DEFINED_READER_WITH_KEY, USER_DEFINED_WRITER_WITH_KEY,
                VENDOR_ID_S2E,
            },
        },
    };

    use crate::{domain_participant_factory::RtpsStructureImpl, tasks::task_sedp_reader_discovery};

    use super::{task_sedp_writer_discovery, task_spdp_discovery};

    mock! {
        DdsDataReader<Foo: 'static>{}

        impl<Foo> DataReader<Foo> for DdsDataReader<Foo>{
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
                &mut self,
                max_samples: i32,
                sample_states: &[SampleStateKind],
                view_states: &[ViewStateKind],
                instance_states: &[InstanceStateKind],
            ) -> DDSResult<Samples<Foo>>;

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

    mock! {
        StatefulReader {
            fn matched_writer_add_(&mut self, a_writer_proxy: RtpsWriterProxyImpl);
        }
    }

    impl RtpsStatefulReaderOperations for MockStatefulReader {
        type WriterProxyType = RtpsWriterProxyImpl;

        fn matched_writer_add(&mut self, a_writer_proxy: Self::WriterProxyType) {
            self.matched_writer_add_(a_writer_proxy)
        }

        fn matched_writer_remove<F>(&mut self, _f: F)
        where
            F: FnMut(&Self::WriterProxyType) -> bool,
        {
            todo!()
        }

        fn matched_writer_lookup(&self, _a_writer_guid: Guid) -> Option<&Self::WriterProxyType> {
            todo!()
        }
    }

    mock! {
        StatefulWriter {
            fn matched_reader_add_(&mut self, a_reader_proxy: RtpsReaderProxyAttributesImpl);
        }
    }

    impl RtpsStatefulWriterOperations for MockStatefulWriter {
        type ReaderProxyType = RtpsReaderProxyAttributesImpl;

        fn matched_reader_add(&mut self, a_reader_proxy: Self::ReaderProxyType) {
            self.matched_reader_add_(a_reader_proxy)
        }

        fn matched_reader_remove<F>(&mut self, _f: F)
        where
            F: FnMut(&Self::ReaderProxyType) -> bool,
        {
            todo!()
        }

        fn matched_reader_lookup(&self, _a_reader_guid: Guid) -> Option<&Self::ReaderProxyType> {
            todo!()
        }

        fn is_acked_by_all(&self) -> bool {
            todo!()
        }
    }

    fn make_participant() -> RtpsShared<DomainParticipantAttributes<RtpsStructureImpl>> {
        let domain_participant = RtpsShared::new(DomainParticipantAttributes::new(
            GuidPrefix([0; 12]),
            DomainId::default(),
            0,
            "".to_string(),
            DomainParticipantQos::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        ));

        domain_participant.write_lock().builtin_publisher =
            Some(RtpsShared::new(PublisherAttributes::new(
                PublisherQos::default(),
                RtpsGroupImpl::new(GUID_UNKNOWN),
                domain_participant.downgrade(),
            )));

        let sedp_topic_subscription = RtpsShared::new(TopicAttributes::new(
            TopicQos::default(),
            SedpDiscoveredReaderData::type_name(),
            DCPS_SUBSCRIPTION,
            RtpsWeak::new(),
        ));

        let sedp_topic_publication = RtpsShared::new(TopicAttributes::new(
            TopicQos::default(),
            SedpDiscoveredWriterData::type_name(),
            DCPS_PUBLICATION,
            RtpsWeak::new(),
        ));

        domain_participant
            .write_lock()
            .topic_list
            .push(sedp_topic_subscription.clone());

        domain_participant
            .write_lock()
            .topic_list
            .push(sedp_topic_publication.clone());

        let sedp_builtin_subscriptions_rtps_writer =
            SedpBuiltinSubscriptionsWriter::create(GuidPrefix([0; 12]), &[], &[]);
        let sedp_builtin_subscriptions_data_writer = RtpsShared::new(DataWriterAttributes::new(
            DataWriterQos::default(),
            RtpsWriter::Stateful(sedp_builtin_subscriptions_rtps_writer),
            sedp_topic_subscription.clone(),
            domain_participant
                .read_lock()
                .builtin_publisher
                .as_ref()
                .unwrap()
                .downgrade(),
        ));
        domain_participant
            .read_lock()
            .builtin_publisher
            .as_ref()
            .unwrap()
            .write_lock()
            .data_writer_list
            .push(sedp_builtin_subscriptions_data_writer.clone());

        let sedp_builtin_publications_rtps_writer =
            SedpBuiltinPublicationsWriter::create(GuidPrefix([0; 12]), &[], &[]);
        let sedp_builtin_publications_data_writer = RtpsShared::new(DataWriterAttributes::new(
            DataWriterQos::default(),
            RtpsWriter::Stateful(sedp_builtin_publications_rtps_writer),
            sedp_topic_publication.clone(),
            domain_participant
                .read_lock()
                .builtin_publisher
                .as_ref()
                .unwrap()
                .downgrade(),
        ));
        domain_participant
            .read_lock()
            .builtin_publisher
            .as_ref()
            .unwrap()
            .write_lock()
            .data_writer_list
            .push(sedp_builtin_publications_data_writer.clone());

        domain_participant
    }

    struct MyType;

    impl DdsType for MyType {
        fn type_name() -> &'static str {
            "MyType"
        }

        fn has_key() -> bool {
            false
        }
    }

    impl DdsSerialize for MyType {
        fn serialize<W: std::io::Write, E: rust_dds_rtps_implementation::dds_type::Endianness>(
            &self,
            _writer: W,
        ) -> DDSResult<()> {
            Ok(())
        }
    }

    impl<'de> DdsDeserialize<'de> for MyType {
        fn deserialize(_buf: &mut &'de [u8]) -> DDSResult<Self> {
            Ok(MyType {})
        }
    }

    #[test]
    fn discovery_task_all_sedp_endpoints() {
        let mut mock_spdp_data_reader = MockDdsDataReader::new();
        mock_spdp_data_reader.expect_take().returning(|_, _, _, _| {
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
            .expect_matched_reader_add_()
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
            .expect_matched_writer_add_()
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
            .expect_matched_reader_add_()
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
            .expect_matched_writer_add_()
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
            .expect_matched_reader_add_()
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
            .expect_matched_writer_add_()
            .with(predicate::eq(RtpsWriterProxyImpl::new(
                Guid::new(GuidPrefix([5; 12]), ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER),
                &[],
                &[],
                None,
                ENTITYID_UNKNOWN,
            )))
            .once()
            .return_const(());

        task_spdp_discovery(
            &mut mock_spdp_data_reader,
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
    fn task_sedp_writer_discovery_() {
        let topic = RtpsShared::new(TopicAttributes {
            _qos: TopicQos::default(),
            type_name: MyType::type_name(),
            topic_name: "MyTopic".to_string(),
            parent_participant: RtpsWeak::new(),
        });

        let mut mock_sedp_discovered_writer_data_reader = MockDdsDataReader::new();
        mock_sedp_discovered_writer_data_reader
            .expect_take()
            .returning(|_, _, _, _| {
                Ok(Samples {
                    samples: vec![SedpDiscoveredWriterData {
                        writer_proxy: RtpsWriterProxy {
                            remote_writer_guid: Guid::new(
                                GuidPrefix([1; 12]),
                                EntityId {
                                    entity_key: [1, 2, 3],
                                    entity_kind: USER_DEFINED_WRITER_WITH_KEY,
                                },
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
                            type_name: MyType::type_name().to_string(),
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

        let participant = make_participant();

        let subscriber = RtpsShared::new(SubscriberAttributes::new(
            SubscriberQos::default(),
            RtpsGroupImpl::new(Guid::new(
                GuidPrefix([0; 12]),
                EntityId::new([0, 0, 0], BUILT_IN_READER_GROUP),
            )),
            participant.downgrade(),
        ));
        let subscriber_proxy = SubscriberProxy::new(
            DomainParticipantProxy::new(RtpsWeak::new()),
            subscriber.downgrade(),
        );

        let reader = subscriber_proxy
            .datareader_factory_create_datareader(
                &TopicProxy::<MyType, _>::new(topic.downgrade()),
                None,
                None,
                0,
            )
            .unwrap();

        let subscriber_list = vec![subscriber];

        task_sedp_writer_discovery(
            &mut mock_sedp_discovered_writer_data_reader,
            &subscriber_list,
        );

        let reader_shared = reader.as_ref().upgrade().unwrap();
        let mut reader_lock = reader_shared.write_lock();
        let stateful_reader = reader_lock.rtps_reader.try_as_stateful_reader().unwrap();

        assert!(stateful_reader
            .matched_writer_lookup(Guid::new(
                GuidPrefix([1; 12]),
                EntityId {
                    entity_key: [1, 2, 3],
                    entity_kind: USER_DEFINED_WRITER_WITH_KEY
                },
            ))
            .is_some());
    }

    #[test]
    fn task_sedp_reader_discovery_() {
        let topic = RtpsShared::new(TopicAttributes {
            _qos: TopicQos::default(),
            type_name: MyType::type_name(),
            topic_name: "MyTopic".to_string(),
            parent_participant: RtpsWeak::new(),
        });

        let mut mock_sedp_builtin_subscription_reader = MockDdsDataReader::new();
        mock_sedp_builtin_subscription_reader
            .expect_take()
            .returning(|_, _, _, _| {
                Ok(Samples {
                    samples: vec![SedpDiscoveredReaderData {
                        reader_proxy: RtpsReaderProxy {
                            remote_reader_guid: Guid::new(
                                GuidPrefix([1; 12]),
                                EntityId {
                                    entity_key: [1, 2, 3],
                                    entity_kind: USER_DEFINED_READER_WITH_KEY,
                                },
                            ),
                            unicast_locator_list: vec![],
                            multicast_locator_list: vec![],
                            remote_group_entity_id: EntityId::new([0; 3], 0),
                            expects_inline_qos: false,
                        },
                        subscription_builtin_topic_data: SubscriptionBuiltinTopicData {
                            key: BuiltInTopicKey { value: [1; 16] },
                            participant_key: BuiltInTopicKey { value: [1; 16] },
                            topic_name: "MyTopic".to_string(),
                            type_name: MyType::type_name().to_string(),
                            durability: DurabilityQosPolicy::default(),
                            deadline: DeadlineQosPolicy::default(),
                            latency_budget: LatencyBudgetQosPolicy::default(),
                            liveliness: LivelinessQosPolicy::default(),
                            reliability: ReliabilityQosPolicy {
                                kind: ReliabilityQosPolicyKind::BestEffortReliabilityQos,
                                max_blocking_time: Duration::new(3, 0),
                            },
                            user_data: UserDataQosPolicy::default(),
                            ownership: OwnershipQosPolicy::default(),
                            destination_order: DestinationOrderQosPolicy::default(),
                            presentation: PresentationQosPolicy::default(),
                            partition: PartitionQosPolicy::default(),
                            topic_data: TopicDataQosPolicy::default(),
                            group_data: GroupDataQosPolicy::default(),
                            time_based_filter: TimeBasedFilterQosPolicy::default(),
                        },
                    }],
                })
            });

        let participant = make_participant();

        let publisher = RtpsShared::new(PublisherAttributes::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(Guid::new(
                GuidPrefix([0; 12]),
                EntityId::new([0, 0, 0], BUILT_IN_READER_GROUP),
            )),
            participant.downgrade(),
        ));
        let publisher_proxy = PublisherProxy::new(publisher.downgrade());

        let writer = publisher_proxy
            .datawriter_factory_create_datawriter(
                &TopicProxy::<MyType, _>::new(topic.downgrade()),
                None,
                None,
                0,
            )
            .unwrap();

        let publisher_list = vec![publisher];

        task_sedp_reader_discovery(&mut mock_sedp_builtin_subscription_reader, &publisher_list);

        let writer_shared = writer.as_ref().upgrade().unwrap();
        let mut writer_lock = writer_shared.write_lock();
        let stateful_writer = writer_lock.rtps_writer.try_as_stateful_writer().unwrap();

        assert!(stateful_writer
            .matched_reader_lookup(Guid::new(
                GuidPrefix([1; 12]),
                EntityId {
                    entity_key: [1, 2, 3],
                    entity_kind: USER_DEFINED_READER_WITH_KEY
                },
            ))
            .is_some());
    }
}
