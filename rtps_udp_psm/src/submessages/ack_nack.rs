use rust_rtps_pim::messages::Submessage;

use crate::RtpsUdpPsm;

pub struct AckNack {
    endianness_flag: <<Self as Submessage>::PSM as rust_rtps_pim::messages::Types>::SubmessageFlag,
    final_flag: <<Self as Submessage>::PSM as rust_rtps_pim::messages::Types>::SubmessageFlag,
    reader_id: rust_rtps_pim::messages::submessage_elements::EntityId<<Self as Submessage>::PSM>,
    writer_id: rust_rtps_pim::messages::submessage_elements::EntityId<<Self as Submessage>::PSM>,
    reader_sn_state:
        rust_rtps_pim::messages::submessage_elements::SequenceNumberSet<<Self as Submessage>::PSM>,
    count: rust_rtps_pim::messages::submessage_elements::Count<<Self as Submessage>::PSM>,
}

impl Submessage for AckNack {
    type PSM = RtpsUdpPsm;

    fn submessage_header(&self) -> rust_rtps_pim::messages::SubmessageHeader<Self::PSM> {
        todo!()
    }
}

impl rust_rtps_pim::messages::submessages::AckNack for AckNack {
    fn new(
        endianness_flag: <Self::PSM as rust_rtps_pim::messages::Types>::SubmessageFlag,
        final_flag: <Self::PSM as rust_rtps_pim::messages::Types>::SubmessageFlag,
        reader_id: rust_rtps_pim::messages::submessage_elements::EntityId<Self::PSM>,
        writer_id: rust_rtps_pim::messages::submessage_elements::EntityId<Self::PSM>,
        reader_sn_state: rust_rtps_pim::messages::submessage_elements::SequenceNumberSet<Self::PSM>,
        count: rust_rtps_pim::messages::submessage_elements::Count<Self::PSM>,
    ) -> Self {
        Self {
            endianness_flag,
            final_flag,
            reader_id,
            writer_id,
            reader_sn_state,
            count,
        }
    }

    fn endianness_flag(&self) -> <Self::PSM as rust_rtps_pim::messages::Types>::SubmessageFlag {
        self.endianness_flag
    }

    fn final_flag(&self) -> <Self::PSM as rust_rtps_pim::messages::Types>::SubmessageFlag {
        self.final_flag
    }

    fn reader_id(&self) -> &rust_rtps_pim::messages::submessage_elements::EntityId<Self::PSM> {
        &self.reader_id
    }

    fn writer_id(&self) -> &rust_rtps_pim::messages::submessage_elements::EntityId<Self::PSM> {
        &self.writer_id
    }

    fn reader_sn_state(
        &self,
    ) -> &rust_rtps_pim::messages::submessage_elements::SequenceNumberSet<Self::PSM> {
        &self.reader_sn_state
    }

    fn count(&self) -> &rust_rtps_pim::messages::submessage_elements::Count<Self::PSM> {
        &self.count
    }
}
