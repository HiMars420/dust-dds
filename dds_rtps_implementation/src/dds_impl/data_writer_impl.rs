use std::cell::RefCell;

use rust_dds_api::{
    dcps_psm::InstanceHandle,
    infrastructure::{entity::Entity, qos::DataWriterQos},
    publication::{
        data_writer::DataWriter, data_writer_listener::DataWriterListener, publisher::Publisher,
    },
    return_type::DDSResult,
    topic::topic::Topic,
};
use rust_rtps_pim::{
    behavior::{
        stateful_writer_behavior::StatefulWriterBehavior,
        stateless_writer_behavior::StatelessWriterBehavior,
        writer::{
            reader_locator::{RtpsReaderLocatorAttributes, RtpsReaderLocatorOperations},
            reader_proxy::{RtpsReaderProxyAttributes, RtpsReaderProxyOperations},
            writer::{RtpsWriterAttributes, RtpsWriterOperations},
        },
    },
    messages::types::Count,
    structure::{
        history_cache::{
            RtpsHistoryCacheAddChange, RtpsHistoryCacheGetChange, RtpsHistoryCacheOperations,
        },
        types::{ChangeKind, Locator},
    },
};
use rust_rtps_psm::messages::{
    overall_structure::RtpsSubmessageTypeWrite, submessage_elements::Parameter,
    submessages::DataSubmessageWrite,
};

use crate::{
    dds_type::{DdsSerialize, LittleEndian},
    rtps_impl::rtps_writer_history_cache_impl::WriterCacheChange,
    utils::clock::Timer,
};

use super::publisher_impl::{StatefulWriterSubmessageProducer, StatelessWriterSubmessageProducer};

pub struct DataWriterImpl<Foo, W, C> {
    _qos: DataWriterQos,
    rtps_writer_impl: W,
    _listener: Option<Box<dyn DataWriterListener<DataType = Foo> + Send + Sync>>,
    heartbeat_timer: C,
    heartbeat_count: Count,
}

impl<Foo, W, C> DataWriterImpl<Foo, W, C>
where
    Foo: Send + 'static,
{
    pub fn new(qos: DataWriterQos, rtps_writer_impl: W, heartbeat_timer: C) -> Self {
        Self {
            _qos: qos,
            rtps_writer_impl,
            _listener: None,
            heartbeat_timer,
            heartbeat_count: Count(0),
        }
    }
}

impl<Foo, W, C> AsRef<W> for DataWriterImpl<Foo, W, C> {
    fn as_ref(&self) -> &W {
        &self.rtps_writer_impl
    }
}

impl<Foo, W, C> AsMut<W> for DataWriterImpl<Foo, W, C> {
    fn as_mut(&mut self) -> &mut W {
        &mut self.rtps_writer_impl
    }
}

impl<Foo, W, C, H> DataWriter<Foo> for DataWriterImpl<Foo, W, C>
where
    Foo: DdsSerialize,
    W: RtpsWriterOperations<
            CacheChangeType = H::CacheChangeType,
            DataType = Vec<u8>,
            ParameterListType = Vec<Parameter<Vec<u8>>>,
        > + RtpsWriterAttributes<WriterHistoryCacheType = H>,
    H: RtpsHistoryCacheAddChange,
{
    fn register_instance(&mut self, _instance: Foo) -> DDSResult<Option<InstanceHandle>> {
        unimplemented!()
    }

    fn register_instance_w_timestamp(
        &mut self,
        _instance: Foo,
        _timestamp: rust_dds_api::dcps_psm::Time,
    ) -> DDSResult<Option<InstanceHandle>> {
        todo!()
    }

    fn unregister_instance(
        &mut self,
        _instance: Foo,
        _handle: Option<InstanceHandle>,
    ) -> DDSResult<()> {
        unimplemented!()
    }

    fn unregister_instance_w_timestamp(
        &mut self,
        _instance: Foo,
        _handle: Option<InstanceHandle>,
        _timestamp: rust_dds_api::dcps_psm::Time,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_key_value(&self, _key_holder: &mut Foo, _handle: InstanceHandle) -> DDSResult<()> {
        todo!()
    }

    fn lookup_instance(&self, _instance: &Foo) -> DDSResult<Option<InstanceHandle>> {
        todo!()
    }

    fn write(&mut self, _data: &Foo, _handle: Option<InstanceHandle>) -> DDSResult<()> {
        unimplemented!()
    }

    fn write_w_timestamp(
        &mut self,
        data: &Foo,
        _handle: Option<InstanceHandle>,
        _timestamp: rust_dds_api::dcps_psm::Time,
    ) -> DDSResult<()> {
        let mut serialized_data = Vec::new();
        data.serialize::<_, LittleEndian>(&mut serialized_data)
            .unwrap();
        let change =
            self.rtps_writer_impl
                .new_change(ChangeKind::Alive, serialized_data, vec![], 0);
        self.rtps_writer_impl.writer_cache().add_change(change);
        Ok(())
    }

    fn dispose(&mut self, _data: Foo, _handle: Option<InstanceHandle>) -> DDSResult<()> {
        unimplemented!()
    }

    fn dispose_w_timestamp(
        &mut self,
        _data: Foo,
        _handle: Option<InstanceHandle>,
        _timestamp: rust_dds_api::dcps_psm::Time,
    ) -> DDSResult<()> {
        todo!()
    }

    fn wait_for_acknowledgments(
        &self,
        _max_wait: rust_dds_api::dcps_psm::Duration,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_liveliness_lost_status(
        &self,
        _status: &mut rust_dds_api::dcps_psm::LivelinessLostStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_offered_deadline_missed_status(
        &self,
        _status: &mut rust_dds_api::dcps_psm::OfferedDeadlineMissedStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_offered_incompatible_qos_status(
        &self,
        _status: &mut rust_dds_api::dcps_psm::OfferedIncompatibleQosStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_publication_matched_status(
        &self,
        _status: &mut rust_dds_api::dcps_psm::PublicationMatchedStatus,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_topic(&self) -> &dyn Topic<Foo> {
        unimplemented!()
    }

    fn get_publisher(&self) -> &dyn Publisher {
        unimplemented!()
    }

    fn assert_liveliness(&self) -> DDSResult<()> {
        todo!()
    }

    fn get_matched_subscription_data(
        &self,
        _subscription_data: rust_dds_api::builtin_topics::SubscriptionBuiltinTopicData,
        _subscription_handle: InstanceHandle,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_matched_subscriptions(
        &self,
        _subscription_handles: &mut [InstanceHandle],
    ) -> DDSResult<()> {
        todo!()
    }
}

impl<Foo, W, C> Entity for DataWriterImpl<Foo, W, C> {
    type Qos = DataWriterQos;
    type Listener = Box<dyn DataWriterListener<DataType = Foo>>;

    fn set_qos(&mut self, _qos: Option<Self::Qos>) -> DDSResult<()> {
        // let qos = qos.unwrap_or_default();
        // qos.is_consistent()?;
        // self.qos = qos;
        // Ok(())
        todo!()
    }

    fn get_qos(&self) -> DDSResult<Self::Qos> {
        // &self.qos
        todo!()
    }

    fn set_listener(
        &self,
        _a_listener: Option<Self::Listener>,
        _mask: rust_dds_api::dcps_psm::StatusMask,
    ) -> DDSResult<()> {
        todo!()
    }

    fn get_listener(&self) -> DDSResult<Option<Self::Listener>> {
        todo!()
    }

    fn get_statuscondition(
        &self,
    ) -> DDSResult<rust_dds_api::infrastructure::entity::StatusCondition> {
        todo!()
    }

    fn get_status_changes(&self) -> DDSResult<rust_dds_api::dcps_psm::StatusMask> {
        todo!()
    }

    fn enable(&self) -> DDSResult<()> {
        todo!()
    }

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle> {
        todo!()
    }
}

impl<Foo, C, W, R, H> StatelessWriterSubmessageProducer for DataWriterImpl<Foo, W, C>
where
    for<'a> &'a mut W: IntoIterator<Item = StatelessWriterBehavior<'a, R, H>>,
    R: RtpsReaderLocatorOperations + RtpsReaderLocatorAttributes + 'static,
    H: for<'a> RtpsHistoryCacheGetChange<CacheChangeType = WriterCacheChange> + 'static,
{
    fn produce_submessages(&mut self) -> Vec<(&'_ Locator, Vec<RtpsSubmessageTypeWrite<'_>>)> {
        let mut destined_submessages = Vec::new();

        for behavior in &mut self.rtps_writer_impl {
            match behavior {
                StatelessWriterBehavior::BestEffort(mut best_effort_behavior) => {
                    let submessages = RefCell::new(Vec::new());
                    best_effort_behavior.send_unsent_changes(
                        |data: DataSubmessageWrite| {
                            submessages
                                .borrow_mut()
                                .push(RtpsSubmessageTypeWrite::Data(data))
                        },
                        |gap| {
                            submessages
                                .borrow_mut()
                                .push(RtpsSubmessageTypeWrite::Gap(gap))
                        },
                    );
                    let submessages = submessages.take();
                    if !submessages.is_empty() {
                        destined_submessages
                            .push((best_effort_behavior.reader_locator.locator(), submessages));
                    }
                }
                StatelessWriterBehavior::Reliable(_) => todo!(),
            };
        }
        destined_submessages
    }
}

impl<Foo, C, W, R, H> StatefulWriterSubmessageProducer for DataWriterImpl<Foo, W, C>
where
    W: RtpsWriterAttributes,
    for<'a> &'a mut W: IntoIterator<Item = StatefulWriterBehavior<'a, R, H>>,
    H: RtpsHistoryCacheOperations
        + for<'a> RtpsHistoryCacheGetChange<CacheChangeType = WriterCacheChange>
        + 'static,
    R: RtpsReaderProxyOperations + RtpsReaderProxyAttributes + 'static,
    C: Timer,
{
    fn produce_submessages(
        &mut self,
    ) -> Vec<(
        &'_ dyn RtpsReaderProxyAttributes,
        Vec<RtpsSubmessageTypeWrite<'_>>,
    )> {
        let mut destined_submessages = Vec::new();
        let heartbeat_period_duration = std::time::Duration::new(
            self.rtps_writer_impl.heartbeat_period().seconds as u64,
            self.rtps_writer_impl.heartbeat_period().fraction,
        );

        let send_heartbeat = if self.heartbeat_timer.elapsed() > heartbeat_period_duration {
            self.heartbeat_count += Count(1);
            self.heartbeat_timer.reset();
            true
        } else {
            false
        };

        for behavior in &mut self.rtps_writer_impl {
            match behavior {
                StatefulWriterBehavior::BestEffort(_) => todo!(),
                StatefulWriterBehavior::Reliable(mut reliable_behavior) => {
                    let submessages = RefCell::new(Vec::new());
                    if send_heartbeat {
                        reliable_behavior.send_heartbeat(self.heartbeat_count, &mut |heartbeat| {
                            submessages
                                .borrow_mut()
                                .push(RtpsSubmessageTypeWrite::Heartbeat(heartbeat));
                        });
                    }

                    reliable_behavior.send_unsent_changes(
                        |data| {
                            submessages
                                .borrow_mut()
                                .push(RtpsSubmessageTypeWrite::Data(data))
                        },
                        |gap| {
                            submessages
                                .borrow_mut()
                                .push(RtpsSubmessageTypeWrite::Gap(gap))
                        },
                    );
                    let submessages = submessages.take();

                    if !submessages.is_empty() {
                        let reader_proxy_attributes: &dyn RtpsReaderProxyAttributes =
                            reliable_behavior.reader_proxy;
                        destined_submessages.push((reader_proxy_attributes, submessages));
                    }
                }
            }
        }

        destined_submessages
    }
}

#[cfg(test)]
mod tests {

    use rust_rtps_pim::{
        behavior::{
            types::{Duration, DURATION_ZERO},
            writer::{
                reader_proxy::RtpsReaderProxy,
                stateful_writer::{RtpsStatefulWriterConstructor, RtpsStatefulWriterOperations},
            },
        },
        structure::types::{ReliabilityKind, TopicKind, ENTITYID_UNKNOWN, GUID_UNKNOWN},
    };

    use crate::{
        dds_impl::publisher_impl, rtps_impl::rtps_stateful_writer_impl::RtpsStatefulWriterImpl,
    };

    use super::*;
    struct EmptyTimer;

    impl Timer for EmptyTimer {
        fn reset(&mut self) {
            unimplemented!()
        }

        fn elapsed(&self) -> std::time::Duration {
            unimplemented!()
        }
    }

    struct MockData(Vec<u8>);

    impl DdsSerialize for MockData {
        fn serialize<W: std::io::Write, R: crate::dds_type::Endianness>(
            &self,
            mut writer: W,
        ) -> DDSResult<()> {
            writer.write(&self.0).unwrap();
            Ok(())
        }
    }

    // struct MockCacheChange;

    // #[test]
    // fn write_w_timestamp() {
    //     struct MockWriterCache;

    //     impl RtpsHistoryCacheAddChange for MockWriterCache {
    //         type CacheChangeType = MockWriterCache;

    //         fn add_change(&mut self, _change: Self::CacheChangeType) {}
    //     }

    //     struct MockWriter {
    //         cache: MockWriterCache,
    //     }

    //     impl RtpsWriterOperations for MockWriter {
    //         type CacheChangeType = MockCacheChange;

    //         fn new_change(
    //             &mut self,
    //             kind: ChangeKind,
    //             data: <Self::CacheChangeType as RtpsCacheChangeConstructor>::DataType,
    //             inline_qos: <Self::CacheChangeType as RtpsCacheChangeConstructor>::ParameterListType,
    //             handle: InstanceHandle,
    //         ) -> Self::CacheChangeType {
    //             todo!()
    //         }

    //         // type CacheChangeType;

    //         // fn new_change(
    //         //     &mut self,
    //         //     kind: ChangeKind,
    //         //     data: D,
    //         //     inline_qos: P,
    //         //     handle: InstanceHandle,
    //         // ) -> RtpsCacheChange<P, D> {
    //         //     RtpsCacheChange {
    //         //         data_value: data,
    //         //         kind,
    //         //         instance_handle: handle,
    //         //         inline_qos,
    //         //         sequence_number: 1,
    //         //         writer_guid: GUID_UNKNOWN,
    //         //     }
    //         // }
    //     }

    //     let mut dds_data_writer = DataWriterImpl::new(
    //         DataWriterQos::default(),
    //         MockWriter {
    //             cache: MockWriterCache,
    //         },
    //         Box::new(EmptyTimer),
    //     );

    //     let data_value = MockData(vec![0, 1, 0, 0, 7, 3]);
    //     dds_data_writer
    //         .write_w_timestamp(
    //             &data_value,
    //             None,
    //             rust_dds_api::dcps_psm::Time { sec: 0, nanosec: 0 },
    //         )
    //         .unwrap();
    // }

    #[test]
    fn stateful_writer_heartbeat_send_timer() {
        struct MockTimer;

        impl Timer for MockTimer {
            fn reset(&mut self) {}

            fn elapsed(&self) -> std::time::Duration {
                std::time::Duration::new(5, 0)
            }
        }
        let guid = GUID_UNKNOWN;
        let topic_kind = TopicKind::WithKey;
        let reliability_level = ReliabilityKind::Reliable;
        let unicast_locator_list = &[];
        let multicast_locator_list = &[];
        let push_mode = true;
        let heartbeat_period = Duration::new(2, 0);
        let nack_response_delay = DURATION_ZERO;
        let nack_suppression_duration = DURATION_ZERO;
        let data_max_size_serialized = None;

        let mut rtps_writer_impl = RtpsStatefulWriterImpl::new(
            guid,
            topic_kind,
            reliability_level,
            unicast_locator_list,
            multicast_locator_list,
            push_mode,
            heartbeat_period,
            nack_response_delay,
            nack_suppression_duration,
            data_max_size_serialized,
        );
        let reader_proxy =
            RtpsReaderProxy::new(GUID_UNKNOWN, ENTITYID_UNKNOWN, vec![], vec![], false);

        rtps_writer_impl.matched_reader_add(reader_proxy);

        let mut data_writer_impl: DataWriterImpl<MockData, _, _> =
            DataWriterImpl::new(DataWriterQos::default(), rtps_writer_impl, MockTimer);

        let destined_submessages1 =
            publisher_impl::StatefulWriterSubmessageProducer::produce_submessages(
                &mut data_writer_impl,
            );
        let produced_submessages1 = &destined_submessages1[0].1;
        assert_eq!(produced_submessages1.len(), 1);
        if let RtpsSubmessageTypeWrite::Heartbeat(heartbeat_submessage) = &produced_submessages1[0]
        {
            assert_eq!(heartbeat_submessage.count.value, Count(1));
        } else {
            assert!(false, "Wrong submessage");
        }

        let destined_submessages2 = data_writer_impl.produce_submessages();
        let produced_submessages2 = &destined_submessages2[0].1;
        if let RtpsSubmessageTypeWrite::Heartbeat(heartbeat_submessage) = &produced_submessages2[0]
        {
            assert_eq!(heartbeat_submessage.count.value, Count(2));
        } else {
            assert!(false, "Wrong submessage");
        }
    }
}
