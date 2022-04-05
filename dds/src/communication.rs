use std::{cell::RefCell, ops::DerefMut};

use dds_implementation::{
    dds_impl::{
        data_reader_proxy::RtpsReader, data_writer_proxy::RtpsWriter,
        publisher_proxy::PublisherAttributes, subscriber_proxy::SubscriberAttributes,
    },
    utils::shared_object::DdsShared,
};

use rtps_pim::{
    behavior::{
        reader::writer_proxy::RtpsWriterProxyAttributes,
        stateful_writer_behavior::StatefulWriterSendSubmessages,
        stateless_writer_behavior::StatelessWriterSendSubmessages,
        writer::{
            reader_locator::RtpsReaderLocatorAttributes, reader_proxy::RtpsReaderProxyAttributes,
        },
    },
    messages::{
        overall_structure::{RtpsMessage, RtpsMessageHeader, RtpsSubmessageType},
        submessage_elements::{Parameter, TimestampSubmessageElement},
        submessages::InfoTimestampSubmessage,
        types::{FragmentNumber, TIME_INVALID},
    },
    structure::{
        entity::RtpsEntityAttributes,
        types::{
            GuidPrefix, Locator, ProtocolVersion, SequenceNumber, VendorId, PROTOCOLVERSION,
            VENDOR_ID_S2E,
        },
    },
    transport::{TransportRead, TransportWrite},
};

use crate::{domain_participant_factory::RtpsStructureImpl, message_receiver::MessageReceiver};

pub struct Communication<T> {
    pub version: ProtocolVersion,
    pub vendor_id: VendorId,
    pub guid_prefix: GuidPrefix,
    pub transport: T,
}

impl<T> Communication<T>
where
    T: for<'a> TransportWrite<
        Vec<
            RtpsSubmessageType<
                Vec<SequenceNumber>,
                Vec<Parameter<'a>>,
                &'a [u8],
                Vec<Locator>,
                Vec<FragmentNumber>,
            >,
        >,
    >,
{
    pub fn send_publisher_message(&mut self, publisher: &PublisherAttributes<RtpsStructureImpl>) {
        let message_header = RtpsMessageHeader {
            protocol: rtps_pim::messages::types::ProtocolId::PROTOCOL_RTPS,
            version: PROTOCOLVERSION,
            vendor_id: VENDOR_ID_S2E,
            guid_prefix: publisher.rtps_group.entity.guid.prefix(),
        };

        for any_data_writer in publisher.data_writer_list.write_lock().iter_mut() {
            let mut rtps_writer = any_data_writer.rtps_writer.write_lock();

            match rtps_writer.deref_mut() {
                RtpsWriter::Stateless(stateless_rtps_writer) => {
                    let message_header = RtpsMessageHeader {
                        guid_prefix: stateless_rtps_writer.writer.endpoint.entity.guid.prefix,
                        ..message_header.clone()
                    };

                    let destined_submessages = RefCell::new(Vec::new());
                    stateless_rtps_writer.send_submessages(
                        |reader_locator, data| {
                            let info_ts = if let Some(time) = any_data_writer
                                .sample_info
                                .read_lock()
                                .get(&data.writer_sn.value)
                            {
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
                                reader_locator.locator(),
                                vec![RtpsSubmessageType::InfoTimestamp(info_ts)],
                            ));
                            destined_submessages.borrow_mut().push((
                                reader_locator.locator(),
                                vec![RtpsSubmessageType::Data(data)],
                            ));
                        },
                        |reader_locator, gap| {
                            destined_submessages.borrow_mut().push((
                                reader_locator.locator(),
                                vec![RtpsSubmessageType::Gap(gap)],
                            ));
                        },
                        |_, _| (),
                    );

                    for (locator, submessages) in destined_submessages.take() {
                        self.transport.write(
                            &RtpsMessage {
                                header: message_header.clone(),
                                submessages,
                            },
                            locator,
                        );
                    }
                }
                RtpsWriter::Stateful(stateful_rtps_writer) => {
                    let message_header = RtpsMessageHeader {
                        guid_prefix: stateful_rtps_writer.writer.endpoint.entity.guid.prefix,
                        ..message_header.clone()
                    };

                    let destined_submessages = RefCell::new(Vec::new());
                    stateful_rtps_writer.send_submessages(
                        |reader_proxy, data| {
                            let info_ts = if let Some(time) = any_data_writer
                                .sample_info
                                .read_lock()
                                .get(&data.writer_sn.value)
                            {
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
                                reader_proxy.unicast_locator_list()[0],
                                vec![RtpsSubmessageType::InfoTimestamp(info_ts)],
                            ));
                            destined_submessages.borrow_mut().push((
                                reader_proxy.unicast_locator_list()[0],
                                vec![RtpsSubmessageType::Data(data)],
                            ));
                        },
                        |reader_proxy, gap| {
                            destined_submessages.borrow_mut().push((
                                reader_proxy.unicast_locator_list()[0],
                                vec![RtpsSubmessageType::Gap(gap)],
                            ));
                        },
                        |reader_proxy, heartbeat| {
                            destined_submessages.borrow_mut().push((
                                reader_proxy.unicast_locator_list()[0],
                                vec![RtpsSubmessageType::Heartbeat(heartbeat)],
                            ));
                        },
                    );
                    for (locator, submessages) in destined_submessages.take() {
                        self.transport.write(
                            &RtpsMessage {
                                header: message_header.clone(),
                                submessages,
                            },
                            locator,
                        );
                    }
                }
            }
        }
    }

    pub fn send_subscriber_message(
        &mut self,
        subscriber: &SubscriberAttributes<RtpsStructureImpl>,
    ) {
        for any_data_reader in subscriber.data_reader_list.write_lock().iter_mut() {
            if let RtpsReader::Stateful(stateful_rtps_reader) =
                any_data_reader.rtps_reader.write_lock().deref_mut()
            {
                let message_header = RtpsMessageHeader {
                    protocol: rtps_pim::messages::types::ProtocolId::PROTOCOL_RTPS,
                    version: PROTOCOLVERSION,
                    vendor_id: VENDOR_ID_S2E,
                    guid_prefix: stateful_rtps_reader.guid().prefix,
                };

                for (writer_proxy, acknacks) in stateful_rtps_reader.produce_acknack_submessages() {
                    let message = RtpsMessage {
                        header: message_header.clone(),
                        submessages: acknacks
                            .into_iter()
                            .map(|acknack| RtpsSubmessageType::AckNack(acknack))
                            .collect(),
                    };

                    for &locator in writer_proxy.unicast_locator_list() {
                        self.transport.write(&message, locator);
                    }
                }
            }
        }
    }
}

impl<T> Communication<T>
where
    T: for<'a> TransportRead<
        'a,
        Vec<
            RtpsSubmessageType<
                Vec<SequenceNumber>,
                Vec<Parameter<'a>>,
                &'a [u8],
                Vec<Locator>,
                Vec<FragmentNumber>,
            >,
        >,
    >,
{
    pub fn receive(
        &mut self,
        publisher_list: &[DdsShared<PublisherAttributes<RtpsStructureImpl>>],
        subscriber_list: &[DdsShared<SubscriberAttributes<RtpsStructureImpl>>],
    ) {
        while let Some((source_locator, message)) = self.transport.read() {
            MessageReceiver::new().process_message(
                self.guid_prefix,
                publisher_list,
                subscriber_list,
                source_locator,
                &message,
            );
        }
    }
}
