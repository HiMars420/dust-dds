use std::sync::{Arc, Mutex, };

use crate::types::{GUID, ProtocolVersion, VendorId, EntityId, EntityKind, ChangeKind};
use crate::types::constants::{
    ENTITYID_PARTICIPANT,
    PROTOCOL_VERSION_2_4,};
use crate::transport::Transport;
use crate::messages::message_receiver::RtpsMessageReceiver;
use crate::messages::message_sender::RtpsMessageSender;

use crate::behavior::types::Duration;
use crate::behavior::StatelessWriter;
use crate::types::GuidPrefix;
use crate::types::constants::ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER;
use rust_dds_interface::qos::DataWriterQos;
use crate::discovery::spdp::SPDPdiscoveredParticipantData;
use crate::endpoint_types::BuiltInEndpointSet;
use crate::serialized_payload::CdrEndianness;

use super::{RtpsGroup, RtpsEntity, RtpsRun};

use rust_dds_interface::types::{DomainId, InstanceHandle, ReturnCode, TopicKind};
use rust_dds_interface::protocol::{ProtocolEntity, ProtocolParticipant, ProtocolPublisher, ProtocolSubscriber};


pub struct RtpsParticipant {
    guid: GUID,
    domain_id: DomainId,
    protocol_version: ProtocolVersion,
    vendor_id: VendorId,
    userdata_transport: Arc<dyn Transport>,
    metatraffic_transport: Arc<dyn Transport>,
    builtin_publisher: Arc<Mutex<RtpsGroup>>,
    builtin_subscriber: Arc<Mutex<RtpsGroup>>, 
    publisher_list: Vec<Arc<Mutex<RtpsGroup>>>,
    subscriber_list: Vec<Arc<Mutex<RtpsGroup>>>,
}

impl RtpsParticipant {
    pub fn new(
        domain_id: DomainId,
        userdata_transport: impl Transport + 'static,
        metatraffic_transport: impl Transport + 'static,
    ) -> Self {
        let userdata_transport = Arc::new(userdata_transport);
        let metatraffic_transport = Arc::new(metatraffic_transport);
        let protocol_version = PROTOCOL_VERSION_2_4;
        let vendor_id = [99,99];
        let guid_prefix = [5, 6, 7, 8, 9, 5, 1, 2, 3, 4, 10, 11];   // TODO: Should be uniquely generated

        let builtin_publisher_guid = GUID::new(guid_prefix, EntityId::new([3,3,3], EntityKind::BuiltInWriterGroup));
        let builtin_subscriber_guid = GUID::new(guid_prefix, EntityId::new([3,3,3], EntityKind::BuiltInReaderGroup));

        let builtin_publisher = Arc::new(Mutex::new(RtpsGroup::new(builtin_publisher_guid, RtpsMessageSender::new(metatraffic_transport.clone()))));
        let builtin_subscriber = Arc::new(Mutex::new(RtpsGroup::new(builtin_subscriber_guid, RtpsMessageSender::new(metatraffic_transport.clone()))));

        Self {
            guid: GUID::new(guid_prefix,ENTITYID_PARTICIPANT ),
            domain_id,
            protocol_version,
            vendor_id,
            userdata_transport,
            metatraffic_transport,
            builtin_subscriber,
            builtin_publisher,
            publisher_list: Vec::new(),
            subscriber_list: Vec::new(),
        }
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

    pub fn userdata_transport(&self) -> &Arc<dyn Transport> {
        &self.userdata_transport
    }

    pub fn metatraffic_transport(&self) -> &Arc<dyn Transport> {
        &self.metatraffic_transport
    }

    pub fn initialize_spdp(&self, domain_tag: String, lease_duration: Duration) {
        let spdp_data = SPDPdiscoveredParticipantData::new(
            self.domain_id,
            domain_tag,
            self.protocol_version,
            self.guid.prefix(),
            self.vendor_id,
            self.metatraffic_transport.unicast_locator_list().clone(),
            self.metatraffic_transport.multicast_locator_list().clone(),
            self.userdata_transport.unicast_locator_list().clone(),
            self.userdata_transport.multicast_locator_list().clone(),
            BuiltInEndpointSet::new(0),
            lease_duration,
        );

        let writer_guid = GUID::new(self.guid.prefix(), ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER);
        let writer_qos = DataWriterQos::default(); // TODO: Should be adjusted according to the SPDP writer
        let mut spdp_builtin_participant_writer = StatelessWriter::new(writer_guid, TopicKind::WithKey, &writer_qos);

        let change = spdp_builtin_participant_writer.new_change(ChangeKind::Alive, Some(spdp_data.data(CdrEndianness::LittleEndian)), None, spdp_data.key());
        spdp_builtin_participant_writer.writer_cache().add_change(change).unwrap();

        self.builtin_publisher.lock().unwrap().mut_endpoints().push(Arc::new(Mutex::new(spdp_builtin_participant_writer)));
    }
}

impl RtpsEntity for RtpsParticipant {
    fn guid(&self) -> GUID {
        self.guid
    }
}

impl RtpsRun for RtpsParticipant {
    fn run(&mut self) {
        RtpsMessageReceiver::receive(
            self.guid.prefix(),
            self.metatraffic_transport.as_ref(),
            &[&self.builtin_subscriber, &self.builtin_publisher]
        );

        self.builtin_publisher.lock().unwrap().run();
    }
}

impl ProtocolEntity for RtpsParticipant {
    fn get_instance_handle(&self) -> InstanceHandle {
        self.guid.into()
    }

    fn enable(&self) -> ReturnCode<()> {
        Ok(())
    }
}

impl ProtocolParticipant for RtpsParticipant {
    fn create_publisher(&mut self) -> Arc<Mutex<dyn ProtocolPublisher>> {
        let index = match self.publisher_list.iter()
            .max_by(|&x, &y| 
            x.lock().unwrap().guid().entity_id().entity_key()[0].cmp(&y.lock().unwrap().guid().entity_id().entity_key()[0])) {
                Some(group) => group.lock().unwrap().guid().entity_id().entity_key()[0] + 1,
                None => 0,
        };

        let guid_prefix = self.guid.prefix();
        let entity_id = EntityId::new([index as u8,0,0], EntityKind::UserDefinedWriterGroup);
        let publisher_guid = GUID::new(guid_prefix, entity_id);
        // let publisher_sender = RtpsMessageSender::new(self.userdata_transport.clone());
        let new_publisher = Arc::new(Mutex::new(RtpsGroup::new(publisher_guid, RtpsMessageSender::new(self.userdata_transport.clone()))));
        self.publisher_list.push(new_publisher.clone());

        new_publisher
    }

    fn create_subscriber(&mut self) -> Arc<Mutex<dyn ProtocolSubscriber>> {
        let index = match self.subscriber_list.iter()
            .max_by(|&x, &y| 
            x.lock().unwrap().guid().entity_id().entity_key()[0].cmp(&y.lock().unwrap().guid().entity_id().entity_key()[0])) {
                Some(group) => group.lock().unwrap().guid().entity_id().entity_key()[0] + 1,
                None => 0,
        };

        let guid_prefix = self.guid.prefix();
        let entity_id = EntityId::new([index as u8,0,0], EntityKind::UserDefinedReaderGroup);
        let subscriber_guid = GUID::new(guid_prefix, entity_id);
        // let subscriber_sender = RtpsMessageSender::new(self.userdata_transport.clone());
        let new_subscriber = Arc::new(Mutex::new(RtpsGroup::new(subscriber_guid, RtpsMessageSender::new(self.userdata_transport.clone()))));
        self.subscriber_list.push(new_subscriber.clone());

        new_subscriber
    }

    fn get_builtin_subscriber(&self) -> Arc<Mutex<dyn ProtocolSubscriber>> {
        self.builtin_subscriber.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Locator;

    struct MockTransport{
        multicast_locator_list: Vec<Locator>,
    }

    impl MockTransport{
        fn new() -> Self {
            Self {
                multicast_locator_list: vec![Locator::new_udpv4(7400, [235,0,0,1])],
            }
        }
    }

    impl Transport for MockTransport {
        fn write(&self, _message: crate::RtpsMessage, _destination_locator: &Locator) {
            todo!()
        }

        fn read(&self) -> crate::transport::TransportResult<Option<(crate::RtpsMessage, Locator)>> {
            todo!()
        }

        fn unicast_locator_list(&self) -> &Vec<Locator> {
            todo!()
        }

        fn multicast_locator_list(&self) -> &Vec<Locator> {
            &self.multicast_locator_list
        }

        fn as_any(&self) -> &dyn std::any::Any {
            todo!()
        }
    }

    #[test]
    fn create_publisher() {
        let mut participant = RtpsParticipant::new(0, MockTransport::new(), MockTransport::new());
        let participant_guid_prefix = &participant.get_instance_handle()[0..12];

        let publisher1 = participant.create_publisher();
        let publisher1 = publisher1.lock().unwrap();
        let publisher1_entityid = [0,0,0,8];
        assert_eq!(&publisher1.get_instance_handle()[0..12], participant_guid_prefix); 
        assert_eq!(publisher1.get_instance_handle()[12..16], publisher1_entityid);

        let publisher2 = participant.create_publisher();
        let publisher2 = publisher2.lock().unwrap();
        let publisher2_entityid = [1,0,0,8];
        assert_eq!(&publisher2.get_instance_handle()[0..12], participant_guid_prefix); 
        assert_eq!(publisher2.get_instance_handle()[12..16], publisher2_entityid);

        std::mem::drop(publisher1);

        let publisher3 = participant.create_publisher();
        let publisher3 = publisher3.lock().unwrap();
        let publisher3_entityid = [0,0,0,8];
        assert_eq!(&publisher3.get_instance_handle()[0..12], participant_guid_prefix); 
        assert_eq!(publisher3.get_instance_handle()[12..16], publisher3_entityid);
    }

    #[test]
    fn create_subscriber() {
        let mut participant = RtpsParticipant::new(0, MockTransport::new(), MockTransport::new());
        let participant_guid_prefix = &participant.get_instance_handle()[0..12];

        let subscriber1 = participant.create_subscriber();
        let subscriber1 = subscriber1.lock().unwrap();
        let subscriber1_entityid = [0,0,0,9];
        assert_eq!(&subscriber1.get_instance_handle()[0..12], participant_guid_prefix); 
        assert_eq!(subscriber1.get_instance_handle()[12..16], subscriber1_entityid);

        let subscriber2 = participant.create_subscriber();
        let subscriber2 = subscriber2.lock().unwrap();
        let subscriber2_entityid = [1,0,0,9];
        assert_eq!(&subscriber2.get_instance_handle()[0..12], participant_guid_prefix); 
        assert_eq!(subscriber2.get_instance_handle()[12..16], subscriber2_entityid);

        std::mem::drop(subscriber1);

        let subscriber3 = participant.create_subscriber();
        let subscriber3 = subscriber3.lock().unwrap();
        let subscriber3_entityid = [0,0,0,9];
        assert_eq!(&subscriber3.get_instance_handle()[0..12], participant_guid_prefix); 
        assert_eq!(subscriber3.get_instance_handle()[12..16], subscriber3_entityid);
    }
}

