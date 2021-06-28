use crate::psm::RtpsUdpPsm;

use super::header::SubmessageHeader;

#[derive(Debug, PartialEq)]
pub struct Pad;

impl rust_rtps_pim::messages::submessages::PadSubmessage<RtpsUdpPsm> for Pad {}

impl rust_rtps_pim::messages::Submessage<RtpsUdpPsm> for Pad {
    fn submessage_header(&self) -> SubmessageHeader {
        todo!()
    }
}
