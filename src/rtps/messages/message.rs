use super::types::constants::PROTOCOL_RTPS;
use crate::rtps::types::{GuidPrefix, ProtocolVersion, VendorId};
use super::submessages::RtpsSubmessage;
use super::types::ProtocolId;

#[derive(PartialEq, Debug)]
pub struct Header {
    protocol: ProtocolId,
    version: ProtocolVersion,
    vendor_id: VendorId,
    guid_prefix: GuidPrefix,
}

impl Header {
    pub fn new(version: ProtocolVersion, vendor_id: VendorId, guid_prefix: GuidPrefix) -> Self {
        Self {
            protocol: PROTOCOL_RTPS,
            version: version,
            vendor_id: vendor_id,
            guid_prefix,
        }
    }

    pub fn protocol(&self) -> ProtocolId {
        self.protocol
    }

    pub fn version(&self) -> ProtocolVersion {
        self.version
    }

    pub fn vendor_id(&self) -> VendorId {
        self.vendor_id
    }

    pub fn guid_prefix(&self) -> GuidPrefix {
        self.guid_prefix
    }   
}

#[derive(PartialEq, Debug)]
pub struct RtpsMessage {
    header: Header,
    submessages: Vec<RtpsSubmessage>,
}

impl RtpsMessage {
    pub fn new(version: ProtocolVersion, vendor_id: VendorId, guid_prefix: GuidPrefix, submessages: Vec<RtpsSubmessage>) -> Self {
        if submessages.is_empty() {
            panic!("At least one submessage is required");
        };

        RtpsMessage {
            header: Header::new(version, vendor_id, guid_prefix),
            submessages,
        }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn submessages(&self) -> &Vec<RtpsSubmessage> {
        &self.submessages
    }

    pub fn take_submessages(self) -> Vec<RtpsSubmessage> {
        self.submessages
    }
}