use std::{cell::RefCell, collections::HashMap};

use crate::{
    data_representation_builtin_endpoints::discovered_writer_data::RtpsWriterProxy,
    data_representation_inline_qos::{
        parameter_id_values::PID_STATUS_INFO,
        types::{STATUS_INFO_DISPOSED_FLAG, STATUS_INFO_UNREGISTERED_FLAG},
    },
    rtps_impl::{
        rtps_history_cache_impl::{RtpsCacheChangeImpl, RtpsHistoryCacheImpl, RtpsParameter},
        rtps_stateful_writer_impl::{RtpsReaderProxyImpl, RtpsStatefulWriterImpl},
        rtps_stateless_writer_impl::{RtpsReaderLocatorAttributesImpl, RtpsStatelessWriterImpl},
        utils::clock::StdTimer,
    },
};
use dds_api::{
    builtin_topics::{PublicationBuiltinTopicData, SubscriptionBuiltinTopicData},
    dcps_psm::{
        BuiltInTopicKey, Duration, InstanceHandle, LivelinessLostStatus,
        OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus, PublicationMatchedStatus,
        QosPolicyCount, StatusMask, Time, HANDLE_NIL_NATIVE, LENGTH_UNLIMITED,
    },
    domain::domain_participant::DomainParticipant,
    infrastructure::{
        entity::{Entity, StatusCondition},
        qos::DataWriterQos,
        qos_policy::{
            DeadlineQosPolicy, DestinationOrderQosPolicy, DurabilityQosPolicy,
            DurabilityServiceQosPolicy, GroupDataQosPolicy, LatencyBudgetQosPolicy,
            LifespanQosPolicy, LivelinessQosPolicy, OwnershipQosPolicy, OwnershipStrengthQosPolicy,
            PartitionQosPolicy, PresentationQosPolicy, ReliabilityQosPolicy,
            ReliabilityQosPolicyKind, TopicDataQosPolicy, UserDataQosPolicy,
            DEADLINE_QOS_POLICY_ID, DESTINATIONORDER_QOS_POLICY_ID, DURABILITY_QOS_POLICY_ID,
            LATENCYBUDGET_QOS_POLICY_ID, LIVELINESS_QOS_POLICY_ID, OWNERSHIPSTRENGTH_QOS_POLICY_ID,
            PRESENTATION_QOS_POLICY_ID, RELIABILITY_QOS_POLICY_ID,
        },
    },
    publication::{
        data_writer::{DataWriter, DataWriterGetPublisher, DataWriterGetTopic, FooDataWriter},
        data_writer_listener::DataWriterListener,
        publisher::Publisher,
    },
    return_type::{DdsError, DdsResult},
    topic::topic_description::TopicDescription,
};

use rtps_pim::{
    behavior::{
        stateful_writer_behavior::{
            RtpsStatefulWriterReceiveAckNackSubmessage, RtpsStatefulWriterSendSubmessages,
        },
        stateless_writer_behavior::{
            RtpsStatelessWriterReceiveAckNackSubmessage, RtpsStatelessWriterSendSubmessages,
        },
        writer::{
            reader_locator::{RtpsReaderLocatorAttributes, RtpsReaderLocatorConstructor},
            reader_proxy::{RtpsReaderProxyAttributes, RtpsReaderProxyConstructor},
            stateful_writer::{RtpsStatefulWriterAttributes, RtpsStatefulWriterOperations},
            stateless_writer::RtpsStatelessWriterOperations,
            writer::{RtpsWriterAttributes, RtpsWriterOperations},
        },
    },
    discovery::{
        participant_discovery::ParticipantDiscovery,
        spdp::spdp_discovered_participant_data::RtpsSpdpDiscoveredParticipantDataAttributes,
    },
    messages::{
        overall_structure::RtpsMessageHeader,
        submessage_elements::TimestampSubmessageElement,
        submessages::{AckNackSubmessage, InfoTimestampSubmessage},
        types::{ParameterId, TIME_INVALID},
    },
    structure::{
        cache_change::{RtpsCacheChangeAttributes, RtpsCacheChangeConstructor},
        entity::RtpsEntityAttributes,
        history_cache::RtpsHistoryCacheOperations,
        types::{
            ChangeKind, EntityId, Guid, GuidPrefix, SequenceNumber, PROTOCOLVERSION, VENDOR_ID_S2E,
        },
    },
};
use serde::Serialize;

use crate::{
    data_representation_builtin_endpoints::{
        discovered_reader_data::DiscoveredReaderData, discovered_topic_data::DiscoveredTopicData,
        discovered_writer_data::DiscoveredWriterData,
        spdp_discovered_participant_data::SpdpDiscoveredParticipantData,
    },
    dds_type::{DdsSerialize, DdsType, LittleEndian},
    transport::{RtpsMessage, RtpsSubmessageType, TransportWrite},
    utils::{
        discovery_traits::AddMatchedReader,
        rtps_communication_traits::{ReceiveRtpsAckNackSubmessage, SendRtpsMessage},
        shared_object::{DdsRwLock, DdsShared, DdsWeak},
    },
};

use super::{
    publisher_impl::{AnnounceDataWriter, PublisherImpl},
    topic_impl::TopicImpl,
};

fn calculate_instance_handle(serialized_key: &[u8]) -> [u8; 16] {
    if serialized_key.len() <= 16 {
        let mut h = [0; 16];
        h[..serialized_key.len()].clone_from_slice(serialized_key);
        h
    } else {
        md5::compute(serialized_key).into()
    }
}

fn retrieve_instance_handle(
    handle: Option<InstanceHandle>,
    registered_instance_list: &HashMap<InstanceHandle, Vec<u8>>,
    serialized_key: &[u8],
) -> DdsResult<[u8; 16]> {
    match handle {
        Some(h) => {
            if let Some(stored_key) = registered_instance_list.get(&h) {
                if stored_key == &serialized_key {
                    Ok(h)
                } else {
                    Err(DdsError::PreconditionNotMet(
                        "Handle does not match instance".to_string(),
                    ))
                }
            } else {
                Err(DdsError::BadParameter)
            }
        }
        None => {
            let instance_handle = calculate_instance_handle(&serialized_key);
            if registered_instance_list.contains_key(&instance_handle) {
                Ok(instance_handle)
            } else {
                Err(DdsError::PreconditionNotMet(
                    "Instance not registered with this DataWriter".to_string(),
                ))
            }
        }
    }
}

pub trait AnyDataWriterListener<DW> {
    fn trigger_on_liveliness_lost(&mut self, _the_writer: DW, _status: LivelinessLostStatus);
    fn trigger_on_offered_deadline_missed(
        &mut self,
        _the_writer: DW,
        _status: OfferedDeadlineMissedStatus,
    );
    fn trigger_on_offered_incompatible_qos(
        &mut self,
        _the_writer: DW,
        _status: OfferedIncompatibleQosStatus,
    );
    fn trigger_on_publication_matched(
        &mut self,
        _the_writer: DW,
        _status: PublicationMatchedStatus,
    );
}

impl<Foo, DW> AnyDataWriterListener<DW> for Box<dyn DataWriterListener<Foo = Foo> + Send + Sync>
where
    DW: FooDataWriter<Foo>,
{
    fn trigger_on_liveliness_lost(&mut self, the_writer: DW, status: LivelinessLostStatus) {
        self.on_liveliness_lost(&the_writer, status);
    }

    fn trigger_on_offered_deadline_missed(
        &mut self,
        the_writer: DW,
        status: OfferedDeadlineMissedStatus,
    ) {
        self.on_offered_deadline_missed(&the_writer, status);
    }

    fn trigger_on_offered_incompatible_qos(
        &mut self,
        the_writer: DW,
        status: OfferedIncompatibleQosStatus,
    ) {
        self.on_offered_incompatible_qos(&the_writer, status);
    }

    fn trigger_on_publication_matched(&mut self, the_writer: DW, status: PublicationMatchedStatus) {
        self.on_publication_matched(&the_writer, status)
    }
}

pub enum RtpsWriter {
    Stateless(RtpsStatelessWriterImpl<StdTimer>),
    Stateful(RtpsStatefulWriterImpl<StdTimer>),
}

impl RtpsEntityAttributes for RtpsWriter {
    fn guid(&self) -> Guid {
        match self {
            RtpsWriter::Stateless(w) => w.guid(),
            RtpsWriter::Stateful(w) => w.guid(),
        }
    }
}

impl RtpsWriterOperations for RtpsWriter {
    type CacheChangeType = RtpsCacheChangeImpl;

    fn new_change(
        &mut self,
        kind: ChangeKind,
        data: <Self::CacheChangeType as RtpsCacheChangeConstructor>::DataType,
        inline_qos: <Self::CacheChangeType as RtpsCacheChangeConstructor>::ParameterListType,
        handle: rtps_pim::structure::types::InstanceHandle,
    ) -> Self::CacheChangeType {
        match self {
            RtpsWriter::Stateless(w) => w.new_change(kind, data, inline_qos, handle),
            RtpsWriter::Stateful(w) => w.new_change(kind, data, inline_qos, handle),
        }
    }
}

impl RtpsHistoryCacheOperations for RtpsWriter {
    type CacheChangeType = RtpsCacheChangeImpl;

    fn add_change(&mut self, change: Self::CacheChangeType) {
        match self {
            RtpsWriter::Stateless(w) => w.add_change(change),
            RtpsWriter::Stateful(w) => w.add_change(change),
        }
    }

    fn remove_change<F>(&mut self, f: F)
    where
        F: FnMut(&Self::CacheChangeType) -> bool,
    {
        match self {
            RtpsWriter::Stateless(w) => w.remove_change(f),
            RtpsWriter::Stateful(w) => w.remove_change(f),
        }
    }

    fn get_seq_num_min(&self) -> Option<SequenceNumber> {
        match self {
            RtpsWriter::Stateless(w) => w.get_seq_num_min(),
            RtpsWriter::Stateful(w) => w.get_seq_num_min(),
        }
    }

    fn get_seq_num_max(&self) -> Option<SequenceNumber> {
        match self {
            RtpsWriter::Stateless(w) => w.get_seq_num_max(),
            RtpsWriter::Stateful(w) => w.get_seq_num_max(),
        }
    }
}

impl RtpsWriterAttributes for RtpsWriter {
    type HistoryCacheType = RtpsHistoryCacheImpl;

    fn push_mode(&self) -> bool {
        match self {
            RtpsWriter::Stateless(w) => w.push_mode(),
            RtpsWriter::Stateful(w) => w.push_mode(),
        }
    }

    fn heartbeat_period(&self) -> rtps_pim::behavior::types::Duration {
        match self {
            RtpsWriter::Stateless(w) => w.heartbeat_period(),
            RtpsWriter::Stateful(w) => w.heartbeat_period(),
        }
    }

    fn nack_response_delay(&self) -> rtps_pim::behavior::types::Duration {
        match self {
            RtpsWriter::Stateless(w) => w.nack_response_delay(),
            RtpsWriter::Stateful(w) => w.nack_response_delay(),
        }
    }

    fn nack_suppression_duration(&self) -> rtps_pim::behavior::types::Duration {
        match self {
            RtpsWriter::Stateless(w) => w.nack_suppression_duration(),
            RtpsWriter::Stateful(w) => w.nack_suppression_duration(),
        }
    }

    fn last_change_sequence_number(&self) -> SequenceNumber {
        match self {
            RtpsWriter::Stateless(w) => w.last_change_sequence_number(),
            RtpsWriter::Stateful(w) => w.last_change_sequence_number(),
        }
    }

    fn data_max_size_serialized(&self) -> Option<i32> {
        match self {
            RtpsWriter::Stateless(w) => w.data_max_size_serialized(),
            RtpsWriter::Stateful(w) => w.data_max_size_serialized(),
        }
    }

    fn writer_cache(&mut self) -> &mut Self::HistoryCacheType {
        match self {
            RtpsWriter::Stateless(w) => w.writer_cache(),
            RtpsWriter::Stateful(w) => w.writer_cache(),
        }
    }
}

pub struct DataWriterImpl {
    qos: DdsRwLock<DataWriterQos>,
    rtps_writer: DdsRwLock<RtpsWriter>,
    sample_info: DdsRwLock<HashMap<SequenceNumber, Time>>,
    registered_instance_list: DdsRwLock<HashMap<InstanceHandle, Vec<u8>>>,
    listener: DdsRwLock<Option<<DdsShared<Self> as Entity>::Listener>>,
    topic: DdsShared<TopicImpl>,
    publisher: DdsWeak<PublisherImpl>,
    publication_matched_status: DdsRwLock<PublicationMatchedStatus>,
    offered_deadline_missed_status: DdsRwLock<OfferedDeadlineMissedStatus>,
    offered_incompatible_qos_status: DdsRwLock<OfferedIncompatibleQosStatus>,
    liveliness_lost_status: DdsRwLock<LivelinessLostStatus>,
    matched_subscription_list: DdsRwLock<HashMap<InstanceHandle, SubscriptionBuiltinTopicData>>,
    enabled: DdsRwLock<bool>,
}

impl DataWriterImpl {
    pub fn new(
        qos: DataWriterQos,
        rtps_writer: RtpsWriter,
        listener: Option<<DdsShared<Self> as Entity>::Listener>,
        topic: DdsShared<TopicImpl>,
        publisher: DdsWeak<PublisherImpl>,
    ) -> DdsShared<Self> {
        let liveliness_lost_status = LivelinessLostStatus {
            total_count: 0,
            total_count_change: 0,
        };

        let publication_matched_status = PublicationMatchedStatus {
            total_count: 0,
            total_count_change: 0,
            last_subscription_handle: HANDLE_NIL_NATIVE,
            current_count: 0,
            current_count_change: 0,
        };

        let offered_deadline_missed_status = OfferedDeadlineMissedStatus {
            total_count: 0,
            total_count_change: 0,
            last_instance_handle: HANDLE_NIL_NATIVE,
        };

        let offered_incompatible_qos_status = OfferedIncompatibleQosStatus {
            total_count: 0,
            total_count_change: 0,
            last_policy_id: 0,
            policies: vec![],
        };

        DdsShared::new(DataWriterImpl {
            qos: DdsRwLock::new(qos),
            rtps_writer: DdsRwLock::new(rtps_writer),
            sample_info: DdsRwLock::new(HashMap::new()),
            registered_instance_list: DdsRwLock::new(HashMap::new()),
            listener: DdsRwLock::new(listener),
            topic,
            publisher,
            publication_matched_status: DdsRwLock::new(publication_matched_status),
            offered_deadline_missed_status: DdsRwLock::new(offered_deadline_missed_status),
            offered_incompatible_qos_status: DdsRwLock::new(offered_incompatible_qos_status),
            liveliness_lost_status: DdsRwLock::new(liveliness_lost_status),
            matched_subscription_list: DdsRwLock::new(HashMap::new()),
            enabled: DdsRwLock::new(false),
        })
    }

    /// NOTE: This function is only useful for the SEDP writers so we probably need a separate
    /// type for those.
    pub fn add_matched_participant(
        &self,
        participant_discovery: &ParticipantDiscovery<SpdpDiscoveredParticipantData>,
    ) {
        let mut rtps_writer_lock = self.rtps_writer.write_lock();
        if let RtpsWriter::Stateful(rtps_writer) = &mut *rtps_writer_lock {
            if !rtps_writer
                .matched_readers()
                .into_iter()
                .any(|r| r.remote_reader_guid().prefix == participant_discovery.guid_prefix())
            {
                let type_name = self.topic.get_type_name().unwrap();
                if type_name == DiscoveredWriterData::type_name() {
                    participant_discovery
                        .discovered_participant_add_publications_writer(rtps_writer);
                } else if type_name == DiscoveredReaderData::type_name() {
                    participant_discovery
                        .discovered_participant_add_subscriptions_writer(rtps_writer);
                } else if type_name == DiscoveredTopicData::type_name() {
                    participant_discovery.discovered_participant_add_topics_writer(rtps_writer);
                }
            }
        }
    }
}

impl DdsShared<DataWriterImpl> {
    fn get_timestamp(&self) -> Time {
        self.datawriter_get_publisher()
            .expect("Failed to get parent publisher of datawriter.")
            .get_participant()
            .expect("Failed to get parent participant of publisher")
            .get_current_time()
            .expect("Failed to get current time from participant")
    }
}

impl ReceiveRtpsAckNackSubmessage for DdsShared<DataWriterImpl> {
    fn on_acknack_submessage_received(
        &self,
        acknack_submessage: &AckNackSubmessage<Vec<SequenceNumber>>,
        source_guid_prefix: GuidPrefix,
    ) {
        match &mut *self.rtps_writer.write_lock() {
            RtpsWriter::Stateless(stateless_rtps_writer) => {
                stateless_rtps_writer.on_acknack_submessage_received(&acknack_submessage)
            }
            RtpsWriter::Stateful(stateful_rtps_writer) => stateful_rtps_writer
                .on_acknack_submessage_received(&acknack_submessage, source_guid_prefix),
        }
    }
}

impl AddMatchedReader for DdsShared<DataWriterImpl> {
    fn add_matched_reader(&self, discovered_reader_data: &DiscoveredReaderData) {
        let reader_info = &discovered_reader_data.subscription_builtin_topic_data;
        let writer_topic_name = self.topic.get_name().unwrap();
        let writer_type_name = self.topic.get_type_name().unwrap();

        if reader_info.topic_name == writer_topic_name && &reader_info.type_name == writer_type_name
        {
            let writer_qos_lock = self.qos.read_lock();
            let parent_publisher_qos = self.get_publisher().unwrap().get_qos().unwrap();

            let mut incompatible_qos_policy_list = Vec::new();
            if !(writer_qos_lock.durability >= reader_info.durability) {
                incompatible_qos_policy_list.push(DURABILITY_QOS_POLICY_ID);
            }
            if !(parent_publisher_qos.presentation.access_scope
                >= reader_info.presentation.access_scope
                && parent_publisher_qos.presentation.coherent_access
                    == reader_info.presentation.coherent_access
                && parent_publisher_qos.presentation.ordered_access
                    == reader_info.presentation.ordered_access)
            {
                incompatible_qos_policy_list.push(PRESENTATION_QOS_POLICY_ID);
            }
            if !(writer_qos_lock.deadline >= reader_info.deadline) {
                incompatible_qos_policy_list.push(DEADLINE_QOS_POLICY_ID);
            }
            if !(writer_qos_lock.latency_budget >= reader_info.latency_budget) {
                incompatible_qos_policy_list.push(LATENCYBUDGET_QOS_POLICY_ID);
            }
            if !(writer_qos_lock.ownership == reader_info.ownership) {
                incompatible_qos_policy_list.push(OWNERSHIPSTRENGTH_QOS_POLICY_ID);
            }
            if !(writer_qos_lock.liveliness >= reader_info.liveliness) {
                incompatible_qos_policy_list.push(LIVELINESS_QOS_POLICY_ID);
            }
            if !(writer_qos_lock.reliability.kind >= reader_info.reliability.kind) {
                incompatible_qos_policy_list.push(RELIABILITY_QOS_POLICY_ID);
            }
            if !(writer_qos_lock.destination_order >= reader_info.destination_order) {
                incompatible_qos_policy_list.push(DESTINATIONORDER_QOS_POLICY_ID);
            }

            if incompatible_qos_policy_list.is_empty() {
                match &mut *self.rtps_writer.write_lock() {
                    RtpsWriter::Stateless(w) => {
                        for locator in discovered_reader_data
                            .reader_proxy
                            .unicast_locator_list
                            .iter()
                            .chain(
                                discovered_reader_data
                                    .reader_proxy
                                    .multicast_locator_list
                                    .iter(),
                            )
                        {
                            let a_locator = RtpsReaderLocatorAttributesImpl::new(
                                locator.clone(),
                                discovered_reader_data.reader_proxy.expects_inline_qos,
                            );
                            w.reader_locator_add(a_locator);
                        }
                    }
                    RtpsWriter::Stateful(w) => {
                        let reader_proxy = RtpsReaderProxyImpl::new(
                            discovered_reader_data.reader_proxy.remote_reader_guid,
                            discovered_reader_data.reader_proxy.remote_group_entity_id,
                            discovered_reader_data
                                .reader_proxy
                                .unicast_locator_list
                                .as_ref(),
                            discovered_reader_data
                                .reader_proxy
                                .multicast_locator_list
                                .as_ref(),
                            discovered_reader_data.reader_proxy.expects_inline_qos,
                            true,
                        );
                        w.matched_reader_add(reader_proxy);
                    }
                }
                self.matched_subscription_list
                    .write_lock()
                    .insert(reader_info.key.value.into(), reader_info.clone());

                // Drop the publication_matched_status_lock such that the listener can be triggered
                // if needed
                {
                    let mut publication_matched_status_lock =
                        self.publication_matched_status.write_lock();
                    publication_matched_status_lock.total_count += 1;
                    publication_matched_status_lock.total_count_change += 1;
                    publication_matched_status_lock.current_count_change += 1;
                }

                let mut listener_lock = self.listener.write_lock();
                if let Some(l) = listener_lock.as_mut() {
                    let publication_matched_status = self.get_publication_matched_status().unwrap();
                    l.trigger_on_publication_matched(self.clone(), publication_matched_status)
                }
            } else {
                {
                    let mut offered_incompatible_qos_status_lock =
                        self.offered_incompatible_qos_status.write_lock();
                    offered_incompatible_qos_status_lock.total_count += 1;
                    offered_incompatible_qos_status_lock.total_count_change += 1;
                    offered_incompatible_qos_status_lock.last_policy_id =
                        incompatible_qos_policy_list[0];
                    for incompatible_qos_policy in incompatible_qos_policy_list.into_iter() {
                        if let Some(policy_count) = offered_incompatible_qos_status_lock
                            .policies
                            .iter_mut()
                            .find(|x| x.policy_id == incompatible_qos_policy)
                        {
                            policy_count.count += 1;
                        } else {
                            offered_incompatible_qos_status_lock
                                .policies
                                .push(QosPolicyCount {
                                    policy_id: incompatible_qos_policy,
                                    count: 1,
                                })
                        }
                    }
                }

                let mut listener_lock = self.listener.write_lock();
                if let Some(l) = listener_lock.as_mut() {
                    let offered_incompatible_qos_status =
                        self.get_offered_incompatible_qos_status().unwrap();
                    l.trigger_on_offered_incompatible_qos(
                        self.clone(),
                        offered_incompatible_qos_status,
                    )
                }
            }
        }
    }
}

impl<Foo> FooDataWriter<Foo> for DdsShared<DataWriterImpl>
where
    Foo: DdsType + DdsSerialize,
{
    fn register_instance(&self, instance: &Foo) -> DdsResult<Option<InstanceHandle>> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        let timestamp = self.get_timestamp();
        self.register_instance_w_timestamp(instance, timestamp)
    }

    fn register_instance_w_timestamp(
        &self,
        instance: &Foo,
        _timestamp: Time,
    ) -> DdsResult<Option<InstanceHandle>> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        if Foo::has_key() {
            let serialized_key = instance.get_serialized_key::<LittleEndian>();
            let instance_handle = calculate_instance_handle(&serialized_key);

            let mut registered_instances_lock = self.registered_instance_list.write_lock();
            let qos_lock = self.qos.read_lock();
            if !registered_instances_lock.contains_key(&instance_handle) {
                if qos_lock.resource_limits.max_instances == LENGTH_UNLIMITED
                    || (registered_instances_lock.len() as i32)
                        < qos_lock.resource_limits.max_instances
                {
                    registered_instances_lock.insert(instance_handle, serialized_key);
                } else {
                    return Err(DdsError::OutOfResources);
                }
            }
            Ok(Some(instance_handle))
        } else {
            Ok(None)
        }
    }

    fn unregister_instance(&self, instance: &Foo, handle: Option<InstanceHandle>) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        let timestamp = self.get_timestamp();
        self.unregister_instance_w_timestamp(instance, handle, timestamp)
    }

    fn unregister_instance_w_timestamp(
        &self,
        instance: &Foo,
        handle: Option<InstanceHandle>,
        timestamp: Time,
    ) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        if Foo::has_key() {
            let serialized_key = instance.get_serialized_key::<LittleEndian>();

            let mut rtps_writer_lock = self.rtps_writer.write_lock();
            let mut sample_info_lock = self.sample_info.write_lock();
            let mut registered_instance_list_lock = self.registered_instance_list.write_lock();

            let instance_handle = retrieve_instance_handle(
                handle,
                &*registered_instance_list_lock,
                serialized_key.as_ref(),
            )?;
            let mut serialized_status_info = Vec::new();
            let mut serializer =
                cdr::Serializer::<_, cdr::LittleEndian>::new(&mut serialized_status_info);
            STATUS_INFO_UNREGISTERED_FLAG
                .serialize(&mut serializer)
                .unwrap();

            let inline_qos = vec![RtpsParameter::new(
                ParameterId(PID_STATUS_INFO),
                serialized_status_info,
            )];

            let change = rtps_writer_lock.new_change(
                ChangeKind::NotAliveUnregistered,
                serialized_key,
                inline_qos,
                instance_handle,
            );
            let sequence_number = change.sequence_number();
            rtps_writer_lock.add_change(change);
            sample_info_lock.insert(sequence_number, timestamp);
            registered_instance_list_lock.remove(&instance_handle);
            Ok(())
        } else {
            Err(DdsError::IllegalOperation)
        }
    }

    fn get_key_value(&self, key_holder: &mut Foo, handle: InstanceHandle) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        let registered_instance_list_lock = self.registered_instance_list.read_lock();

        let serialized_key = registered_instance_list_lock
            .get(&handle)
            .ok_or(DdsError::BadParameter)?;

        key_holder.set_key_fields_from_serialized_key(serialized_key.as_ref())
    }

    fn lookup_instance(&self, instance: &Foo) -> DdsResult<Option<InstanceHandle>> {
        let serialized_key = instance.get_serialized_key::<LittleEndian>();
        let instance_handle = calculate_instance_handle(&serialized_key);
        let registered_instance_list_lock = self.registered_instance_list.read_lock();
        if registered_instance_list_lock.contains_key(&instance_handle) {
            Ok(Some(instance_handle))
        } else {
            Ok(None)
        }
    }

    fn write(&self, data: &Foo, handle: Option<InstanceHandle>) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        let timestamp = self.get_timestamp();
        self.write_w_timestamp(data, handle, timestamp)
    }

    fn write_w_timestamp(
        &self,
        data: &Foo,
        _handle: Option<InstanceHandle>,
        timestamp: Time,
    ) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        let mut serialized_data = Vec::new();
        data.serialize::<_, LittleEndian>(&mut serialized_data)?;
        let mut rtps_writer_lock = self.rtps_writer.write_lock();
        let mut sample_info_lock = self.sample_info.write_lock();
        let change =
            rtps_writer_lock.new_change(ChangeKind::Alive, serialized_data, vec![], [0; 16]);
        let sequence_number = change.sequence_number();
        rtps_writer_lock.add_change(change);

        sample_info_lock.insert(sequence_number, timestamp);

        Ok(())
    }

    fn dispose(&self, data: &Foo, handle: Option<InstanceHandle>) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        let timestamp = self.get_timestamp();
        self.dispose_w_timestamp(data, handle, timestamp)
    }

    fn dispose_w_timestamp(
        &self,
        data: &Foo,
        handle: Option<InstanceHandle>,
        timestamp: Time,
    ) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        if Foo::has_key() {
            let serialized_key = data.get_serialized_key::<LittleEndian>();

            let mut rtps_writer_lock = self.rtps_writer.write_lock();
            let mut sample_info_lock = self.sample_info.write_lock();
            let registered_instance_list_lock = self.registered_instance_list.read_lock();

            let instance_handle = retrieve_instance_handle(
                handle,
                &*registered_instance_list_lock,
                serialized_key.as_ref(),
            )?;
            let mut serialized_status_info = Vec::new();
            let mut serializer =
                cdr::Serializer::<_, cdr::LittleEndian>::new(&mut serialized_status_info);
            STATUS_INFO_DISPOSED_FLAG
                .serialize(&mut serializer)
                .unwrap();

            let inline_qos = vec![RtpsParameter::new(
                ParameterId(PID_STATUS_INFO),
                serialized_status_info,
            )];

            let change = rtps_writer_lock.new_change(
                ChangeKind::NotAliveDisposed,
                serialized_key,
                inline_qos,
                instance_handle,
            );
            let sequence_number = change.sequence_number();
            rtps_writer_lock.add_change(change);
            sample_info_lock.insert(sequence_number, timestamp);

            Ok(())
        } else {
            Err(DdsError::IllegalOperation)
        }
    }
}

impl DataWriter for DdsShared<DataWriterImpl> {
    fn wait_for_acknowledgments(&self, _max_wait: Duration) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    fn get_liveliness_lost_status(&self) -> DdsResult<LivelinessLostStatus> {
        let liveliness_lost_status = self.liveliness_lost_status.read_lock().clone();
        self.liveliness_lost_status.write_lock().total_count_change = 0;
        Ok(liveliness_lost_status)
    }

    fn get_offered_deadline_missed_status(&self) -> DdsResult<OfferedDeadlineMissedStatus> {
        let offered_deadline_missed_status =
            self.offered_deadline_missed_status.read_lock().clone();
        self.offered_deadline_missed_status
            .write_lock()
            .total_count_change = 0;
        Ok(offered_deadline_missed_status)
    }

    fn get_offered_incompatible_qos_status(&self) -> DdsResult<OfferedIncompatibleQosStatus> {
        let offered_incompatible_qos_status =
            self.offered_incompatible_qos_status.read_lock().clone();
        self.offered_incompatible_qos_status
            .write_lock()
            .total_count_change = 0;
        Ok(offered_incompatible_qos_status)
    }

    fn get_publication_matched_status(&self) -> DdsResult<PublicationMatchedStatus> {
        let mut publication_matched_status_lock = self.publication_matched_status.write_lock();

        let mut publication_matched_status = publication_matched_status_lock.clone();
        publication_matched_status.current_count =
            self.matched_subscription_list.read_lock().len() as i32;

        publication_matched_status_lock.current_count_change = 0;
        publication_matched_status_lock.total_count_change = 0;
        Ok(publication_matched_status)
    }

    fn assert_liveliness(&self) -> DdsResult<()> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        todo!()
    }

    fn get_matched_subscription_data(
        &self,
        subscription_handle: InstanceHandle,
    ) -> DdsResult<SubscriptionBuiltinTopicData> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        self.matched_subscription_list
            .read_lock()
            .get(&subscription_handle)
            .cloned()
            .ok_or(DdsError::BadParameter)
    }

    fn get_matched_subscriptions(&self) -> DdsResult<Vec<InstanceHandle>> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        Ok(self
            .matched_subscription_list
            .read_lock()
            .iter()
            .map(|(key, _)| key.clone())
            .collect())
    }
}

impl DataWriterGetPublisher for DdsShared<DataWriterImpl> {
    type PublisherType = DdsShared<PublisherImpl>;

    fn datawriter_get_publisher(&self) -> DdsResult<Self::PublisherType> {
        Ok(self
            .publisher
            .upgrade()
            .expect("Failed to get parent publisher of data writer"))
    }
}

impl DataWriterGetTopic for DdsShared<DataWriterImpl> {
    type TopicType = DdsShared<TopicImpl>;

    fn datawriter_get_topic(&self) -> DdsResult<Self::TopicType> {
        Ok(self.topic.clone())
    }
}

impl Entity for DdsShared<DataWriterImpl> {
    type Qos = DataWriterQos;
    type Listener = Box<dyn AnyDataWriterListener<DdsShared<DataWriterImpl>> + Send + Sync>;

    fn set_qos(&self, qos: Option<Self::Qos>) -> DdsResult<()> {
        let qos = qos.unwrap_or_default();

        qos.is_consistent()?;
        if *self.enabled.read_lock() {
            self.qos.read_lock().check_immutability(&qos)?;
        }

        *self.qos.write_lock() = qos;

        Ok(())
    }

    fn get_qos(&self) -> DdsResult<Self::Qos> {
        Ok(self.qos.read_lock().clone())
    }

    fn set_listener(&self, a_listener: Option<Self::Listener>, _mask: StatusMask) -> DdsResult<()> {
        *self.listener.write_lock() = a_listener;
        Ok(())
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
        if !self.publisher.upgrade()?.is_enabled() {
            return Err(DdsError::PreconditionNotMet(
                "Parent publisher disabled".to_string(),
            ));
        }

        self.publisher.upgrade()?.announce_datawriter(self.into());
        *self.enabled.write_lock() = true;

        Ok(())
    }

    fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        if !*self.enabled.read_lock() {
            return Err(DdsError::NotEnabled);
        }

        Ok(self.rtps_writer.read_lock().guid().into())
    }
}

impl Into<DiscoveredWriterData> for &DdsShared<DataWriterImpl> {
    fn into(self) -> DiscoveredWriterData {
        let guid = self.rtps_writer.read_lock().guid();

        DiscoveredWriterData {
            writer_proxy: RtpsWriterProxy {
                remote_writer_guid: guid,
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
                data_max_size_serialized: None,
                remote_group_entity_id: EntityId::new([0; 3], 0),
            },

            publication_builtin_topic_data: PublicationBuiltinTopicData {
                key: BuiltInTopicKey { value: guid.into() },
                participant_key: BuiltInTopicKey { value: [1; 16] },
                topic_name: self.topic.get_name().unwrap(),
                type_name: self.topic.get_type_name().unwrap().to_string(),
                durability: DurabilityQosPolicy::default(),
                durability_service: DurabilityServiceQosPolicy::default(),
                deadline: DeadlineQosPolicy::default(),
                latency_budget: LatencyBudgetQosPolicy::default(),
                liveliness: LivelinessQosPolicy::default(),
                reliability: ReliabilityQosPolicy {
                    kind: ReliabilityQosPolicyKind::ReliableReliabilityQos,
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
        }
    }
}

impl SendRtpsMessage for DdsShared<DataWriterImpl> {
    fn send_message(&self, transport: &mut impl TransportWrite) {
        let destined_submessages = RefCell::new(Vec::new());

        let mut rtps_writer_lock = self.rtps_writer.write_lock();
        let sample_info_lock = self.sample_info.read_lock();
        let guid_prefix = rtps_writer_lock.guid().prefix();
        match &mut *rtps_writer_lock {
            RtpsWriter::Stateless(stateless_rtps_writer) => {
                stateless_rtps_writer.send_submessages(
                    |reader_locator, data| {
                        let info_ts =
                            if let Some(time) = sample_info_lock.get(&data.writer_sn.value) {
                                InfoTimestampSubmessage {
                                    endianness_flag: true,
                                    invalidate_flag: false,
                                    timestamp: TimestampSubmessageElement {
                                        value: rtps_pim::messages::types::Time(
                                            ((time.sec as u64) << 32) + time.nanosec as u64,
                                        ),
                                    },
                                }
                            } else {
                                InfoTimestampSubmessage {
                                    endianness_flag: true,
                                    invalidate_flag: true,
                                    timestamp: TimestampSubmessageElement {
                                        value: TIME_INVALID,
                                    },
                                }
                            };
                        destined_submessages.borrow_mut().push((
                            vec![reader_locator.locator()],
                            vec![
                                RtpsSubmessageType::InfoTimestamp(info_ts),
                                RtpsSubmessageType::Data(data),
                            ],
                        ));
                    },
                    |reader_locator, gap| {
                        destined_submessages.borrow_mut().push((
                            vec![reader_locator.locator()],
                            vec![RtpsSubmessageType::Gap(gap)],
                        ));
                    },
                    |_, _| (),
                );
            }
            RtpsWriter::Stateful(stateful_rtps_writer) => {
                stateful_rtps_writer.send_submessages(
                    |reader_proxy, data| {
                        let info_ts =
                            if let Some(time) = sample_info_lock.get(&data.writer_sn.value) {
                                InfoTimestampSubmessage {
                                    endianness_flag: true,
                                    invalidate_flag: false,
                                    timestamp: TimestampSubmessageElement {
                                        value: rtps_pim::messages::types::Time(
                                            ((time.sec as u64) << 32) + time.nanosec as u64,
                                        ),
                                    },
                                }
                            } else {
                                InfoTimestampSubmessage {
                                    endianness_flag: true,
                                    invalidate_flag: true,
                                    timestamp: TimestampSubmessageElement {
                                        value: TIME_INVALID,
                                    },
                                }
                            };
                        destined_submessages.borrow_mut().push((
                            reader_proxy.unicast_locator_list().to_vec(),
                            vec![
                                RtpsSubmessageType::InfoTimestamp(info_ts),
                                RtpsSubmessageType::Data(data),
                            ],
                        ));
                    },
                    |reader_proxy, gap| {
                        destined_submessages.borrow_mut().push((
                            reader_proxy.unicast_locator_list().to_vec(),
                            vec![RtpsSubmessageType::Gap(gap)],
                        ));
                    },
                    |reader_proxy, heartbeat| {
                        destined_submessages.borrow_mut().push((
                            reader_proxy.unicast_locator_list().to_vec(),
                            vec![RtpsSubmessageType::Heartbeat(heartbeat)],
                        ));
                    },
                );
            }
        }
        let writer_destined_submessages = destined_submessages.take();

        for (locator_list, submessages) in writer_destined_submessages {
            let header = RtpsMessageHeader {
                protocol: rtps_pim::messages::types::ProtocolId::PROTOCOL_RTPS,
                version: PROTOCOLVERSION,
                vendor_id: VENDOR_ID_S2E,
                guid_prefix,
            };

            let rtps_message = RtpsMessage {
                header,
                submessages,
            };
            for locator in locator_list {
                transport.write(&rtps_message, locator);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use dds_api::{
        dcps_psm::{BuiltInTopicKey, QosPolicyCount, TIME_INVALID},
        infrastructure::{
            qos::{PublisherQos, TopicQos},
            qos_policy::{
                DeadlineQosPolicy, DestinationOrderQosPolicy, DurabilityQosPolicy,
                GroupDataQosPolicy, LatencyBudgetQosPolicy, LivelinessQosPolicy,
                OwnershipQosPolicy, PartitionQosPolicy, PresentationQosPolicy,
                ReliabilityQosPolicy, ReliabilityQosPolicyKind, ResourceLimitsQosPolicy,
                TimeBasedFilterQosPolicy, TopicDataQosPolicy, UserDataQosPolicy,
            },
        },
    };
    use mockall::mock;
    use rtps_pim::{
        behavior::{
            types::DURATION_ZERO,
            writer::{
                reader_locator::RtpsReaderLocatorConstructor,
                stateful_writer::RtpsStatefulWriterConstructor,
                stateless_writer::{RtpsStatelessWriterConstructor, RtpsStatelessWriterOperations},
            },
        },
        messages::{
            submessage_elements::Parameter,
            submessage_elements::{
                EntityIdSubmessageElement, ParameterListSubmessageElement,
                SequenceNumberSubmessageElement, SerializedDataSubmessageElement,
            },
            submessages::DataSubmessage,
        },
        structure::{
            group::RtpsGroupConstructor,
            types::{
                EntityId, Locator, ENTITYID_UNKNOWN, GUIDPREFIX_UNKNOWN, GUID_UNKNOWN,
                PROTOCOLVERSION_2_4,
            },
        },
    };

    use crate::{
        data_representation_builtin_endpoints::discovered_reader_data::RtpsReaderProxy,
        dds_type::Endianness,
        rtps_impl::{
            rtps_group_impl::RtpsGroupImpl, rtps_stateful_writer_impl::RtpsReaderProxyImpl,
            rtps_stateless_writer_impl::RtpsReaderLocatorAttributesImpl,
        },
    };

    use super::*;

    mock! {
        Transport{}

        impl TransportWrite for Transport {
            fn write<'a>(&'a mut self, message: &RtpsMessage<'a>, destination_locator: Locator);
        }
    }

    struct MockFoo {}

    impl DdsSerialize for MockFoo {
        fn serialize<W: Write, E: Endianness>(&self, _writer: W) -> DdsResult<()> {
            Ok(())
        }
    }

    impl DdsType for MockFoo {
        fn type_name() -> &'static str {
            todo!()
        }
    }

    struct MockKeyedFoo {
        key: Vec<u8>,
    }

    impl DdsType for MockKeyedFoo {
        fn type_name() -> &'static str {
            todo!()
        }

        fn has_key() -> bool {
            true
        }

        fn get_serialized_key<E: Endianness>(&self) -> Vec<u8> {
            self.key.clone()
        }

        fn set_key_fields_from_serialized_key(&mut self, key: &[u8]) -> DdsResult<()> {
            self.key = key.to_vec();
            Ok(())
        }
    }

    impl DdsSerialize for MockKeyedFoo {
        fn serialize<W: Write, E: Endianness>(&self, _writer: W) -> DdsResult<()> {
            Ok(())
        }
    }

    fn create_data_writer_test_fixture() -> DdsShared<DataWriterImpl> {
        let dummy_topic = TopicImpl::new(GUID_UNKNOWN, TopicQos::default(), "", "", DdsWeak::new());

        let rtps_writer = RtpsStatefulWriterImpl::new(
            GUID_UNKNOWN,
            rtps_pim::structure::types::TopicKind::WithKey,
            rtps_pim::structure::types::ReliabilityKind::BestEffort,
            &[],
            &[],
            true,
            DURATION_ZERO,
            DURATION_ZERO,
            DURATION_ZERO,
            None,
        );

        let data_writer = DataWriterImpl::new(
            DataWriterQos::default(),
            RtpsWriter::Stateful(rtps_writer),
            None,
            dummy_topic,
            DdsWeak::new(),
        );
        *data_writer.enabled.write_lock() = true;
        data_writer
    }

    #[test]
    fn get_instance_handle() {
        let guid = Guid::new(
            GuidPrefix([3; 12]),
            EntityId {
                entity_key: [3; 3],
                entity_kind: 1,
            },
        );
        let dummy_topic = TopicImpl::new(GUID_UNKNOWN, TopicQos::default(), "", "", DdsWeak::new());

        let rtps_writer = RtpsStatefulWriterImpl::new(
            guid,
            rtps_pim::structure::types::TopicKind::NoKey,
            rtps_pim::structure::types::ReliabilityKind::BestEffort,
            &[],
            &[],
            true,
            DURATION_ZERO,
            DURATION_ZERO,
            DURATION_ZERO,
            None,
        );

        let data_writer = DataWriterImpl::new(
            DataWriterQos::default(),
            RtpsWriter::Stateful(rtps_writer),
            None,
            dummy_topic,
            DdsWeak::new(),
        );
        *data_writer.enabled.write_lock() = true;

        let expected_instance_handle: [u8; 16] = guid.into();
        let instance_handle = data_writer.get_instance_handle().unwrap();
        assert_eq!(expected_instance_handle, instance_handle);
    }

    #[test]
    fn register_instance_w_timestamp_different_keys() {
        let data_writer = create_data_writer_test_fixture();

        let instance_handle = data_writer
            .register_instance_w_timestamp(
                &MockKeyedFoo { key: vec![1, 2] },
                Time { sec: 0, nanosec: 0 },
            )
            .unwrap();
        assert_eq!(
            instance_handle,
            Some([1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
        );

        let instance_handle = data_writer
            .register_instance_w_timestamp(
                &MockKeyedFoo {
                    key: vec![1, 2, 3, 4, 5, 6],
                },
                Time { sec: 0, nanosec: 0 },
            )
            .unwrap();
        assert_eq!(
            instance_handle,
            Some([1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
        );

        let instance_handle = data_writer
            .register_instance_w_timestamp(
                &MockKeyedFoo {
                    key: vec![b'1'; 20],
                },
                Time { sec: 0, nanosec: 0 },
            )
            .unwrap();
        assert_eq!(
            instance_handle,
            Some([
                0x50, 0x20, 0x7f, 0xa2, 0x81, 0x4e, 0x81, 0xa0, 0x67, 0xbd, 0x26, 0x62, 0xba, 0x10,
                0xb0, 0xf1
            ])
        );
    }

    #[test]
    fn register_instance_w_timestamp_no_key() {
        let data_writer = create_data_writer_test_fixture();

        let instance_handle = data_writer
            .register_instance_w_timestamp(&MockFoo {}, TIME_INVALID)
            .unwrap();
        assert_eq!(instance_handle, None);
    }

    #[test]
    fn register_instance_w_timestamp_out_of_resources() {
        let dummy_topic = TopicImpl::new(GUID_UNKNOWN, TopicQos::default(), "", "", DdsWeak::new());

        let rtps_writer = RtpsStatefulWriterImpl::new(
            GUID_UNKNOWN,
            rtps_pim::structure::types::TopicKind::WithKey,
            rtps_pim::structure::types::ReliabilityKind::BestEffort,
            &[],
            &[],
            true,
            DURATION_ZERO,
            DURATION_ZERO,
            DURATION_ZERO,
            None,
        );

        let data_writer = DataWriterImpl::new(
            DataWriterQos {
                resource_limits: ResourceLimitsQosPolicy {
                    max_instances: 2,
                    ..ResourceLimitsQosPolicy::default()
                },
                ..DataWriterQos::default()
            },
            RtpsWriter::Stateful(rtps_writer),
            None,
            dummy_topic,
            DdsWeak::new(),
        );
        *data_writer.enabled.write_lock() = true;

        data_writer
            .register_instance_w_timestamp(&MockKeyedFoo { key: vec![1] }, TIME_INVALID)
            .unwrap();
        data_writer
            .register_instance_w_timestamp(&MockKeyedFoo { key: vec![2] }, TIME_INVALID)
            .unwrap();
        let instance_handle_result =
            data_writer.register_instance_w_timestamp(&MockKeyedFoo { key: vec![3] }, TIME_INVALID);
        assert_eq!(instance_handle_result, Err(DdsError::OutOfResources));

        // Already registered sample does not cause OutOfResources error
        data_writer
            .register_instance_w_timestamp(&MockKeyedFoo { key: vec![2] }, TIME_INVALID)
            .unwrap();
    }

    #[test]
    fn lookup_instance() {
        let data_writer = create_data_writer_test_fixture();

        let instance1 = MockKeyedFoo { key: vec![1] };
        let instance2 = MockKeyedFoo { key: vec![2] };

        let instance_handle1 = data_writer
            .register_instance_w_timestamp(&instance1, TIME_INVALID)
            .unwrap();

        assert_eq!(
            data_writer.lookup_instance(&instance1),
            Ok(instance_handle1)
        );
        assert_eq!(data_writer.lookup_instance(&instance2), Ok(None));
    }

    #[test]
    fn unregister_registered_instance() {
        let data_writer = create_data_writer_test_fixture();
        let instance = MockKeyedFoo { key: vec![1] };
        data_writer
            .register_instance_w_timestamp(&instance, TIME_INVALID)
            .unwrap();
        data_writer
            .unregister_instance_w_timestamp(&instance, None, TIME_INVALID)
            .unwrap();
        assert!(data_writer.lookup_instance(&instance).unwrap().is_none());
    }

    #[test]
    fn unregister_instance_not_registered() {
        let data_writer = create_data_writer_test_fixture();
        let instance = MockKeyedFoo { key: vec![1] };
        let result = data_writer.unregister_instance_w_timestamp(&instance, None, TIME_INVALID);
        assert_eq!(
            result,
            Err(DdsError::PreconditionNotMet(
                "Instance not registered with this DataWriter".to_string()
            ))
        );
    }

    #[test]
    fn unregister_instance_non_registered_handle() {
        let data_writer = create_data_writer_test_fixture();
        let instance = MockKeyedFoo { key: vec![1] };
        data_writer
            .register_instance_w_timestamp(&instance, TIME_INVALID)
            .unwrap();
        let result = data_writer.unregister_instance_w_timestamp(
            &instance,
            Some([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            TIME_INVALID,
        );
        assert_eq!(result, Err(DdsError::BadParameter));
    }

    #[test]
    fn unregister_instance_not_matching_handle() {
        let data_writer = create_data_writer_test_fixture();
        let instance1 = MockKeyedFoo { key: vec![1] };
        let instance2 = MockKeyedFoo { key: vec![2] };
        data_writer
            .register_instance_w_timestamp(&instance1, TIME_INVALID)
            .unwrap();
        data_writer
            .register_instance_w_timestamp(&instance2, TIME_INVALID)
            .unwrap();
        let result = data_writer.unregister_instance_w_timestamp(
            &instance1,
            Some([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            TIME_INVALID,
        );
        assert_eq!(
            result,
            Err(DdsError::PreconditionNotMet(
                "Handle does not match instance".to_string()
            ))
        );
    }

    #[test]
    fn dispose_not_registered() {
        let data_writer = create_data_writer_test_fixture();
        let instance = MockKeyedFoo { key: vec![1] };
        let result = data_writer.dispose_w_timestamp(&instance, None, TIME_INVALID);
        assert_eq!(
            result,
            Err(DdsError::PreconditionNotMet(
                "Instance not registered with this DataWriter".to_string()
            ))
        );
    }

    #[test]
    fn dispose_non_registered_handle() {
        let data_writer = create_data_writer_test_fixture();
        let instance = MockKeyedFoo { key: vec![1] };
        data_writer
            .register_instance_w_timestamp(&instance, TIME_INVALID)
            .unwrap();
        let result = data_writer.dispose_w_timestamp(
            &instance,
            Some([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            TIME_INVALID,
        );
        assert_eq!(result, Err(DdsError::BadParameter));
    }

    #[test]
    fn dispose_not_matching_handle() {
        let data_writer = create_data_writer_test_fixture();
        let instance1 = MockKeyedFoo { key: vec![1] };
        let instance2 = MockKeyedFoo { key: vec![2] };
        data_writer
            .register_instance_w_timestamp(&instance1, TIME_INVALID)
            .unwrap();
        data_writer
            .register_instance_w_timestamp(&instance2, TIME_INVALID)
            .unwrap();
        let result = data_writer.dispose_w_timestamp(
            &instance1,
            Some([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            TIME_INVALID,
        );
        assert_eq!(
            result,
            Err(DdsError::PreconditionNotMet(
                "Handle does not match instance".to_string()
            ))
        );
    }

    #[test]
    fn write_w_timestamp_stateless_message() {
        let mut stateless_rtps_writer = RtpsStatelessWriterImpl::new(
            GUID_UNKNOWN,
            rtps_pim::structure::types::TopicKind::NoKey,
            rtps_pim::structure::types::ReliabilityKind::BestEffort,
            &[],
            &[],
            true,
            DURATION_ZERO,
            DURATION_ZERO,
            DURATION_ZERO,
            None,
        );
        let locator = Locator::new(1, 7400, [1; 16]);
        let expects_inline_qos = false;
        let reader_locator = RtpsReaderLocatorAttributesImpl::new(locator, expects_inline_qos);
        stateless_rtps_writer.reader_locator_add(reader_locator);

        let dummy_topic = TopicImpl::new(GUID_UNKNOWN, TopicQos::default(), "", "", DdsWeak::new());

        let data_writer = DataWriterImpl::new(
            DataWriterQos::default(),
            RtpsWriter::Stateless(stateless_rtps_writer),
            None,
            dummy_topic,
            DdsWeak::new(),
        );
        *data_writer.enabled.write_lock() = true;

        data_writer
            .write_w_timestamp(&MockFoo {}, None, Time { sec: 0, nanosec: 0 })
            .unwrap();

        let mut mock_transport = MockTransport::new();
        mock_transport
            .expect_write()
            .withf(move |message, destination_locator| {
                message.submessages.len() == 2 && destination_locator == &locator
            })
            .once()
            .return_const(());
        data_writer.send_message(&mut mock_transport);
    }

    #[test]
    fn write_w_timestamp_stateful_message() {
        let mut stateful_rtps_writer = RtpsStatefulWriterImpl::new(
            GUID_UNKNOWN,
            rtps_pim::structure::types::TopicKind::NoKey,
            rtps_pim::structure::types::ReliabilityKind::BestEffort,
            &[],
            &[],
            true,
            DURATION_ZERO,
            DURATION_ZERO,
            DURATION_ZERO,
            None,
        );
        let locator = Locator::new(1, 7400, [1; 16]);
        let expects_inline_qos = false;
        let is_active = true;
        let reader_proxy = RtpsReaderProxyImpl::new(
            GUID_UNKNOWN,
            ENTITYID_UNKNOWN,
            &[locator],
            &[],
            expects_inline_qos,
            is_active,
        );
        stateful_rtps_writer.matched_reader_add(reader_proxy);

        let dummy_topic = TopicImpl::new(GUID_UNKNOWN, TopicQos::default(), "", "", DdsWeak::new());

        let data_writer = DataWriterImpl::new(
            DataWriterQos::default(),
            RtpsWriter::Stateful(stateful_rtps_writer),
            None,
            dummy_topic,
            DdsWeak::new(),
        );
        *data_writer.enabled.write_lock() = true;

        data_writer
            .write_w_timestamp(&MockFoo {}, None, Time { sec: 0, nanosec: 0 })
            .unwrap();

        let mut mock_transport = MockTransport::new();
        mock_transport
            .expect_write()
            .withf(move |message, destination_locator| {
                message.submessages.len() == 2 && destination_locator == &locator
            })
            .once()
            .return_const(());
        data_writer.send_message(&mut mock_transport);
    }

    #[test]
    fn unregister_w_timestamp_message() {
        let mut stateless_rtps_writer = RtpsStatelessWriterImpl::new(
            GUID_UNKNOWN,
            rtps_pim::structure::types::TopicKind::NoKey,
            rtps_pim::structure::types::ReliabilityKind::BestEffort,
            &[],
            &[],
            true,
            DURATION_ZERO,
            DURATION_ZERO,
            DURATION_ZERO,
            None,
        );
        let locator = Locator::new(1, 7400, [1; 16]);
        let expects_inline_qos = false;
        let reader_locator = RtpsReaderLocatorAttributesImpl::new(locator, expects_inline_qos);
        stateless_rtps_writer.reader_locator_add(reader_locator);

        let dummy_topic = TopicImpl::new(GUID_UNKNOWN, TopicQos::default(), "", "", DdsWeak::new());

        let data_writer = DataWriterImpl::new(
            DataWriterQos::default(),
            RtpsWriter::Stateless(stateless_rtps_writer),
            None,
            dummy_topic,
            DdsWeak::new(),
        );
        *data_writer.enabled.write_lock() = true;

        let instance = MockKeyedFoo { key: vec![1] };

        data_writer
            .register_instance_w_timestamp(&instance, Time { sec: 0, nanosec: 0 })
            .unwrap();
        data_writer
            .unregister_instance_w_timestamp(&instance, None, Time { sec: 0, nanosec: 0 })
            .unwrap();

        let mut mock_transport = MockTransport::new();
        let expected_message = RtpsMessage {
            header: RtpsMessageHeader {
                protocol: rtps_pim::messages::types::ProtocolId::PROTOCOL_RTPS,
                version: PROTOCOLVERSION_2_4,
                vendor_id: VENDOR_ID_S2E,
                guid_prefix: GUIDPREFIX_UNKNOWN,
            },
            submessages: vec![
                RtpsSubmessageType::InfoTimestamp(InfoTimestampSubmessage {
                    endianness_flag: true,
                    invalidate_flag: false,
                    timestamp: TimestampSubmessageElement {
                        value: rtps_pim::messages::types::Time(0),
                    },
                }),
                RtpsSubmessageType::Data(DataSubmessage {
                    endianness_flag: true,
                    inline_qos_flag: true,
                    data_flag: false,
                    key_flag: true,
                    non_standard_payload_flag: false,
                    reader_id: EntityIdSubmessageElement {
                        value: ENTITYID_UNKNOWN,
                    },
                    writer_id: EntityIdSubmessageElement {
                        value: ENTITYID_UNKNOWN,
                    },
                    writer_sn: SequenceNumberSubmessageElement { value: 1 },
                    inline_qos: ParameterListSubmessageElement {
                        parameter: vec![Parameter {
                            parameter_id: ParameterId(PID_STATUS_INFO),
                            length: 4,
                            value: &[2, 0, 0, 0],
                        }],
                    },
                    serialized_payload: SerializedDataSubmessageElement { value: &[1] },
                }),
            ],
        };
        mock_transport
            .expect_write()
            .withf(move |message, destination_locator| {
                message == &expected_message && destination_locator == &locator
            })
            .once()
            .return_const(());
        data_writer.send_message(&mut mock_transport);
    }

    #[test]
    fn dispose_w_timestamp_message() {
        let mut stateless_rtps_writer = RtpsStatelessWriterImpl::new(
            GUID_UNKNOWN,
            rtps_pim::structure::types::TopicKind::NoKey,
            rtps_pim::structure::types::ReliabilityKind::BestEffort,
            &[],
            &[],
            true,
            DURATION_ZERO,
            DURATION_ZERO,
            DURATION_ZERO,
            None,
        );
        let locator = Locator::new(1, 7400, [1; 16]);
        let expects_inline_qos = false;
        let reader_locator = RtpsReaderLocatorAttributesImpl::new(locator, expects_inline_qos);
        stateless_rtps_writer.reader_locator_add(reader_locator);

        let dummy_topic = TopicImpl::new(GUID_UNKNOWN, TopicQos::default(), "", "", DdsWeak::new());

        let data_writer = DataWriterImpl::new(
            DataWriterQos::default(),
            RtpsWriter::Stateless(stateless_rtps_writer),
            None,
            dummy_topic,
            DdsWeak::new(),
        );
        *data_writer.enabled.write_lock() = true;

        let instance = MockKeyedFoo { key: vec![1] };

        data_writer
            .register_instance_w_timestamp(&instance, Time { sec: 0, nanosec: 0 })
            .unwrap();
        data_writer
            .dispose_w_timestamp(&instance, None, Time { sec: 0, nanosec: 0 })
            .unwrap();

        let mut mock_transport = MockTransport::new();
        let expected_message = RtpsMessage {
            header: RtpsMessageHeader {
                protocol: rtps_pim::messages::types::ProtocolId::PROTOCOL_RTPS,
                version: PROTOCOLVERSION_2_4,
                vendor_id: VENDOR_ID_S2E,
                guid_prefix: GUIDPREFIX_UNKNOWN,
            },
            submessages: vec![
                RtpsSubmessageType::InfoTimestamp(InfoTimestampSubmessage {
                    endianness_flag: true,
                    invalidate_flag: false,
                    timestamp: TimestampSubmessageElement {
                        value: rtps_pim::messages::types::Time(0),
                    },
                }),
                RtpsSubmessageType::Data(DataSubmessage {
                    endianness_flag: true,
                    inline_qos_flag: true,
                    data_flag: false,
                    key_flag: true,
                    non_standard_payload_flag: false,
                    reader_id: EntityIdSubmessageElement {
                        value: ENTITYID_UNKNOWN,
                    },
                    writer_id: EntityIdSubmessageElement {
                        value: ENTITYID_UNKNOWN,
                    },
                    writer_sn: SequenceNumberSubmessageElement { value: 1 },
                    inline_qos: ParameterListSubmessageElement {
                        parameter: vec![Parameter {
                            parameter_id: ParameterId(PID_STATUS_INFO),
                            length: 4,
                            value: &[1, 0, 0, 0],
                        }],
                    },
                    serialized_payload: SerializedDataSubmessageElement { value: &[1] },
                }),
            ],
        };
        mock_transport
            .expect_write()
            .withf(move |message, destination_locator| {
                message == &expected_message && destination_locator == &locator
            })
            .once()
            .return_const(());
        data_writer.send_message(&mut mock_transport);
    }

    #[test]
    fn get_key_value_known_instance() {
        let data_writer = create_data_writer_test_fixture();

        let instance_handle = data_writer
            .register_instance_w_timestamp(
                &MockKeyedFoo { key: vec![1, 2] },
                Time { sec: 0, nanosec: 0 },
            )
            .unwrap()
            .unwrap();

        let mut keyed_foo = MockKeyedFoo { key: vec![] };
        data_writer
            .get_key_value(&mut keyed_foo, instance_handle)
            .unwrap();
        assert_eq!(keyed_foo.key, vec![1, 2]);
    }

    #[test]
    fn get_key_value_unknown_instance() {
        let data_writer = create_data_writer_test_fixture();

        data_writer
            .register_instance_w_timestamp(
                &MockKeyedFoo { key: vec![1, 2] },
                Time { sec: 0, nanosec: 0 },
            )
            .unwrap()
            .unwrap();

        let mut keyed_foo = MockKeyedFoo { key: vec![] };
        assert_eq!(
            data_writer.get_key_value(&mut keyed_foo, [1; 16]),
            Err(DdsError::BadParameter)
        );
    }

    #[test]
    fn add_compatible_matched_reader() {
        let type_name = "test_type";
        let topic_name = "test_topic".to_string();
        let parent_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            DdsWeak::new(),
        );
        let test_topic = TopicImpl::new(
            GUID_UNKNOWN,
            TopicQos::default(),
            type_name,
            &topic_name,
            DdsWeak::new(),
        );

        let rtps_writer = RtpsStatefulWriterImpl::new(
            GUID_UNKNOWN,
            rtps_pim::structure::types::TopicKind::WithKey,
            rtps_pim::structure::types::ReliabilityKind::BestEffort,
            &[],
            &[],
            true,
            DURATION_ZERO,
            DURATION_ZERO,
            DURATION_ZERO,
            None,
        );

        let data_writer = DataWriterImpl::new(
            DataWriterQos::default(),
            RtpsWriter::Stateful(rtps_writer),
            None,
            test_topic,
            parent_publisher.downgrade(),
        );
        *data_writer.enabled.write_lock() = true;
        let subscription_builtin_topic_data = SubscriptionBuiltinTopicData {
            key: BuiltInTopicKey { value: [2; 16] },
            participant_key: BuiltInTopicKey { value: [1; 16] },
            topic_name: topic_name.clone(),
            type_name: type_name.to_string(),
            durability: DurabilityQosPolicy::default(),
            deadline: DeadlineQosPolicy::default(),
            latency_budget: LatencyBudgetQosPolicy::default(),
            liveliness: LivelinessQosPolicy::default(),
            reliability: ReliabilityQosPolicy {
                kind: ReliabilityQosPolicyKind::BestEffortReliabilityQos,
                max_blocking_time: Duration::new(0, 0),
            },
            ownership: OwnershipQosPolicy::default(),
            destination_order: DestinationOrderQosPolicy::default(),
            user_data: UserDataQosPolicy::default(),
            time_based_filter: TimeBasedFilterQosPolicy::default(),
            presentation: PresentationQosPolicy::default(),
            partition: PartitionQosPolicy::default(),
            topic_data: TopicDataQosPolicy::default(),
            group_data: GroupDataQosPolicy::default(),
        };
        let discovered_reader_data = DiscoveredReaderData {
            reader_proxy: RtpsReaderProxy {
                remote_reader_guid: Guid {
                    prefix: GuidPrefix([2; 12]),
                    entity_id: EntityId {
                        entity_key: [2; 3],
                        entity_kind: 2,
                    },
                },
                remote_group_entity_id: ENTITYID_UNKNOWN,
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
                expects_inline_qos: false,
            },
            subscription_builtin_topic_data: subscription_builtin_topic_data.clone(),
        };
        data_writer.add_matched_reader(&discovered_reader_data);

        let publication_matched_status = data_writer.get_publication_matched_status().unwrap();
        assert_eq!(publication_matched_status.current_count, 1);
        assert_eq!(publication_matched_status.current_count_change, 1);
        assert_eq!(publication_matched_status.total_count, 1);
        assert_eq!(publication_matched_status.total_count_change, 1);

        let matched_subscriptions = data_writer.get_matched_subscriptions().unwrap();
        assert_eq!(matched_subscriptions.len(), 1);
        assert_eq!(matched_subscriptions[0], [2; 16]);
        let matched_subscription_data = data_writer
            .get_matched_subscription_data(matched_subscriptions[0])
            .unwrap();
        assert_eq!(matched_subscription_data, subscription_builtin_topic_data);
    }

    #[test]
    fn add_incompatible_matched_reader() {
        let type_name = "test_type";
        let topic_name = "test_topic".to_string();
        let parent_publisher = PublisherImpl::new(
            PublisherQos::default(),
            RtpsGroupImpl::new(GUID_UNKNOWN),
            DdsWeak::new(),
        );
        let test_topic = TopicImpl::new(
            GUID_UNKNOWN,
            TopicQos::default(),
            type_name,
            &topic_name,
            DdsWeak::new(),
        );

        let rtps_writer = RtpsStatefulWriterImpl::new(
            GUID_UNKNOWN,
            rtps_pim::structure::types::TopicKind::WithKey,
            rtps_pim::structure::types::ReliabilityKind::BestEffort,
            &[],
            &[],
            true,
            DURATION_ZERO,
            DURATION_ZERO,
            DURATION_ZERO,
            None,
        );
        let mut data_writer_qos = DataWriterQos::default();
        data_writer_qos.reliability.kind = ReliabilityQosPolicyKind::BestEffortReliabilityQos;
        let data_writer = DataWriterImpl::new(
            data_writer_qos,
            RtpsWriter::Stateful(rtps_writer),
            None,
            test_topic,
            parent_publisher.downgrade(),
        );
        *data_writer.enabled.write_lock() = true;
        let subscription_builtin_topic_data = SubscriptionBuiltinTopicData {
            key: BuiltInTopicKey { value: [2; 16] },
            participant_key: BuiltInTopicKey { value: [1; 16] },
            topic_name: topic_name.clone(),
            type_name: type_name.to_string(),
            durability: DurabilityQosPolicy::default(),
            deadline: DeadlineQosPolicy::default(),
            latency_budget: LatencyBudgetQosPolicy::default(),
            liveliness: LivelinessQosPolicy::default(),
            reliability: ReliabilityQosPolicy {
                kind: ReliabilityQosPolicyKind::ReliableReliabilityQos,
                max_blocking_time: Duration::new(0, 0),
            },
            ownership: OwnershipQosPolicy::default(),
            destination_order: DestinationOrderQosPolicy::default(),
            user_data: UserDataQosPolicy::default(),
            time_based_filter: TimeBasedFilterQosPolicy::default(),
            presentation: PresentationQosPolicy::default(),
            partition: PartitionQosPolicy::default(),
            topic_data: TopicDataQosPolicy::default(),
            group_data: GroupDataQosPolicy::default(),
        };
        let discovered_reader_data = DiscoveredReaderData {
            reader_proxy: RtpsReaderProxy {
                remote_reader_guid: Guid {
                    prefix: GuidPrefix([2; 12]),
                    entity_id: EntityId {
                        entity_key: [2; 3],
                        entity_kind: 2,
                    },
                },
                remote_group_entity_id: ENTITYID_UNKNOWN,
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
                expects_inline_qos: false,
            },
            subscription_builtin_topic_data: subscription_builtin_topic_data.clone(),
        };
        data_writer.add_matched_reader(&discovered_reader_data);

        let matched_subscriptions = data_writer.get_matched_subscriptions().unwrap();
        assert_eq!(matched_subscriptions.len(), 0);

        let offered_incompatible_qos_status =
            data_writer.get_offered_incompatible_qos_status().unwrap();
        assert_eq!(offered_incompatible_qos_status.total_count, 1);
        assert_eq!(offered_incompatible_qos_status.total_count_change, 1);
        assert_eq!(
            offered_incompatible_qos_status.last_policy_id,
            RELIABILITY_QOS_POLICY_ID
        );
        assert_eq!(
            offered_incompatible_qos_status.policies,
            vec![QosPolicyCount {
                policy_id: RELIABILITY_QOS_POLICY_ID,
                count: 1,
            }]
        )
    }
}
