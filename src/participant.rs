use crate::stateless_writer::StatelessWriter;
use crate::stateless_reader::StatelessReader;
use crate::stateful_writer::{StatefulWriter, ReaderProxy};
use crate::stateful_reader::{StatefulReader, WriterProxy};
use crate::types::{GUID, GuidPrefix, Locator, ProtocolVersion, VendorId, TopicKind, ChangeKind, ReliabilityKind};
use crate::types::constants::{
    ENTITYID_PARTICIPANT,
    ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER,
    ENTITYID_SPDP_BUILTIN_PARTICIPANT_DETECTOR,
    ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
    ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
    ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
    ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
    ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
    ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
    LOCATOR_KIND_UDPv4};
use crate::endpoint_types::BuiltInEndpointSet;
use crate::messages::Endianness;
use crate::behavior::types::Duration;
use crate::behavior::types::constants::DURATION_ZERO;
use crate::spdp::SPDPdiscoveredParticipantData;
use crate::transport::{Transport, UdpTransport};
use crate::messages::message_sender::rtps_message_sender;
use crate::messages::message_receiver::rtps_message_receiver;
use crate::endpoint_types::DomainId;


pub struct Participant<T: Transport> {
    guid: GUID,
    domain_id: DomainId,
    default_unicast_locator_list: Vec<Locator>,
    default_multicast_locator_list: Vec<Locator>,
    metatraffic_unicast_locator_list: Vec<Locator>,
    metatraffic_multicast_locator_list: Vec<Locator>,
    protocol_version: ProtocolVersion,
    vendor_id: VendorId,
    domain_tag: String,
    metatraffic_transport: T,
    spdp_builtin_participant_reader: StatelessReader,
    spdp_builtin_participant_writer: StatelessWriter,
    builtin_endpoint_set: BuiltInEndpointSet,
    sedp_builtin_publications_reader: StatefulReader,
    sedp_builtin_publications_writer: StatefulWriter,
    sedp_builtin_subscriptions_reader: StatefulReader,
    sedp_builtin_subscriptions_writer: StatefulWriter,
    sedp_builtin_topics_reader: StatefulReader,
    sedp_builtin_topics_writer: StatefulWriter,
}

impl<T: Transport> Participant<T> {
    fn new(
        default_unicast_locator_list: Vec<Locator>,
        default_multicast_locator_list: Vec<Locator>,
        protocol_version: ProtocolVersion,
        vendor_id: VendorId,
    ) -> Self {
        let domain_id = 0; // TODO: Should be configurable
        let lease_duration = Duration::from_secs(100); // TODO: Should be configurable
        let endianness = Endianness::LittleEndian; // TODO: Should be configurable
        let expects_inline_qos = false;
        const PB : u32 = 7400;  // TODO: Should be configurable
        const DG : u32 = 250;   // TODO: Should be configurable
        const PG : u32 = 2; // TODO: Should be configurable
        const D0 : u32 = 0; // TODO: Should be configurable
        const D1 : u32 = 10;    // TODO: Should be configurable
        const D2 : u32 = 1; // TODO: Should be configurable
        const D3 : u32 = 11;    // TODO: Should be configurable

        let guid_prefix = [5, 6, 7, 8, 9, 5, 1, 2, 3, 4, 10, 11];   // TODO: Should be uniquely generated

        let spdp_well_known_multicast_port = PB + DG * domain_id + D0;

        let metatraffic_unicast_locator = Locator::new(
            LOCATOR_KIND_UDPv4,
            spdp_well_known_multicast_port,
            crate::transport::get_interface_address(&"Ethernet").unwrap(),
        );

        let metatraffic_multicast_locator = Locator::new(
            LOCATOR_KIND_UDPv4,
            spdp_well_known_multicast_port,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239, 255, 0, 1],
        );

        let metatraffic_transport = T::new(metatraffic_unicast_locator, Some(metatraffic_multicast_locator)).unwrap();

        let spdp_builtin_participant_writer = StatelessWriter::new(
            GUID::new(guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            TopicKind::WithKey);
        spdp_builtin_participant_writer.reader_locator_add(metatraffic_multicast_locator);

        let spdp_builtin_participant_reader = StatelessReader::new(
            GUID::new(guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_DETECTOR),
            TopicKind::WithKey,
            vec![],
            vec![metatraffic_multicast_locator],
            expects_inline_qos,
        );
        
        

        let expects_inline_qos = false;
        let heartbeat_period = Duration::from_secs(5);
        let heartbeat_response_delay = Duration::from_millis(500);
        let nack_response_delay = DURATION_ZERO;
        let nack_supression_duration = DURATION_ZERO;


        let sedp_builtin_publications_reader = StatefulReader::new(
            GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR),
            TopicKind::WithKey,
            ReliabilityKind::Reliable,
            expects_inline_qos,
            heartbeat_response_delay,
        );

        let sedp_builtin_publications_writer = StatefulWriter::new(
            GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER),
            TopicKind::WithKey,
            ReliabilityKind::Reliable,
            true,
            heartbeat_period,
            nack_response_delay,
            nack_supression_duration
        );

        let sedp_builtin_subscriptions_reader = StatefulReader::new(
            GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR),
            TopicKind::WithKey,
            ReliabilityKind::Reliable,
            expects_inline_qos,
            heartbeat_response_delay,
        );

        let sedp_builtin_subscriptions_writer = StatefulWriter::new(
            GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER),
            TopicKind::WithKey,
            ReliabilityKind::Reliable,
            true,
            heartbeat_period,
            nack_response_delay,
            nack_supression_duration
        );
        
        let sedp_builtin_topics_reader = StatefulReader::new(
            GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR),
            TopicKind::WithKey,
            ReliabilityKind::Reliable,
            expects_inline_qos,
            heartbeat_response_delay,
        );

        let sedp_builtin_topics_writer = StatefulWriter::new(
            GUID::new(guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER),
            TopicKind::WithKey,
            ReliabilityKind::Reliable,
            true,
            heartbeat_period,
            nack_response_delay,
            nack_supression_duration
        );

        let builtin_endpoint_set = BuiltInEndpointSet::new(
            BuiltInEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_ANNOUNCER |
            BuiltInEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR |
            BuiltInEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER |
            BuiltInEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR |
            BuiltInEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER |
            BuiltInEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR |
            BuiltInEndpointSet::BUILTIN_ENDPOINT_TOPICS_ANNOUNCER |
            BuiltInEndpointSet::BUILTIN_ENDPOINT_TOPICS_DETECTOR
        );

        // Fill up the metatraffic locator lists. By default only the SPDP will
        // use the multicast and the remaining built-in endpoints will communicate
        // over unicast.
        let metatraffic_unicast_locator_list = vec![metatraffic_unicast_locator];
        let metatraffic_multicast_locator_list = vec![];

        let participant = Self {
            guid: GUID::new(guid_prefix,ENTITYID_PARTICIPANT ),
            domain_id,
            default_unicast_locator_list,
            default_multicast_locator_list,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            protocol_version,
            vendor_id,
            domain_tag: "".to_string(),
            metatraffic_transport,
            builtin_endpoint_set,
            spdp_builtin_participant_reader,
            spdp_builtin_participant_writer,
            sedp_builtin_publications_reader,
            sedp_builtin_publications_writer,
            sedp_builtin_subscriptions_reader,
            sedp_builtin_subscriptions_writer,
            sedp_builtin_topics_reader,
            sedp_builtin_topics_writer,
        };

        let spdp_discovered_data = SPDPdiscoveredParticipantData::new_from_participant(&participant, lease_duration);
        let spdp_change = participant.spdp_builtin_participant_writer.new_change(ChangeKind::Alive,Some(spdp_discovered_data.data(endianness)) , None, spdp_discovered_data.key());
        participant.spdp_builtin_participant_writer.writer_cache().add_change(spdp_change);
        
        participant
    }

    pub fn guid(&self) -> GUID {
        self.guid
    }

    pub fn domain_id(&self) -> DomainId {
        self.domain_id
    }

    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub fn vendor_id(&self) -> VendorId {
        self.vendor_id
    }

    pub fn default_unicast_locator_list(&self) -> &Vec<Locator> {
        &self.default_unicast_locator_list
    }

    pub fn default_multicast_locator_list(&self) -> &Vec<Locator> {
        &self.default_multicast_locator_list
    }

    pub fn metatraffic_unicast_locator_list(&self) -> &Vec<Locator> {
        &self.metatraffic_unicast_locator_list
    }

    pub fn metatraffic_multicast_locator_list(&self) -> &Vec<Locator> {
        &self.metatraffic_multicast_locator_list
    }

    pub fn builtin_endpoint_set(&self) -> BuiltInEndpointSet {
        self.builtin_endpoint_set
    }

    pub fn domain_tag(&self) -> &String {
        &self.domain_tag
    }

    fn run(&self) {
        rtps_message_receiver(
            &self.metatraffic_transport, 
            self.guid.prefix(), 
            &[&self.spdp_builtin_participant_reader],
        &[&self.sedp_builtin_publications_reader, &self.sedp_builtin_subscriptions_reader, &self.sedp_builtin_topics_reader]);
        self.spdp_builtin_participant_reader.run();
        self.sedp_builtin_publications_reader.run();
        self.sedp_builtin_subscriptions_reader.run();
        self.sedp_builtin_topics_reader.run();

        self.spdp_builtin_participant_writer.run();
        self.sedp_builtin_publications_writer.run();
        self.sedp_builtin_subscriptions_writer.run();
        self.sedp_builtin_topics_writer.run();
        rtps_message_sender(&self.metatraffic_transport, self.guid.prefix(), &[&self.spdp_builtin_participant_writer],
    &[&self.sedp_builtin_publications_writer, &self.sedp_builtin_subscriptions_writer, &self.sedp_builtin_topics_writer]);
    }

    
    fn add_discovered_participant(&self, discovered_participant: &SPDPdiscoveredParticipantData) {
        // Implements the process described in
        // 8.5.5.1 Discovery of a new remote Participant

        if discovered_participant.domain_id() != self.domain_id {
            return;
        }

        if discovered_participant.domain_tag() != &self.domain_tag {
            return;
        }

        if discovered_participant.available_built_in_endpoints().has(BuiltInEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR) {
            let guid = GUID::new(discovered_participant.guid_prefix(), ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR);
            let proxy = ReaderProxy::new(
                guid,
                discovered_participant.metatraffic_unicast_locator_list().clone(),
            discovered_participant.metatraffic_multicast_locator_list().clone(),
        discovered_participant.expects_inline_qos(),
    true );
            self.sedp_builtin_publications_writer.matched_reader_add(proxy);
        }

        if discovered_participant.available_built_in_endpoints().has(BuiltInEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER) {
            let guid = GUID::new(discovered_participant.guid_prefix(), ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER);
            let proxy = WriterProxy::new(
                guid,
                discovered_participant.metatraffic_unicast_locator_list().clone(), 
                discovered_participant.metatraffic_multicast_locator_list().clone());
            self.sedp_builtin_publications_reader.matched_writer_add(proxy);
        }

        if discovered_participant.available_built_in_endpoints().has(BuiltInEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR) {
            let guid = GUID::new(discovered_participant.guid_prefix(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
            let proxy = ReaderProxy::new(
                guid,
                discovered_participant.metatraffic_unicast_locator_list().clone(),
            discovered_participant.metatraffic_multicast_locator_list().clone(),
        discovered_participant.expects_inline_qos(),
    true );
            self.sedp_builtin_subscriptions_writer.matched_reader_add(proxy);
        }
        
        if discovered_participant.available_built_in_endpoints().has(BuiltInEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER) {
            let guid = GUID::new(discovered_participant.guid_prefix(), ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
            let proxy = WriterProxy::new(
                guid,
                discovered_participant.metatraffic_unicast_locator_list().clone(), 
                discovered_participant.metatraffic_multicast_locator_list().clone());
            self.sedp_builtin_subscriptions_reader.matched_writer_add(proxy);
        }

        if discovered_participant.available_built_in_endpoints().has(BuiltInEndpointSet::BUILTIN_ENDPOINT_TOPICS_DETECTOR) {
            let guid = GUID::new(discovered_participant.guid_prefix(), ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR);
            let proxy = ReaderProxy::new(
                guid,
                discovered_participant.metatraffic_unicast_locator_list().clone(),
            discovered_participant.metatraffic_multicast_locator_list().clone(),
        discovered_participant.expects_inline_qos(),
    true );
            self.sedp_builtin_topics_writer.matched_reader_add(proxy);
        }

        if discovered_participant.available_built_in_endpoints().has(BuiltInEndpointSet::BUILTIN_ENDPOINT_TOPICS_ANNOUNCER) {
            let guid = GUID::new(discovered_participant.guid_prefix(), ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER);
            let proxy = WriterProxy::new(
                guid,
                discovered_participant.metatraffic_unicast_locator_list().clone(), 
                discovered_participant.metatraffic_multicast_locator_list().clone());
            self.sedp_builtin_topics_reader.matched_writer_add(proxy);
        }           
    }

    fn remove_discovered_participant(&self, remote_participant_guid_prefix: GuidPrefix) {
        // Implements the process described in
        // 8.5.5.2 Removal of a previously discovered Participant
        let guid = GUID::new(remote_participant_guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR);
        self.sedp_builtin_publications_writer.matched_reader_remove(&guid);

        let guid = GUID::new(remote_participant_guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER);
        self.sedp_builtin_publications_reader.matched_writer_remove(&guid);

        let guid = GUID::new(remote_participant_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
        self.sedp_builtin_subscriptions_writer.matched_reader_remove(&guid);

        let guid = GUID::new(remote_participant_guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        self.sedp_builtin_subscriptions_reader.matched_writer_remove(&guid);

        let guid = GUID::new(remote_participant_guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR);
        self.sedp_builtin_topics_writer.matched_reader_remove(&guid);

        let guid = GUID::new(remote_participant_guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER);
        self.sedp_builtin_topics_reader.matched_writer_remove(&guid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::constants::{PROTOCOL_VERSION_2_4};
    use crate::transport_stub::StubTransport;

    #[test]
    fn participant() {
        let participant = Participant::<StubTransport>::new(
            vec![],
            vec![],
            PROTOCOL_VERSION_2_4,
            [99,99]);

        participant.run();

        println!("Message: {:?}",participant.metatraffic_transport.pop_write().unwrap());

        participant.run();
    }
    // #[test]
    // fn test_participant() {
    //     let addr = [127, 0, 0, 1];
    //     let multicast_group = [239, 255, 0, 1];
    //     let port = 7400;
    //     let sender = std::net::UdpSocket::bind(SocketAddr::from((addr, 0))).unwrap();

    //     let vendor_id = [99, 99];
    //     let protocol_version = ProtocolVersion { major: 2, minor: 4 };
    //     let mut participant = Participant::new(vec![], vec![], protocol_version, vendor_id);

    //     let data = [
    //         0x52, 0x54, 0x50, 0x53, //000 protocol: ProtocolId_t => 'R', 'T', 'P', 'S',
    //         0x02, 0x01, 0x01, 0x02, //004 version: ProtocolVersion_t => 2.1 | vendorId: VendorId_t => 1,2
    //         0x7f, 0x20, 0xf7, 0xd7, //008 guidPrefix: GuidPrefix_t => 127, 32, 247, 215
    //         0x00, 0x00, 0x01, 0xbb, //012 guidPrefix: GuidPrefix_t => 0, 0, 1, 187
    //         0x00, 0x00, 0x00, 0x01, //016 guidPrefix: GuidPrefix_t => 0, 0, 0, 1
    //         0x09, 0x01, 0x08, 0x00, //020 submessageId: SubmessageKind => INFO_TS | flags: SubmessageFlag[8] => Endianess=little | submessageLength: ushort => 8
    //         0x9e, 0x81, 0xbc, 0x5d, //024  [InfoTimestamp Submessage]
    //         0x97, 0xde, 0x48, 0x26, //028  [InfoTimestamp Submessage]
    //         0x15, 0x07, 0x1c, 0x01, //032 submessageId: SubmessageKind => DATA | flags: SubmessageFlag[8] => N=0|K=0|D=1|Q=1|E=1 Endianess=little && InlineQosFlag && serializedPayload contains data | submessageLength (octetsToNextHeader): ushort => 284
    //         0x00, 0x00, 0x10, 0x00, //036  [Data Submessage] Flags: extraFlags | octetsToInlineQos: ushort => 16
    //         0x00, 0x00, 0x00, 0x00, //040  [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
    //         0x00, 0x01, 0x00, 0xc2, //044  [Data Submessage] EntityId writerId => ENTITYID_SPDP_BUILTIN_PARTICIPANT_DETECTOR ([0, 0x01, 0x00], ENTITY_KIND_BUILT_IN_READER_WITH_KEY)
    //         0x00, 0x00, 0x00, 0x00, //048  [Data Submessage] SequenceNumber writerSN 
    //         0x01, 0x00, 0x00, 0x00, //052  [Data Submessage] SequenceNumber writerSN => 1
    //         0x70, 0x00, 0x10, 0x00, //056  [Data Submessage: inLineQos] parameterId_1: short => PID_KEY_HASH | length: short => 16
    //         0x7f, 0x20, 0xf7, 0xd7, //060  [Data Submessage: inLineQos: KEY_HASH] 
    //         0x00, 0x00, 0x01, 0xbb, //064  [Data Submessage: inLineQos: KEY_HASH] 
    //         0x00, 0x00, 0x00, 0x01, //068  [Data Submessage: inLineQos: KEY_HASH]  
    //         0x00, 0x00, 0x01, 0xc1, //072  [Data Submessage: inLineQos: KEY_HASH]  
    //         0x01, 0x00, 0x00, 0x00, //076  [Data Submessage]  parameterId_1: short => PID_SENTINEL | 0
    //         0x00, 0x03, 0x00, 0x00, //080  [Data Submessage: SerializedPayload]   representation_identifier: octet[2] => PL_CDR_LE | representation_options: octet[2] => none
    //         0x15, 0x00, 0x04, 0x00, //084  [Data Submessage: SerializedPayload]   parameterId_1: short => PID_PROTOCOL_VERSION | length: short => 4
    //         0x02, 0x01, 0x00, 0x00, //088  [Data Submessage: SerializedPayload: PID_PROTOCOL_VERSION]  major: octet => 2 | minor: octet =>1 | padding 
    //         0x16, 0x00, 0x04, 0x00, //092  [Data Submessage: SerializedPayload]  parameterId_1: short => PID_VENDORID  | length: short => 4
    //         0x01, 0x02, 0x00, 0x00, //096  [Data Submessage: SerializedPayload: PID_VENDORID] vendorId: octet[2] => 12
    //         0x31, 0x00, 0x18, 0x00, //100  [Data Submessage: SerializedPayload]  parameterId_1: short =>  PID_DEFAULT_UNICAST_LOCATOR | length: short => 24
    //         0x01, 0x00, 0x00, 0x00, //104  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0xf3, 0x1c, 0x00, 0x00, //108  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0x00, 0x00, 0x00, 0x00, //112  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0x00, 0x00, 0x00, 0x00, //116  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]  
    //         0x00, 0x00, 0x00, 0x00, //120  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0xc0, 0xa8, 0x02, 0x04, //124  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]
    //         0x32, 0x00, 0x18, 0x00, //128  [Data Submessage: SerializedPayload] parameterId_1: short => PID_METATRAFFIC_UNICAST_LOCATOR | length: short => 24
    //         0x01, 0x00, 0x00, 0x00, //132  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0xf2, 0x1c, 0x00, 0x00, //136  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0x00, 0x00, 0x00, 0x00, //140  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0x00, 0x00, 0x00, 0x00, //144  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]  
    //         0x00, 0x00, 0x00, 0x00, //148  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0xc0, 0xa8, 0x02, 0x04, //152  [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0x02, 0x00, 0x08, 0x00, //156  [Data Submessage: SerializedPayload] parameterId_1: short => PID_PARTICIPANT_LEASE_DURATION | length: short => 8
    //         0x0b, 0x00, 0x00, 0x00, //160  [Data Submessage: SerializedPayload: PID_PARTICIPANT_LEASE_DURATION] seconds: long => 11 
    //         0x00, 0x00, 0x00, 0x00, //164  [Data Submessage: SerializedPayload: PID_PARTICIPANT_LEASE_DURATION] fraction: ulong => 0    
    //         0x50, 0x00, 0x10, 0x00, //168  [Data Submessage: SerializedPayload] parameterId_1: short => PID_PARTICIPANT_GUID | length: short => 16
    //         0x7f, 0x20, 0xf7, 0xd7, //172  [Data Submessage: SerializedPayload: PID_PARTICIPANT_GUID] 
    //         0x00, 0x00, 0x01, 0xbb, //176  [Data Submessage: SerializedPayload: PID_PARTICIPANT_GUID]   
    //         0x00, 0x00, 0x00, 0x01, //180  [Data Submessage: SerializedPayload: PID_PARTICIPANT_GUID]   
    //         0x00, 0x00, 0x01, 0xc1, //184  [Data Submessage: SerializedPayload: PID_PARTICIPANT_GUID]   
    //         0x58, 0x00, 0x04, 0x00, //188  [Data Submessage: SerializedPayload] parameterId_1: short => PID_BUILTIN_ENDPOINT_SET | length: short => 4
    //         0x15, 0x04, 0x00, 0x00, //192  [Data Submessage: SerializedPayload: PID_BUILTIN_ENDPOINT_SET] BuiltinEndpointSet: bitmask => (0100 0001 0101‬) PARTICIPANT_ANNOUNCER && PUBLICATIONS_ANNOUNCER && SUBSCRIPTIONS_ANNOUNCER && PARTICIPANT_MESSAGE_DATA_WRITER
    //         0x00, 0x80, 0x04, 0x00, //196  [Data Submessage: SerializedPayload] parameterId_1: short => Vendor-specific ParameterId (0x8000) | length: short => 4   
    //         0x15, 0x00, 0x00, 0x00, //200  [Data Submessage: SerializedPayload: Vendor-specific 0x0]  
    //         0x07, 0x80, 0x5c, 0x00, //204  [Data Submessage: SerializedPayload] parameterId_1: short => Vendor-specific ParameterId (0x8007) | length: short => 92     
    //         0x00, 0x00, 0x00, 0x00, //208  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x2f, 0x00, 0x00, 0x00, //212  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x05, 0x00, 0x00, 0x00, //216  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x00, 0x00, 0x00, 0x00, //220  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x50, 0x00, 0x00, 0x00, //224  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x42, 0x00, 0x00, 0x00, //228  [Data Submessage: SerializedPayload: Vendor-specific 0x7]  
    //         0x44, 0x45, 0x53, 0x4b, //232  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x54, 0x4f, 0x50, 0x2d, //236  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x4f, 0x52, 0x46, 0x44, //240  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x4f, 0x53, 0x35, 0x2f, //244  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x36, 0x2e, 0x31, 0x30, //248  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x2e, 0x32, 0x2f, 0x63, //252  [Data Submessage: SerializedPayload: Vendor-specific 0x7]  
    //         0x63, 0x36, 0x66, 0x62, //256  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x39, 0x61, 0x62, 0x33, //260  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x36, 0x2f, 0x39, 0x30, //264  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x37, 0x65, 0x66, 0x66, //268  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x30, 0x32, 0x65, 0x33, //272  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x2f, 0x22, 0x78, 0x38, //276  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x36, 0x5f, 0x36, 0x34, //280  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x2e, 0x77, 0x69, 0x6e, //284  [Data Submessage: SerializedPayload: Vendor-specific 0x7]  
    //         0x2d, 0x76, 0x73, 0x32, //288  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x30, 0x31, 0x35, 0x22, //292  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x2f, 0x00, 0x00, 0x00, //296  [Data Submessage: SerializedPayload: Vendor-specific 0x7]   
    //         0x25, 0x80, 0x0c, 0x00, //300  [Data Submessage: SerializedPayload] parameterId_1: short => Vendor-specific ParameterId (0x8025) | length: short => 12       
    //         0xd7, 0xf7, 0x20, 0x7f, //304  [Data Submessage: SerializedPayload: Vendor-specific ParameterId 0x25]   
    //         0xbb, 0x01, 0x00, 0x00, //308  [Data Submessage: SerializedPayload: Vendor-specific ParameterId 0x25]   
    //         0x01, 0x00, 0x00, 0x00, //312  [Data Submessage: SerializedPayload: Vendor-specific ParameterId 0x25]  
    //         0x01, 0x00, 0x00, 0x00, //316  [Data Submessage: SerializedPayload] parameterId_1: short => PID_SENTINEL |  length: short => 0
    //     ];
    //     sender
    //         .send_to(&data, SocketAddr::from((multicast_group, port)))
    //         .unwrap();

    //     assert_eq!(
    //         participant
    //             .spdp_builtin_participant_reader
    //             .reader_cache
    //             .get_changes()
    //             .len(),
    //         0
    //     );

    //     assert_eq!(participant.participant_proxy_list.len(), 0);

    //     participant.receive_data();

    //     assert_eq!(
    //         participant
    //             .spdp_builtin_participant_reader
    //             .reader_cache
    //             .get_changes()
    //             .len(),
    //         1
    //     );

    //     assert_eq!(participant.participant_proxy_list.len(), 1);
    // }

    // #[test]
    // fn create_participant_proxy_data() {
    //     let vendor_id = [0x01, 0x42];
    //     let protocol_version = ProtocolVersion { major: 2, minor: 1 };
    //     let default_unicast_address = [0,0,0,0,0,0,0,0,0,0,0,0,192,168,2,4];
    //     let metatraffic_multicast_address = [0,0,0,0,0,0,0,0,0,0,0,0,192,168,2,4];
    //     let participant = Participant::new(vec![Locator::new(1,7411,default_unicast_address)], vec![Locator::new(1,7410,metatraffic_multicast_address)], protocol_version, vendor_id);
    //     let data = vec![
    //         0x00, 0x03, 0x00, 0x00, // [Data Submessage: SerializedPayload]   representation_identifier: octet[2] => PL_CDR_LE | representation_options: octet[2] => none
    //         0x15, 0x00, 0x04, 0x00, // [Data Submessage: SerializedPayload]   parameterId_1: short => PID_PROTOCOL_VERSION | length: short => 4
    //         0x02, 0x01, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_PROTOCOL_VERSION]  major: octet => 2 | minor: octet =>1 | padding 
    //         0x16, 0x00, 0x04, 0x00, // [Data Submessage: SerializedPayload]  parameterId_1: short => PID_VENDORID  | length: short => 4
    //         0x01, 0x42, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_VENDORID] vendorId: octet[2] => 12
    //         0x31, 0x00, 0x18, 0x00, // [Data Submessage: SerializedPayload]  parameterId_1: short =>  PID_DEFAULT_UNICAST_LOCATOR | length: short => 24
    //         0x01, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0xf3, 0x1c, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0x00, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0x00, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]  
    //         0x00, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0xc0, 0xa8, 0x02, 0x04, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]
    //         0x32, 0x00, 0x18, 0x00, // [Data Submessage: SerializedPayload] parameterId_1: short => PID_METATRAFFIC_UNICAST_LOCATOR | length: short => 24
    //         0x01, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0xf2, 0x1c, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0x00, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0x00, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]  
    //         0x00, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0xc0, 0xa8, 0x02, 0x04, // [Data Submessage: SerializedPayload: PID_DEFAULT_UNICAST_LOCATOR]   
    //         0x02, 0x00, 0x08, 0x00, // [Data Submessage: SerializedPayload] parameterId_1: short => PID_PARTICIPANT_LEASE_DURATION | length: short => 8
    //         0x0b, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_PARTICIPANT_LEASE_DURATION] seconds: long => 11 
    //         0x00, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_PARTICIPANT_LEASE_DURATION] fraction: ulong => 0    
    //         0x50, 0x00, 0x10, 0x00, // [Data Submessage: SerializedPayload] parameterId_1: short => PID_PARTICIPANT_GUID | length: short => 16
    //         0x05, 0x06, 0x07, 0x08, // [Data Submessage: SerializedPayload: PID_PARTICIPANT_GUID] 
    //         0x09, 0x05, 0x01, 0x02, // [Data Submessage: SerializedPayload: PID_PARTICIPANT_GUID]   
    //         0x03, 0x04, 0x0a, 0x0b, // [Data Submessage: SerializedPayload: PID_PARTICIPANT_GUID]   
    //         0x00, 0x00, 0x01, 0xc1, // [Data Submessage: SerializedPayload: PID_PARTICIPANT_GUID]   
    //         0x58, 0x00, 0x04, 0x00, // [Data Submessage: SerializedPayload] parameterId_1: short => PID_BUILTIN_ENDPOINT_SET | length: short => 4
    //         0x15, 0x04, 0x00, 0x00, // [Data Submessage: SerializedPayload: PID_BUILTIN_ENDPOINT_SET] BuiltinEndpointSet: bitmask => (0100 0001 0101‬) PARTICIPANT_ANNOUNCER && PUBLICATIONS_ANNOUNCER && SUBSCRIPTIONS_ANNOUNCER && PARTICIPANT_MESSAGE_DATA_WRITER
    //         0x01, 0x00, 0x00, 0x00, // [Data Submessage: SerializedPayload] parameterId_1: short => PID_SENTINEL |  length: short => 0
    //     ];
    //     assert_eq!(
    //         cdr::serialize::<_,_,PlCdrLe>(&participant, Infinite).unwrap(),
    //         data);
    // }
}
