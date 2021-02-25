// use crate::{
//     behavior::{
//         stateless_writer::ReaderLocator,
//         types::{
//             constants::{DURATION_INFINITE, DURATION_ZERO},
//             Duration,
//         },
//         StatelessReader, StatelessWriter,
//     },
//     types::{
//         constants::{
//             ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER, ENTITYID_SPDP_BUILTIN_PARTICIPANT_DETECTOR,
//         },
//         GuidPrefix, Locator, ReliabilityKind, TopicKind, GUID,
//     },
// };

// pub struct SPDPbuiltinParticipantWriter;

// impl SPDPbuiltinParticipantWriter {
//     pub fn new(
//         guid_prefix: GuidPrefix,
//         unicast_locator_list: Vec<Locator>,
//         multicast_locator_list: Vec<Locator>,
//         _resend_period: Duration,
//         reader_locator: Vec<ReaderLocator>,
//     ) -> StatelessWriter {
//         let guid = GUID::new(guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER);
//         let topic_kind = TopicKind::WithKey;
//         let reliability_level = ReliabilityKind::BestEffort;

//         // These values are unspecified in the standard and not used for the
//         // stateless writer anyway
//         let push_mode = true;
//         let heartbeat_period = DURATION_INFINITE;
//         let nack_response_delay = DURATION_ZERO;
//         let nack_suppression_duration = DURATION_ZERO;
//         let data_max_sized_serialized = None;

//         let mut spdp_builtin_participant_writer = StatelessWriter::new();

//         let mut spdp_builtin_participant_writer = StatelessWriter::new(
//             guid,
//             unicast_locator_list,
//             multicast_locator_list,
//             topic_kind,
//             reliability_level,
//             push_mode,
//             heartbeat_period,
//             nack_response_delay,
//             nack_suppression_duration,
//             data_max_sized_serialized,
//         );

//         for locator in reader_locator{
//             spdp_builtin_participant_writer.reader_locator_add(locator);
//         }

//         spdp_builtin_participant_writer
//     }
// }

// pub struct SPDPbuiltinParticipantReader;

// impl SPDPbuiltinParticipantReader {
//     pub fn new(
//         guid_prefix: GuidPrefix,
//         unicast_locator_list: Vec<Locator>,
//         multicast_locator_list: Vec<Locator>,
//     ) -> StatelessReader {
//         let guid = GUID::new(guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_DETECTOR);
//         let topic_kind = TopicKind::WithKey;
//         let reliability_level = ReliabilityKind::BestEffort;

//         let expects_inline_qos = false;
//         let heartbeat_response_delay = DURATION_ZERO;
//         let heartbeat_supression_duration = DURATION_ZERO;

//         let spdp_builtin_participant_reader = StatelessReader::new(
//             guid,
//             unicast_locator_list,
//             multicast_locator_list,
//             topic_kind,
//             reliability_level,
//             expects_inline_qos,
//             heartbeat_response_delay,
//             heartbeat_supression_duration,
//         );

//         spdp_builtin_participant_reader
//     }
// }
