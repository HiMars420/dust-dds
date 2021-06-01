use crate::{EntityId, RtpsUdpPsm, SequenceNumber, SequenceNumberSet, SubmessageFlag};

use super::SubmessageHeader;

pub struct Gap;

impl rust_rtps_pim::messages::submessages::GapSubmessage<RtpsUdpPsm> for Gap {
    type EntityId = EntityId;
    type SequenceNumber = SequenceNumber;
    type SequenceNumberSet = SequenceNumberSet;

    fn endianness_flag(&self) -> SubmessageFlag {
        todo!()
    }

    fn reader_id(&self) -> &Self::EntityId {
        todo!()
    }

    fn writer_id(&self) -> &Self::EntityId {
        todo!()
    }

    fn gap_start(&self) -> &Self::SequenceNumber {
        todo!()
    }

    fn gap_list(&self) -> &Self::SequenceNumberSet {
        todo!()
    }
}

impl rust_rtps_pim::messages::Submessage<RtpsUdpPsm> for Gap {
    type SubmessageHeader = SubmessageHeader;

    fn submessage_header(&self) -> Self::SubmessageHeader {
        todo!()
    }
}
