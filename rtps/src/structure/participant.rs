use std::sync::{Arc, Weak, Mutex};
use crate::types::{GUID, Locator, ProtocolVersion, VendorId, EntityId, EntityKind};
use crate::types::constants::{
    ENTITYID_PARTICIPANT,
    PROTOCOL_VERSION_2_4,};
use crate::transport::Transport;

use super::publisher::RtpsPublisher;
use rust_dds_interface::types::DomainId;
use rust_dds_interface::protocol::{ProtocolEntity, ProtocolParticipant, ProtocolPublisher, ProtocolSubscriber};



pub struct RtpsParticipant {
    guid: GUID,
    domain_id: DomainId,
    protocol_version: ProtocolVersion,
    vendor_id: VendorId,
    userdata_transport: Box<dyn Transport>,
    metatraffic_transport: Box<dyn Transport>,
    publisher_list: Mutex<[Weak<RtpsPublisher>;32]>,
    subscriber_list: Mutex<[Weak<RtpsPublisher>;32]>,
}

impl RtpsParticipant {
    pub fn new(
        domain_id: DomainId,
        userdata_transport: impl Transport + 'static,
        metatraffic_transport: impl Transport + 'static,
    ) -> Self {
        let protocol_version = PROTOCOL_VERSION_2_4;
        let vendor_id = [99,99];
        let guid_prefix = [5, 6, 7, 8, 9, 5, 1, 2, 3, 4, 10, 11];   // TODO: Should be uniquely generated


        Self {
            guid: GUID::new(guid_prefix,ENTITYID_PARTICIPANT ),
            domain_id,
            protocol_version,
            vendor_id,
            userdata_transport: Box::new(userdata_transport),
            metatraffic_transport: Box::new(metatraffic_transport),
            publisher_list: Mutex::new(Default::default()),
            subscriber_list: Mutex::new(Default::default()),
        }
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
        self.userdata_transport.unicast_locator_list()
    }

    pub fn default_multicast_locator_list(&self) -> &Vec<Locator> {
        self.userdata_transport.multicast_locator_list()
    }

    pub fn metatraffic_unicast_locator_list(&self) -> &Vec<Locator> {
        self.metatraffic_transport.unicast_locator_list()
    }

    pub fn metatraffic_multicast_locator_list(&self) -> &Vec<Locator> {
        self.metatraffic_transport.multicast_locator_list()
    }
}

impl ProtocolEntity for RtpsParticipant {
    fn get_instance_handle(&self) -> rust_dds_interface::types::InstanceHandle {
        todo!()
    }

    fn enable(&self) -> rust_dds_interface::types::ReturnCode<()> {
        todo!()
    }
}

impl ProtocolParticipant for RtpsParticipant {
    fn create_publisher(&self) -> Arc<dyn ProtocolPublisher> {
        let mut publisher_list = self.publisher_list.lock().unwrap();
        let index = publisher_list.iter().position(|x| x.strong_count() == 0).unwrap();

        let guid_prefix = self.guid.prefix();
        let entity_id = EntityId::new([index as u8,0,0], EntityKind::UserDefinedWriterGroup);
        let publisher_guid = GUID::new(guid_prefix, entity_id);
        let new_publisher = Arc::new(RtpsPublisher::new(publisher_guid));
        publisher_list[index] = Arc::downgrade(&new_publisher);

        new_publisher
    }

    fn create_subscriber(&self) -> Arc<dyn ProtocolSubscriber> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct MockTransport;

    impl Transport for MockTransport {
        fn write(&self, _message: crate::RtpsMessage, _destination_locator_list: &[Locator]) {
            todo!()
        }

        fn read(&self) -> crate::transport::TransportResult<Option<(crate::RtpsMessage, Locator)>> {
            todo!()
        }

        fn unicast_locator_list(&self) -> &Vec<Locator> {
            todo!()
        }

        fn multicast_locator_list(&self) -> &Vec<Locator> {
            todo!()
        }

        fn as_any(&self) -> &dyn std::any::Any {
            todo!()
        }
    }

    #[test]
    fn create_publisher() {
        let participant = RtpsParticipant::new(0, MockTransport, MockTransport);

        assert_eq!(participant.publisher_list.lock().unwrap()[0].strong_count(),0);
        assert_eq!(participant.publisher_list.lock().unwrap()[1].strong_count(),0);

        let publisher1 = participant.create_publisher();

        assert_eq!(participant.publisher_list.lock().unwrap()[0].strong_count(),1);
        assert_eq!(participant.publisher_list.lock().unwrap()[1].strong_count(),0);

        let _publisher2 = participant.create_publisher();

        assert_eq!(participant.publisher_list.lock().unwrap()[0].strong_count(),1);
        assert_eq!(participant.publisher_list.lock().unwrap()[1].strong_count(),1);

        std::mem::drop(publisher1);

        assert_eq!(participant.publisher_list.lock().unwrap()[0].strong_count(),0);
        assert_eq!(participant.publisher_list.lock().unwrap()[1].strong_count(),1);

        let _publisher3 = participant.create_publisher();

        assert_eq!(participant.publisher_list.lock().unwrap()[0].strong_count(),1);
        assert_eq!(participant.publisher_list.lock().unwrap()[1].strong_count(),1);
    }
}

