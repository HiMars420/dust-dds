use dds_api::{
    builtin_topics::SubscriptionBuiltinTopicData,
    dcps_psm::{
        BuiltInTopicKey, Duration, InstanceHandle, InstanceStateMask, SampleLostStatus,
        SampleStateMask, StatusMask, ViewStateMask,
    },
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::{DataReaderQos, SubscriberQos, TopicQos},
        qos_policy::{
            DeadlineQosPolicy, DestinationOrderQosPolicy, DurabilityQosPolicy, GroupDataQosPolicy,
            LatencyBudgetQosPolicy, LivelinessQosPolicy, OwnershipQosPolicy, PartitionQosPolicy,
            PresentationQosPolicy, ReliabilityQosPolicy, ReliabilityQosPolicyKind,
            TimeBasedFilterQosPolicy, TopicDataQosPolicy, UserDataQosPolicy,
        },
    },
    return_type::{DdsError, DdsResult},
    subscription::{
        data_reader::{AnyDataReader, DataReaderGetTopicDescription},
        subscriber::{Subscriber, SubscriberDataReaderFactory, SubscriberGetParticipant},
        subscriber_listener::SubscriberListener,
    },
    topic::topic_description::TopicDescription,
};
use rtps_pim::{
    behavior::{
        reader::{
            stateful_reader::{RtpsStatefulReaderConstructor, RtpsStatefulReaderOperations},
            writer_proxy::{RtpsWriterProxyAttributes, RtpsWriterProxyConstructor},
        },
        stateful_reader_behavior::{
            RtpsStatefulReaderReceiveDataSubmessage, RtpsStatefulReaderReceiveHeartbeatSubmessage,
            RtpsStatefulReaderSendSubmessages,
        },
        stateless_reader_behavior::RtpsStatelessReaderReceiveDataSubmessage,
        writer::stateful_writer::RtpsStatefulWriterAttributes,
    },
    messages::{
        submessage_elements::Parameter,
        submessages::{DataSubmessage, HeartbeatSubmessage},
    },
    structure::{
        entity::RtpsEntityAttributes,
        participant::RtpsParticipantAttributes,
        types::{
            EntityId, Guid, GuidPrefix, ReliabilityKind, SequenceNumber, TopicKind,
            USER_DEFINED_WRITER_NO_KEY, USER_DEFINED_WRITER_WITH_KEY,
        },
    },
};

use crate::{
    data_representation_builtin_endpoints::{
        discovered_reader_data::{DiscoveredReaderData, RtpsReaderProxy},
        discovered_writer_data::DiscoveredWriterData,
    },
    dds_type::DdsType,
    utils::{
        discovery_traits::AddMatchedWriter,
        rtps_communication_traits::{
            ReceiveRtpsDataSubmessage, ReceiveRtpsHeartbeatSubmessage, SendRtpsMessage,
        },
        rtps_structure::RtpsStructure,
        shared_object::{DdsRwLock, DdsShared, DdsWeak},
        timer::ThreadTimer,
    },
};

use super::{
    data_reader_attributes::{DataReaderAttributes, DataReaderConstructor, RtpsReader},
    domain_participant_attributes::{DataReaderDiscovery, DomainParticipantAttributes},
    topic_attributes::TopicAttributes,
};

pub struct SubscriberAttributes<Rtps>
where
    Rtps: RtpsStructure,
{
    qos: SubscriberQos,
    rtps_group: Rtps::Group,
    data_reader_list: DdsRwLock<Vec<DdsShared<DataReaderAttributes<Rtps, ThreadTimer>>>>,
    user_defined_data_reader_counter: u8,
    default_data_reader_qos: DataReaderQos,
    parent_domain_participant: DdsWeak<DomainParticipantAttributes<Rtps>>,
}

pub trait SubscriberConstructor<Rtps>
where
    Rtps: RtpsStructure,
{
    fn new(
        qos: SubscriberQos,
        rtps_group: Rtps::Group,
        parent_domain_participant: DdsWeak<DomainParticipantAttributes<Rtps>>,
    ) -> Self;
}

impl<Rtps> SubscriberConstructor<Rtps> for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure,
{
    fn new(
        qos: SubscriberQos,
        rtps_group: <Rtps>::Group,
        parent_domain_participant: DdsWeak<DomainParticipantAttributes<Rtps>>,
    ) -> Self {
        DdsShared::new(SubscriberAttributes {
            qos,
            rtps_group,
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant,
        })
    }
}

pub trait SubscriberEmpty {
    fn is_empty(&self) -> bool;
}

impl<Rtps> SubscriberEmpty for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure,
{
    fn is_empty(&self) -> bool {
        self.data_reader_list.read_lock().is_empty()
    }
}
pub trait AddDataReader<Rtps>
where
    Rtps: RtpsStructure,
{
    fn add_data_reader(&self, reader: DdsShared<DataReaderAttributes<Rtps, ThreadTimer>>);
}

impl<Rtps> AddDataReader<Rtps> for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure,
{
    fn add_data_reader(&self, reader: DdsShared<DataReaderAttributes<Rtps, ThreadTimer>>) {
        self.data_reader_list.write_lock().push(reader);
    }
}

impl<Rtps, Foo> SubscriberDataReaderFactory<Foo> for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure,
    Rtps::StatefulWriter: for<'a> RtpsStatefulWriterAttributes<'a>,
    for<'a> <Rtps::StatefulWriter as RtpsStatefulWriterAttributes<'a>>::ReaderProxyListType:
        IntoIterator,
    Foo: DdsType,
{
    type TopicType = DdsShared<TopicAttributes<Rtps>>;
    type DataReaderType = DdsShared<DataReaderAttributes<Rtps, ThreadTimer>>;

    fn datareader_factory_create_datareader(
        &self,
        a_topic: &Self::TopicType,
        qos: Option<DataReaderQos>,
        a_listener: Option<<Self::DataReaderType as Entity>::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<Self::DataReaderType>
    where
        Self::DataReaderType: Entity,
    {
        // /////// Build the GUID
        let entity_id = {
            let entity_kind = match Foo::has_key() {
                true => USER_DEFINED_WRITER_WITH_KEY,
                false => USER_DEFINED_WRITER_NO_KEY,
            };

            EntityId::new(
                [
                    self.rtps_group.guid().entity_id().entity_key()[0],
                    self.user_defined_data_reader_counter,
                    0,
                ],
                entity_kind,
            )
        };

        let guid = Guid::new(self.rtps_group.guid().prefix(), entity_id);

        // /////// Create data reader
        let data_reader_shared = {
            let qos = qos.unwrap_or(self.default_data_reader_qos.clone());
            qos.is_consistent()?;

            let topic_kind = match Foo::has_key() {
                true => TopicKind::WithKey,
                false => TopicKind::NoKey,
            };

            let reliability_level = match qos.reliability.kind {
                ReliabilityQosPolicyKind::BestEffortReliabilityQos => ReliabilityKind::BestEffort,
                ReliabilityQosPolicyKind::ReliableReliabilityQos => ReliabilityKind::Reliable,
            };

            let domain_participant = self.parent_domain_participant.upgrade().ok();
            let rtps_reader = RtpsReader::Stateful(Rtps::StatefulReader::new(
                guid,
                topic_kind,
                reliability_level,
                domain_participant
                    .as_ref()
                    .map(|dp| dp.default_unicast_locator_list())
                    .unwrap_or(&[]),
                domain_participant
                    .as_ref()
                    .map(|dp| dp.default_multicast_locator_list())
                    .unwrap_or(&[]),
                rtps_pim::behavior::types::DURATION_ZERO,
                rtps_pim::behavior::types::DURATION_ZERO,
                false,
            ));

            let data_reader_shared: DdsShared<DataReaderAttributes<Rtps, ThreadTimer>> =
                DataReaderConstructor::new(
                    qos,
                    rtps_reader,
                    a_topic.clone(),
                    a_listener,
                    self.downgrade(),
                );

            self.data_reader_list
                .write_lock()
                .push(data_reader_shared.clone());

            data_reader_shared
        };

        // /////// Announce the data reader creation
        if let Ok(domain_participant) = self.parent_domain_participant.upgrade() {
            let sedp_discovered_reader_data = DiscoveredReaderData {
                reader_proxy: RtpsReaderProxy {
                    remote_reader_guid: guid,
                    remote_group_entity_id: entity_id,
                    unicast_locator_list: domain_participant
                        .default_unicast_locator_list()
                        .to_vec(),
                    multicast_locator_list: domain_participant
                        .default_multicast_locator_list()
                        .to_vec(),
                    expects_inline_qos: false,
                },

                subscription_builtin_topic_data: SubscriptionBuiltinTopicData {
                    key: BuiltInTopicKey { value: guid.into() },
                    participant_key: BuiltInTopicKey { value: [1; 16] },
                    topic_name: a_topic.get_name().unwrap().clone(),
                    type_name: Foo::type_name().to_string(),
                    durability: DurabilityQosPolicy::default(),
                    deadline: DeadlineQosPolicy::default(),
                    latency_budget: LatencyBudgetQosPolicy::default(),
                    liveliness: LivelinessQosPolicy::default(),
                    reliability: ReliabilityQosPolicy {
                        kind: ReliabilityQosPolicyKind::BestEffortReliabilityQos,
                        max_blocking_time: Duration::new(3, 0),
                    },
                    ownership: OwnershipQosPolicy::default(),
                    destination_order: DestinationOrderQosPolicy::default(),
                    user_data: UserDataQosPolicy::default(),
                    time_based_filter: TimeBasedFilterQosPolicy::default(),
                    presentation: PresentationQosPolicy::default(),
                    partition: PartitionQosPolicy::default(),
                    topic_data: TopicDataQosPolicy::default(),
                    group_data: GroupDataQosPolicy::default(),
                },
            };
            domain_participant.add_created_data_reader(&sedp_discovered_reader_data);
        }

        Ok(data_reader_shared)
    }

    fn datareader_factory_delete_datareader(
        &self,
        a_datareader: &Self::DataReaderType,
    ) -> DdsResult<()> {
        let data_reader_list = &mut self.data_reader_list.write_lock();
        let data_reader_list_position = data_reader_list
            .iter()
            .position(|x| x == a_datareader)
            .ok_or(DdsError::PreconditionNotMet(
                "Data reader can only be deleted from its parent subscriber".to_string(),
            ))?;
        data_reader_list.remove(data_reader_list_position);

        Ok(())
    }

    fn datareader_factory_lookup_datareader(
        &self,
        topic: &Self::TopicType,
    ) -> DdsResult<Self::DataReaderType> {
        let data_reader_list = &self.data_reader_list.write_lock();

        data_reader_list
            .iter()
            .find_map(|data_reader_shared| {
                let data_reader_topic = data_reader_shared
                    .data_reader_get_topicdescription()
                    .unwrap();

                if data_reader_topic.get_name().ok()? == topic.get_name().ok()?
                    && data_reader_topic.get_type_name().ok()? == Foo::type_name()
                {
                    Some(data_reader_shared.clone())
                } else {
                    None
                }
            })
            .ok_or(DdsError::PreconditionNotMet("Not found".to_string()))
    }
}

impl<Rtps> Subscriber for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure,
{
    fn begin_access(&self) -> DdsResult<()> {
        todo!()
    }

    fn end_access(&self) -> DdsResult<()> {
        todo!()
    }

    fn get_datareaders(
        &self,
        _readers: &mut [&mut dyn AnyDataReader],
        _sample_states: SampleStateMask,
        _view_states: ViewStateMask,
        _instance_states: InstanceStateMask,
    ) -> DdsResult<()> {
        todo!()
    }

    fn notify_datareaders(&self) -> DdsResult<()> {
        todo!()
    }

    fn get_sample_lost_status(&self, _status: &mut SampleLostStatus) -> DdsResult<()> {
        todo!()
    }

    fn delete_contained_entities(&self) -> DdsResult<()> {
        todo!()
    }

    fn set_default_datareader_qos(&self, _qos: Option<DataReaderQos>) -> DdsResult<()> {
        todo!()
    }

    fn get_default_datareader_qos(&self) -> DdsResult<DataReaderQos> {
        todo!()
    }

    fn copy_from_topic_qos(
        &self,
        _a_datareader_qos: &mut DataReaderQos,
        _a_topic_qos: &TopicQos,
    ) -> DdsResult<()> {
        todo!()
    }
}

impl<Rtps> SubscriberGetParticipant for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure,
{
    type DomainParticipant = DdsWeak<DomainParticipantAttributes<Rtps>>;

    fn subscriber_get_participant(&self) -> DdsResult<Self::DomainParticipant> {
        Ok(self.parent_domain_participant.clone())
    }
}

impl<Rtps> Entity for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure,
{
    type Qos = SubscriberQos;
    type Listener = Box<dyn SubscriberListener>;

    fn set_qos(&self, _qos: Option<Self::Qos>) -> DdsResult<()> {
        todo!()
    }

    fn get_qos(&self) -> DdsResult<Self::Qos> {
        Ok(self.qos.clone())
    }

    fn set_listener(
        &self,
        _a_listener: Option<Self::Listener>,
        _mask: StatusMask,
    ) -> DdsResult<()> {
        todo!()
    }

    fn get_listener(&self) -> DdsResult<Option<Self::Listener>> {
        todo!()
    }

    fn get_statuscondition(&self) -> DdsResult<StatusCondition> {
        todo!()
    }

    fn get_status_changes(&self) -> DdsResult<StatusMask> {
        todo!()
    }

    fn enable(&self) -> DdsResult<()> {
        todo!()
    }

    fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        todo!()
    }
}

impl<Rtps> AddMatchedWriter for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure,
    Rtps::StatefulReader: RtpsStatefulReaderOperations,
    <Rtps::StatefulReader as RtpsStatefulReaderOperations>::WriterProxyType:
        RtpsWriterProxyConstructor,
{
    fn add_matched_writer(&self, discovered_writer_data: &DiscoveredWriterData) {
        for data_reader in self.data_reader_list.read_lock().iter() {
            data_reader.add_matched_writer(&discovered_writer_data)
        }
    }
}

impl<Rtps> ReceiveRtpsDataSubmessage for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure + 'static,
    Rtps::Group: Send + Sync,
    Rtps::Participant: Send + Sync,
    Rtps::StatelessWriter: Send + Sync,
    Rtps::StatefulWriter: Send + Sync,
    Rtps::StatelessReader: for<'a> RtpsStatelessReaderReceiveDataSubmessage<Vec<Parameter<'a>>, &'a [u8]>
        + Send
        + Sync,
    Rtps::StatefulReader:
        for<'a> RtpsStatefulReaderReceiveDataSubmessage<Vec<Parameter<'a>>, &'a [u8]> + Send + Sync,
    Rtps::HistoryCache: Send + Sync,
    Rtps::CacheChange: Send + Sync,
{
    fn on_data_submessage_received(
        &self,
        data_submessage: &DataSubmessage<Vec<Parameter>, &[u8]>,
        source_guid_prefix: GuidPrefix,
    ) {
        for data_reader in self.data_reader_list.read_lock().iter() {
            data_reader.on_data_submessage_received(data_submessage, source_guid_prefix)
        }
    }
}

impl<Rtps> ReceiveRtpsHeartbeatSubmessage for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure,
    Rtps::StatefulReader: RtpsStatefulReaderReceiveHeartbeatSubmessage,
{
    fn on_heartbeat_submessage_received(
        &self,
        heartbeat_submessage: &HeartbeatSubmessage,
        source_guid_prefix: GuidPrefix,
    ) {
        for data_reader in self.data_reader_list.read_lock().iter() {
            data_reader.on_heartbeat_submessage_received(heartbeat_submessage, source_guid_prefix)
        }
    }
}

impl<Rtps> SendRtpsMessage for DdsShared<SubscriberAttributes<Rtps>>
where
    Rtps: RtpsStructure,
    Rtps::StatefulReader: RtpsEntityAttributes
        + RtpsStatefulReaderSendSubmessages<Vec<SequenceNumber>>,
    <Rtps::StatefulReader as RtpsStatefulReaderSendSubmessages<Vec<SequenceNumber>>>::WriterProxyType:
        RtpsWriterProxyAttributes,
{
    fn send_message(
        &self,
        transport: &mut impl for<'a> rtps_pim::transport::TransportWrite<
            Vec<
                rtps_pim::messages::overall_structure::RtpsSubmessageType<
                    Vec<rtps_pim::structure::types::SequenceNumber>,
                    Vec<Parameter<'a>>,
                    &'a [u8],
                    Vec<rtps_pim::structure::types::Locator>,
                    Vec<rtps_pim::messages::types::FragmentNumber>,
                >,
            >,
        >,
    ) {
        for data_reader in self.data_reader_list.read_lock().iter() {
            data_reader.send_message(transport);
        }
    }
}

#[cfg(test)]
mod tests {
    use dds_api::return_type::DdsError;
    use rtps_pim::structure::types::{EntityId, Guid, GuidPrefix};

    use crate::{
        dds_type::{DdsDeserialize, DdsType},
        test_utils::{mock_rtps::MockRtps, mock_rtps_group::MockRtpsGroup},
    };

    use super::*;

    macro_rules! make_empty_dds_type {
        ($type_name:ident) => {
            struct $type_name {}

            impl<'de> DdsDeserialize<'de> for $type_name {
                fn deserialize(_buf: &mut &'de [u8]) -> DdsResult<Self> {
                    Ok($type_name {})
                }
            }

            impl DdsType for $type_name {
                fn type_name() -> &'static str {
                    stringify!($type_name)
                }

                fn has_key() -> bool {
                    false
                }
            }
        };
    }

    make_empty_dds_type!(Foo);

    #[test]
    fn create_datareader() {
        let mut subscriber_attributes = SubscriberAttributes::<MockRtps> {
            qos: SubscriberQos::default(),
            rtps_group: MockRtpsGroup::new(),
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant: DdsWeak::new(),
        };
        subscriber_attributes
            .rtps_group
            .expect_guid()
            .return_const(Guid::new(GuidPrefix([1; 12]), EntityId::new([1; 3], 1)));

        let subscriber = DdsShared::new(subscriber_attributes);

        let topic = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic",
            DdsWeak::new(),
        ));

        let data_reader = subscriber.create_datareader::<Foo>(&topic, None, None, 0);

        assert!(data_reader.is_ok());
    }

    #[test]
    fn datareader_factory_delete_datareader() {
        let mut subscriber_attributes = SubscriberAttributes::<MockRtps> {
            qos: SubscriberQos::default(),
            rtps_group: MockRtpsGroup::new(),
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant: DdsWeak::new(),
        };
        subscriber_attributes
            .rtps_group
            .expect_guid()
            .return_const(Guid::new(GuidPrefix([1; 12]), EntityId::new([1; 3], 1)));
        let subscriber = DdsShared::new(subscriber_attributes);

        let topic = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic",
            DdsWeak::new(),
        ));

        let data_reader = subscriber
            .create_datareader::<Foo>(&topic, None, None, 0)
            .unwrap();

        assert_eq!(1, subscriber.data_reader_list.read_lock().len());

        subscriber.delete_datareader::<Foo>(&data_reader).unwrap();
        assert_eq!(0, subscriber.data_reader_list.read_lock().len());
    }

    #[test]
    fn datareader_factory_delete_datareader_from_other_subscriber() {
        let mut subscriber_attributes = SubscriberAttributes::<MockRtps> {
            qos: SubscriberQos::default(),
            rtps_group: MockRtpsGroup::new(),
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant: DdsWeak::new(),
        };
        subscriber_attributes
            .rtps_group
            .expect_guid()
            .return_const(Guid::new(GuidPrefix([1; 12]), EntityId::new([1; 3], 1)));
        let subscriber = DdsShared::new(subscriber_attributes);

        let mut subscriber2_attributes = SubscriberAttributes::<MockRtps> {
            qos: SubscriberQos::default(),
            rtps_group: MockRtpsGroup::new(),
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant: DdsWeak::new(),
        };
        subscriber2_attributes
            .rtps_group
            .expect_guid()
            .return_const(Guid::new(GuidPrefix([1; 12]), EntityId::new([1; 3], 1)));
        let subscriber2 = DdsShared::new(subscriber2_attributes);

        let topic = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic",
            DdsWeak::new(),
        ));

        let data_reader = subscriber
            .create_datareader::<Foo>(&topic, None, None, 0)
            .unwrap();

        assert_eq!(1, subscriber.data_reader_list.read_lock().len());
        assert_eq!(0, subscriber2.data_reader_list.read_lock().len());

        assert!(matches!(
            subscriber2.delete_datareader::<Foo>(&data_reader),
            Err(DdsError::PreconditionNotMet(_))
        ));
    }

    #[test]
    fn datareader_factory_lookup_datareader_when_empty() {
        let mut subscriber_attributes = SubscriberAttributes::<MockRtps> {
            qos: SubscriberQos::default(),
            rtps_group: MockRtpsGroup::new(),
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant: DdsWeak::new(),
        };
        subscriber_attributes
            .rtps_group
            .expect_guid()
            .return_const(Guid::new(GuidPrefix([1; 12]), EntityId::new([1; 3], 1)));
        let subscriber = DdsShared::new(subscriber_attributes);

        let topic = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic",
            DdsWeak::new(),
        ));

        assert!(subscriber.lookup_datareader::<Foo>(&topic).is_err());
    }

    #[test]
    fn datareader_factory_lookup_datareader_when_one_datareader() {
        let mut subscriber_attributes = SubscriberAttributes::<MockRtps> {
            qos: SubscriberQos::default(),
            rtps_group: MockRtpsGroup::new(),
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant: DdsWeak::new(),
        };
        subscriber_attributes
            .rtps_group
            .expect_guid()
            .return_const(Guid::new(GuidPrefix([1; 12]), EntityId::new([1; 3], 1)));
        let subscriber = DdsShared::new(subscriber_attributes);

        let topic = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic",
            DdsWeak::new(),
        ));

        let data_reader = subscriber
            .create_datareader::<Foo>(&topic, None, None, 0)
            .unwrap();

        assert!(subscriber.lookup_datareader::<Foo>(&topic).unwrap() == data_reader);
    }

    make_empty_dds_type!(Bar);

    #[test]
    fn datareader_factory_lookup_datareader_when_one_datareader_with_wrong_type() {
        let mut subscriber_attributes = SubscriberAttributes::<MockRtps> {
            qos: SubscriberQos::default(),
            rtps_group: MockRtpsGroup::new(),
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant: DdsWeak::new(),
        };
        subscriber_attributes
            .rtps_group
            .expect_guid()
            .return_const(Guid::new(GuidPrefix([1; 12]), EntityId::new([1; 3], 1)));
        let subscriber = DdsShared::new(subscriber_attributes);

        let topic_foo = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic",
            DdsWeak::new(),
        ));

        let topic_bar = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Bar::type_name(),
            "topic",
            DdsWeak::new(),
        ));

        subscriber
            .create_datareader::<Bar>(&topic_bar, None, None, 0)
            .unwrap();

        assert!(subscriber.lookup_datareader::<Foo>(&topic_foo).is_err());
    }

    #[test]
    fn datareader_factory_lookup_datareader_when_one_datareader_with_wrong_topic() {
        let mut subscriber_attributes = SubscriberAttributes::<MockRtps> {
            qos: SubscriberQos::default(),
            rtps_group: MockRtpsGroup::new(),
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant: DdsWeak::new(),
        };
        subscriber_attributes
            .rtps_group
            .expect_guid()
            .return_const(Guid::new(GuidPrefix([1; 12]), EntityId::new([1; 3], 1)));
        let subscriber = DdsShared::new(subscriber_attributes);

        let topic1 = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic1",
            DdsWeak::new(),
        ));

        let topic2 = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic2",
            DdsWeak::new(),
        ));

        subscriber
            .create_datareader::<Foo>(&topic2, None, None, 0)
            .unwrap();

        assert!(subscriber.lookup_datareader::<Foo>(&topic1).is_err());
    }

    #[test]
    fn datareader_factory_lookup_datareader_with_two_types() {
        let mut subscriber_attributes = SubscriberAttributes::<MockRtps> {
            qos: SubscriberQos::default(),
            rtps_group: MockRtpsGroup::new(),
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant: DdsWeak::new(),
        };
        subscriber_attributes
            .rtps_group
            .expect_guid()
            .return_const(Guid::new(GuidPrefix([1; 12]), EntityId::new([1; 3], 1)));
        let subscriber = DdsShared::new(subscriber_attributes);

        let topic_foo = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic",
            DdsWeak::new(),
        ));

        let topic_bar = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Bar::type_name(),
            "topic",
            DdsWeak::new(),
        ));

        let data_reader_foo = subscriber
            .create_datareader::<Foo>(&topic_foo, None, None, 0)
            .unwrap();
        let data_reader_bar = subscriber
            .create_datareader::<Bar>(&topic_bar, None, None, 0)
            .unwrap();

        assert!(subscriber.lookup_datareader::<Foo>(&topic_foo).unwrap() == data_reader_foo);

        assert!(subscriber.lookup_datareader::<Bar>(&topic_bar).unwrap() == data_reader_bar);
    }

    #[test]
    fn datareader_factory_lookup_datareader_with_two_topics() {
        let mut subscriber_attributes = SubscriberAttributes::<MockRtps> {
            qos: SubscriberQos::default(),
            rtps_group: MockRtpsGroup::new(),
            data_reader_list: DdsRwLock::new(Vec::new()),
            user_defined_data_reader_counter: 0,
            default_data_reader_qos: DataReaderQos::default(),
            parent_domain_participant: DdsWeak::new(),
        };
        subscriber_attributes
            .rtps_group
            .expect_guid()
            .return_const(Guid::new(GuidPrefix([1; 12]), EntityId::new([1; 3], 1)));
        let subscriber = DdsShared::new(subscriber_attributes);

        let topic1 = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic1",
            DdsWeak::new(),
        ));

        let topic2 = DdsShared::new(TopicAttributes::new(
            TopicQos::default(),
            Foo::type_name(),
            "topic2",
            DdsWeak::new(),
        ));

        let data_reader1 = subscriber
            .create_datareader::<Foo>(&topic1, None, None, 0)
            .unwrap();
        let data_reader2 = subscriber
            .create_datareader::<Foo>(&topic2, None, None, 0)
            .unwrap();

        assert!(subscriber.lookup_datareader::<Foo>(&topic1).unwrap() == data_reader1);

        assert!(subscriber.lookup_datareader::<Foo>(&topic2).unwrap() == data_reader2);
    }
}
