use std::cell::RefCell;

use rust_dds_rtps_implementation::{
    dds_impl::publisher_impl::PublisherImpl,
    utils::{
        message_receiver::{MessageReceiver, ProcessDataSubmessage},
        shared_object::RtpsShared,
        transport::{TransportRead, TransportWrite},
    },
};
use rust_rtps_pim::{
    behavior::{
        stateful_writer_behavior::StatefulWriterBehavior,
        stateless_writer_behavior::StatelessWriterBehavior,
        writer::{
            reader_locator::RtpsReaderLocatorAttributes, reader_proxy::RtpsReaderProxyAttributes,
        },
    },
    messages::overall_structure::RtpsMessageHeader,
    structure::types::{
        GuidPrefix, ProtocolVersion, VendorId, GUIDPREFIX_UNKNOWN, PROTOCOLVERSION, VENDOR_ID_S2E,
    },
};
use rust_rtps_psm::messages::{
    overall_structure::{RtpsMessageWrite, RtpsSubmessageTypeWrite},
    submessages::DataSubmessageWrite,
};

pub struct Communication<T> {
    pub version: ProtocolVersion,
    pub vendor_id: VendorId,
    pub guid_prefix: GuidPrefix,
    pub transport: T,
}

impl<T> Communication<T>
where
    T: TransportWrite,
{
    pub fn send(&mut self, list: &[RtpsShared<PublisherImpl>]) {
        for publisher in list {
            let publisher_lock = publisher.write().unwrap();

            let message_header = RtpsMessageHeader {
                protocol: rust_rtps_pim::messages::types::ProtocolId::PROTOCOL_RTPS,
                version: PROTOCOLVERSION,
                vendor_id: VENDOR_ID_S2E,
                guid_prefix: GUIDPREFIX_UNKNOWN,
            };

            for stateless_writer in publisher_lock.iter_stateless_writer_list() {
                let rtps_stateless_writer_arc_lock =
                    stateless_writer.into_as_mut_stateless_writer();
                let mut rtps_stateless_writer_lock =
                    rtps_stateless_writer_arc_lock.write().unwrap();
                let mut destined_submessages = Vec::new();

                for behavior in rtps_stateless_writer_lock.as_mut() {
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
                                destined_submessages.push((
                                    best_effort_behavior.reader_locator.locator(),
                                    submessages,
                                ));
                            }
                        }
                        StatelessWriterBehavior::Reliable(_) => todo!(),
                    };
                }

                for (locator, submessage) in destined_submessages {
                    let message = RtpsMessageWrite::new(message_header.clone(), submessage);
                    self.transport.write(&message, locator);
                }
            }

            for stateful_writer in publisher_lock.iter_stateful_writer_list() {
                let rtps_stateful_writer_arc_lock = stateful_writer.into_as_mut_stateful_writer();
                let mut rtps_stateful_writer_lock = rtps_stateful_writer_arc_lock.write().unwrap();

                let mut destined_submessages = Vec::new();

                for behavior in rtps_stateful_writer_lock.as_mut() {
                    match behavior {
                        StatefulWriterBehavior::BestEffort(_) => todo!(),
                        StatefulWriterBehavior::Reliable(mut reliable_behavior) => {
                            let submessages = RefCell::new(Vec::new());
                            reliable_behavior.send_heartbeat(&mut |heartbeat| {
                                submessages
                                    .borrow_mut()
                                    .push(RtpsSubmessageTypeWrite::Heartbeat(heartbeat));
                            });

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

                for (reader_proxy, submessage) in destined_submessages {
                    let message = RtpsMessageWrite::new(message_header.clone(), submessage);
                    self.transport
                        .write(&message, &reader_proxy.unicast_locator_list()[0]);
                }
            }
        }
    }
}
impl<T> Communication<T>
where
    T: TransportRead,
{
    pub fn receive(&mut self, list: &[RtpsShared<impl ProcessDataSubmessage>]) {
        if let Some((source_locator, message)) = self.transport.read() {
            MessageReceiver::new().process_message(
                self.guid_prefix,
                list,
                source_locator,
                &message,
            );
        }
    }
}
