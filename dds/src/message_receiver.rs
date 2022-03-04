use rust_dds_rtps_implementation::{
    dds_impl::{data_reader_proxy::RtpsReader, subscriber_proxy::SubscriberAttributes},
    utils::shared_object::RtpsShared,
};
use rust_rtps_pim::{
    behavior::{stateful_reader_behavior::StatefulReaderBehavior, reader::writer_proxy::RtpsWriterProxyAttributes},
    messages::{
        submessage_elements::{
            EntityIdSubmessageElementAttributes, TimestampSubmessageElementAttributes,
        },
        submessages::{DataSubmessageAttributes, InfoTimestampSubmessageAttributes},
        types::{Time, TIME_INVALID},
    },
    structure::types::{
        GuidPrefix, Locator, ProtocolVersion, VendorId, ENTITYID_UNKNOWN, GUIDPREFIX_UNKNOWN,
        LOCATOR_ADDRESS_INVALID, LOCATOR_PORT_INVALID, PROTOCOLVERSION, VENDOR_ID_UNKNOWN, Guid,
    },
};
use rust_rtps_udp_psm::messages::{
    overall_structure::{RtpsMessageRead, RtpsSubmessageTypeRead},
    submessages::{AckNackSubmessageRead, DataSubmessageRead},
};

use crate::domain_participant_factory::RtpsStructureImpl;

pub struct MessageReceiver {
    source_version: ProtocolVersion,
    source_vendor_id: VendorId,
    source_guid_prefix: GuidPrefix,
    dest_guid_prefix: GuidPrefix,
    unicast_reply_locator_list: Vec<Locator>,
    multicast_reply_locator_list: Vec<Locator>,
    have_timestamp: bool,
    timestamp: Time,
}

impl MessageReceiver {
    pub fn new() -> Self {
        Self {
            source_version: PROTOCOLVERSION,
            source_vendor_id: VENDOR_ID_UNKNOWN,
            source_guid_prefix: GUIDPREFIX_UNKNOWN,
            dest_guid_prefix: GUIDPREFIX_UNKNOWN,
            unicast_reply_locator_list: Vec::new(),
            multicast_reply_locator_list: Vec::new(),
            have_timestamp: false,
            timestamp: TIME_INVALID,
        }
    }

    pub fn process_message<'a>(
        &mut self,
        participant_guid_prefix: GuidPrefix,
        list: &'a [RtpsShared<SubscriberAttributes<RtpsStructureImpl>>],
        source_locator: Locator,
        message: &'a RtpsMessageRead,
    ) {
        self.dest_guid_prefix = participant_guid_prefix;
        self.source_version = message.header.version;
        self.source_vendor_id = message.header.vendor_id;
        self.source_guid_prefix = message.header.guid_prefix;
        self.unicast_reply_locator_list.push(Locator::new(
            *source_locator.kind(),
            LOCATOR_PORT_INVALID,
            *source_locator.address(),
        ));
        self.multicast_reply_locator_list.push(Locator::new(
            *source_locator.kind(),
            LOCATOR_PORT_INVALID,
            LOCATOR_ADDRESS_INVALID,
        ));

        for submessage in &message.submessages {
            match submessage {
                RtpsSubmessageTypeRead::AckNack(_) => todo!(),
                RtpsSubmessageTypeRead::Data(data) => {
                    for subscriber in list {
                        let subscriber_lock = subscriber.read_lock();
                        for data_reader in &subscriber_lock.data_reader_list {
                            let mut data_reader_lock = data_reader.write_lock();

                            let mut status_changed = data_reader_lock.status_changed;

                            {
                                let rtps_reader = &mut data_reader_lock.rtps_reader;
                                match rtps_reader {
                                    RtpsReader::Stateless(stateless_rtps_reader) => {
                                        for mut stateless_reader_behavior in
                                            stateless_rtps_reader.into_iter()
                                        {
                                            if data.reader_id().value() == ENTITYID_UNKNOWN
                                                || data.reader_id().value()
                                                    == stateless_reader_behavior
                                                        .reader_guid
                                                        .entity_id()
                                            {
                                                stateless_reader_behavior
                                                    .receive_data(self.source_guid_prefix, data);
                                                status_changed = true;
                                            }
                                        }
                                    }
                                    RtpsReader::Stateful(stateful_rtps_reader) => {
                                        for stateful_reader_behavior in
                                            stateful_rtps_reader.behavior().into_iter()
                                        {
                                            match stateful_reader_behavior {
                                                StatefulReaderBehavior::BestEffort(_) => todo!(),
                                                StatefulReaderBehavior::Reliable(
                                                    mut reliable_stateful_reader,
                                                ) => {
                                                    let writer_guid = Guid::new(
                                                        self.source_guid_prefix,
                                                        data.writer_id().value(),
                                                    );
                                                    if writer_guid
                                                        == reliable_stateful_reader
                                                            .writer_proxy
                                                            .remote_writer_guid()
                                                    {
                                                        reliable_stateful_reader.receive_data(
                                                            self.source_guid_prefix,
                                                            data,
                                                        );
                                                        status_changed = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if data_reader_lock.status_changed != status_changed {
                                data_reader_lock
                                    .listener
                                    .as_ref()
                                    .map(|l| l.on_data_available());
                                data_reader_lock.status_changed = false;
                            }
                        }
                    }
                }
                RtpsSubmessageTypeRead::DataFrag(_) => todo!(),
                RtpsSubmessageTypeRead::Gap(_) => todo!(),
                RtpsSubmessageTypeRead::Heartbeat(_) => (),
                RtpsSubmessageTypeRead::HeartbeatFrag(_) => todo!(),
                RtpsSubmessageTypeRead::InfoDestination(_) => todo!(),
                RtpsSubmessageTypeRead::InfoReply(_) => todo!(),
                RtpsSubmessageTypeRead::InfoSource(_) => todo!(),
                RtpsSubmessageTypeRead::InfoTimestamp(info_timestamp) => {
                    self.process_info_timestamp_submessage(info_timestamp)
                }
                RtpsSubmessageTypeRead::NackFrag(_) => todo!(),
                RtpsSubmessageTypeRead::Pad(_) => todo!(),
            }
        }
    }

    fn process_info_timestamp_submessage(
        &mut self,
        info_timestamp: &impl InfoTimestampSubmessageAttributes<
            TimestampSubmessageElementType = impl TimestampSubmessageElementAttributes,
        >,
    ) {
        if info_timestamp.invalidate_flag() == false {
            self.have_timestamp = true;
            self.timestamp = info_timestamp.timestamp().value();
        } else {
            self.have_timestamp = false;
            self.timestamp = TIME_INVALID;
        }
    }
}

pub trait ProcessDataSubmessage {
    fn process_data_submessage(
        &mut self,
        source_guid_prefix: GuidPrefix,
        _data: &DataSubmessageRead,
    );
}

pub trait ProcessAckNackSubmessage {
    fn process_acknack_submessage(
        &self,
        source_guid_prefix: GuidPrefix,
        _acknack: &AckNackSubmessageRead,
    );
}

#[cfg(test)]
mod tests {

    use rust_rtps_udp_psm::messages::{
        submessage_elements::TimestampSubmessageElementPsm,
        submessages::InfoTimestampSubmessageRead,
    };

    use super::*;

    #[test]
    fn process_info_timestamp_submessage_valid_time() {
        let mut message_receiver = MessageReceiver::new();
        let info_timestamp = InfoTimestampSubmessageRead::new(
            true,
            false,
            TimestampSubmessageElementPsm { value: Time(100) },
        );
        message_receiver.process_info_timestamp_submessage(&info_timestamp);

        assert_eq!(message_receiver.have_timestamp, true);
        assert_eq!(message_receiver.timestamp, Time(100));
    }

    #[test]
    fn process_info_timestamp_submessage_invalid_time() {
        let mut message_receiver = MessageReceiver::new();
        let info_timestamp = InfoTimestampSubmessageRead::new(
            true,
            true,
            TimestampSubmessageElementPsm { value: Time(100) },
        );
        message_receiver.process_info_timestamp_submessage(&info_timestamp);

        assert_eq!(message_receiver.have_timestamp, false);
        assert_eq!(message_receiver.timestamp, TIME_INVALID);
    }

    // #[test]
    // fn process_data() {
    //     struct MockProcessDataSubmessage {
    //         called: RefCell<bool>,
    //     }

    //     impl ProcessDataSubmessage for MockProcessDataSubmessage {
    //         fn process_data_submessage(
    //             &mut self,
    //             _source_guid_prefix: GuidPrefix,
    //             _data: &DataSubmessageRead,
    //         ) {
    //             *self.called.borrow_mut() = true
    //         }
    //     }

    //     let data_submessage = DataSubmessageRead::new(
    //         true,
    //         false,
    //         true,
    //         false,
    //         false,
    //         EntityIdSubmessageElement {
    //             value: EntityId::new([1; 3], BUILT_IN_READER_WITH_KEY),
    //         },
    //         EntityIdSubmessageElement {
    //             value: EntityId::new([1; 3], BUILT_IN_WRITER_WITH_KEY),
    //         },
    //         SequenceNumberSubmessageElement { value: 1 },
    //         ParameterListSubmessageElement { parameter: vec![] },
    //         SerializedDataSubmessageElement {
    //             value: &[1, 2, 3][..],
    //         },
    //     );
    //     let participant_guid_prefix = GuidPrefix([1; 12]);
    //     let reader_group_list = vec![rtps_shared_new(MockProcessDataSubmessage {
    //         called: RefCell::new(false),
    //     })];
    //     let source_locator = Locator::new(1, 7400, [1; 16]);
    //     let header = RtpsMessageHeader {
    //         protocol: ProtocolId::PROTOCOL_RTPS,
    //         version: PROTOCOLVERSION_2_4,
    //         vendor_id: [99, 99],
    //         guid_prefix: GuidPrefix([1; 12]),
    //     };
    //     let submessages = vec![RtpsSubmessageTypeRead::Data(data_submessage)];
    //     let message = RtpsMessageRead::new(header, submessages);

    //     MessageReceiver::new().process_message(
    //         participant_guid_prefix,
    //         &reader_group_list,
    //         source_locator,
    //         &message,
    //     );

    //     assert_eq!(
    //         *rtps_shared_read_lock(&reader_group_list[0]).called.borrow(),
    //         true
    //     );
    // }
}
