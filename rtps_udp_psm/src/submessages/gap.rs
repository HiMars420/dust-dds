use rust_rtps_pim::messages::Submessage;

use crate::RtpsUdpPsm;

pub struct Gap {
    endianness_flag: <<Self as Submessage>::PSM as rust_rtps_pim::messages::Types>::SubmessageFlag,
    reader_id: rust_rtps_pim::messages::submessage_elements::EntityId<<Self as Submessage>::PSM>,
    writer_id: rust_rtps_pim::messages::submessage_elements::EntityId<<Self as Submessage>::PSM>,
    gap_start:
        rust_rtps_pim::messages::submessage_elements::SequenceNumber<<Self as Submessage>::PSM>,
    gap_list:
        rust_rtps_pim::messages::submessage_elements::SequenceNumberSet<<Self as Submessage>::PSM>,
}

impl Submessage for Gap {
    type PSM = RtpsUdpPsm;

    fn submessage_header(&self) -> rust_rtps_pim::messages::SubmessageHeader<Self::PSM> {
        todo!()
    }
}

impl rust_rtps_pim::messages::submessages::Gap for Gap {
    fn new(
        endianness_flag: <Self::PSM as rust_rtps_pim::messages::Types>::SubmessageFlag,
        reader_id: <Self::PSM as rust_rtps_pim::structure::Types>::EntityId,
        writer_id: <Self::PSM as rust_rtps_pim::structure::Types>::EntityId,
        gap_start: <Self::PSM as rust_rtps_pim::structure::Types>::SequenceNumber,
        gap_list: <Self::PSM as rust_rtps_pim::structure::Types>::SequenceNumberVector,
    ) -> Self {
        Self {
            endianness_flag,
            reader_id: rust_rtps_pim::messages::submessage_elements::EntityId { value: reader_id },
            writer_id: rust_rtps_pim::messages::submessage_elements::EntityId { value: writer_id },
            gap_start: rust_rtps_pim::messages::submessage_elements::SequenceNumber {
                value: gap_start,
            },
            gap_list: rust_rtps_pim::messages::submessage_elements::SequenceNumberSet {
                base: gap_start,
                set: gap_list,
            },
        }
    }

    fn endianness_flag(&self) -> <Self::PSM as rust_rtps_pim::messages::Types>::SubmessageFlag {
        self.endianness_flag
    }

    fn reader_id(&self) -> &rust_rtps_pim::messages::submessage_elements::EntityId<Self::PSM> {
        &self.reader_id
    }

    fn writer_id(&self) -> &rust_rtps_pim::messages::submessage_elements::EntityId<Self::PSM> {
        &self.writer_id
    }

    fn gap_start(
        &self,
    ) -> &rust_rtps_pim::messages::submessage_elements::SequenceNumber<Self::PSM> {
        &self.gap_start
    }

    fn gap_list(
        &self,
    ) -> &rust_rtps_pim::messages::submessage_elements::SequenceNumberSet<Self::PSM> {
        &self.gap_list
    }
}
