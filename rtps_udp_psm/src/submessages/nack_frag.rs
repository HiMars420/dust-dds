use crate::{Count, EntityId, RtpsUdpPsm, SequenceNumber, SubmessageFlag, FragmentNumberSet};

pub struct NackFrag;

impl rust_rtps_pim::messages::submessages::NackFrag<RtpsUdpPsm> for NackFrag {
    type EntityId = EntityId;
    type SequenceNumber = SequenceNumber;
    type FragmentNumberSet = FragmentNumberSet;
    type Count = Count;

    fn endianness_flag(&self) -> SubmessageFlag {
        todo!()
    }

    fn reader_id(&self) -> &Self::EntityId {
        todo!()
    }

    fn writer_id(&self) -> &Self::EntityId {
        todo!()
    }

    fn writer_sn(&self) -> &Self::SequenceNumber {
        todo!()
    }

    fn fragment_number_state(&self) -> &Self::FragmentNumberSet {
        todo!()
    }

    fn count(&self) -> &Self::Count {
        todo!()
    }
}

impl rust_rtps_pim::messages::Submessage<RtpsUdpPsm> for NackFrag {
    fn submessage_header(&self) -> rust_rtps_pim::messages::SubmessageHeader<RtpsUdpPsm> {
        todo!()
    }
}
