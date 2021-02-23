use std::ops::{Deref, DerefMut};

use rust_dds_api::{
    dcps_psm::StatusMask, dds_type::DDSType, infrastructure::qos::DataWriterQos,
    publication::data_writer_listener::DataWriterListener,
};
use rust_rtps::behavior::{StatefulWriter, StatelessWriter, Writer};

use super::mask_listener::MaskListener;
struct RtpsDataWriterListener<T: DDSType>(Box<dyn DataWriterListener<DataType = T>>);
trait AnyDataWriterListener: Send + Sync {}

impl<T: DDSType> AnyDataWriterListener for RtpsDataWriterListener<T> {}

pub enum RtpsWriterFlavor {
    Stateful(StatefulWriter),
    Stateless(StatelessWriter),
}

impl Deref for RtpsWriterFlavor {
    type Target = Writer;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Stateful(stateful_writer) => stateful_writer,
            Self::Stateless(stateless_writer) => stateless_writer,
        }
    }
}

impl DerefMut for RtpsWriterFlavor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Stateful(stateful_writer) => stateful_writer,
            Self::Stateless(stateless_writer) => stateless_writer,
        }
    }
}

pub struct RtpsDataWriterImpl {
    rtps_writer_flavor: RtpsWriterFlavor,
    qos: DataWriterQos,
    mask_listener: MaskListener<Box<dyn AnyDataWriterListener>>,
}

impl RtpsDataWriterImpl {
    pub fn new<T: DDSType>(
        rtps_writer_flavor: RtpsWriterFlavor,
        qos: DataWriterQos,
        listener: Option<Box<dyn DataWriterListener<DataType = T>>>,
        status_mask: StatusMask,
    ) -> Self {
        let listener: Option<Box<dyn AnyDataWriterListener>> = match listener {
            Some(listener) => Some(Box::new(RtpsDataWriterListener(listener))),
            None => None,
        };
        let mask_listener = MaskListener::new(listener, status_mask);
        Self {
            rtps_writer_flavor,
            qos,
            mask_listener,
        }
    }
}

// pub type RtpsAnyDataWriterImplRef<'a> = MaybeValidRef<'a, RtpsDataWriterImpl>;

// impl<'a> RtpsAnyDataWriterImplRef<'a> {
//     // fn get(&self) -> DDSResult<MutexGuard<RtpsDataWriterInner>> {
//     //     Ok(MaybeValid::get(self)
//     //         .ok_or(DDSError::AlreadyDeleted)?
//     //         .0
//     //         .lock()
//     //         .unwrap())
//     // }

//     // pub fn delete(&self) -> DDSResult<()> {
//     //     self.get()?.topic.take(); // Drop the topic
//     //     MaybeValid::delete(self);
//     //     Ok(())
//     // }

//     // pub fn write_w_timestamp<T: DDSType>(
//     //     &self,
//     //     data: T,
//     //     _handle: Option<InstanceHandle>,
//     //     _timestamp: Time,
//     // ) -> DDSResult<()> {
//     //     let mut this = self.get()?;
//     //     let kind = ChangeKind::Alive;
//     //     let inline_qos = None;
//     //     let change = this.rtps_writer_flavor.new_change(
//     //         kind,
//     //         Some(data.serialize()),
//     //         inline_qos,
//     //         instance_handle_from_dds_type(data),
//     //     );
//     //     this.rtps_writer_flavor.writer_cache.add_change(change);

//     //     Ok(())
//     // }

//     // pub fn get_qos(&self) -> DDSResult<DataWriterQos> {
//     //     Ok(self.get()?.qos.clone())
//     // }

//     // pub fn set_qos(&self, qos: Option<DataWriterQos>) -> DDSResult<()> {
//     //     let qos = qos.unwrap_or_default();
//     //     qos.is_consistent()?;
//     //     self.get()?.qos = qos;
//     //     Ok(())
//     // }

//     // pub fn produce_messages(&self) -> Vec<DestinedMessages> {
//     //     let mut output = Vec::new();
//     //     if let Some(mut rtps_data_writer_inner) = self.get().ok() {
//     //         match &mut rtps_data_writer_inner.rtps_writer_flavor {
//     //             RtpsWriterFlavor::Stateful(stateful_writer) => {
//     //                 let matched_readers = &mut stateful_writer.matched_readers;
//     //                 let writer = &stateful_writer.writer;
//     //                 for reader_proxy in matched_readers.iter_mut() {
//     //                     let messages = match writer.endpoint.reliability_level {
//     //                         ReliabilityKind::BestEffort => {
//     //                             BestEffortReaderProxyBehavior::produce_messages(
//     //                                 reader_proxy,
//     //                                 &writer.writer_cache,
//     //                                 writer.endpoint.entity.guid.entity_id(),
//     //                                 writer.last_change_sequence_number,
//     //                             )
//     //                         }
//     //                         ReliabilityKind::Reliable => {
//     //                             ReliableReaderProxyBehavior::produce_messages(
//     //                                 reader_proxy,
//     //                                 &writer.writer_cache,
//     //                                 writer.endpoint.entity.guid.entity_id(),
//     //                                 writer.last_change_sequence_number,
//     //                                 writer.heartbeat_period,
//     //                                 writer.nack_response_delay,
//     //                             )
//     //                         }
//     //                     };
//     //                     if !messages.is_empty() {
//     //                         output.push(DestinedMessages::MultiDestination {
//     //                             unicast_locator_list: reader_proxy.unicast_locator_list.clone(),
//     //                             multicast_locator_list: reader_proxy.multicast_locator_list.clone(),
//     //                             messages,
//     //                         });
//     //                     }
//     //                 }
//     //             }
//     //             RtpsWriterFlavor::Stateless(stateless_writer) => {
//     //                 let reader_locators = &mut stateless_writer.reader_locators;
//     //                 let writer = &stateless_writer.writer;
//     //                 for reader_locator in reader_locators.iter_mut() {
//     //                     let messages = BestEffortReaderLocatorBehavior::produce_messages(
//     //                         reader_locator,
//     //                         &writer.writer_cache,
//     //                         writer.endpoint.entity.guid.entity_id(),
//     //                         writer.last_change_sequence_number,
//     //                     );
//     //                     if !messages.is_empty() {
//     //                         let locator = reader_locator.locator;
//     //                         output.push(DestinedMessages::SingleDestination { locator, messages });
//     //                     }
//     //                 }
//     //             }
//     //         }
//     //     }
//     //     output
//     // }

//     pub fn try_receive_message(&self, _message: u8) {
//         todo!()
//         // let this = self.get().ok();
//         // if let Some(mut rtps_writer) = this {
//         //     match &mut *rtps_writer {
//         //         RtpsDataWriterFlavor::Stateful(stateful_writer) => {
//         //             stateful_writer.try_receive_message()
//         //         }
//         //         _ => (),
//         //     }
//         // }
//     }
// }

// pub struct RtpsDataWriterImplPhantom<T> {
//     phantom_data: PhantomData<T>,
// }
