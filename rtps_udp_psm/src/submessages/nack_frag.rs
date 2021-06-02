use crate::{Count, EntityId, FragmentNumberSet, RtpsUdpPsm, SequenceNumber, SubmessageFlag};

use super::SubmessageHeader;

pub struct NackFrag;

impl rust_rtps_pim::messages::submessages::NackFragSubmessage<RtpsUdpPsm> for NackFrag {
    type EntityId = EntityId;
    type SequenceNumber = SequenceNumber;
    type FragmentNumberSet = FragmentNumberSet;
    type Count = Count;

    fn new(
        _endianness_flag: SubmessageFlag,
        _reader_id: Self::EntityId,
        _writer_id: Self::EntityId,
        _writer_sn: Self::SequenceNumber,
        _fragment_number_state: Self::FragmentNumberSet,
        _count: Self::Count,
    ) -> Self {
        todo!()
    }

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
    type SubmessageHeader = SubmessageHeader;

    fn submessage_header(&self) -> Self::SubmessageHeader {
        todo!()
    }
}
