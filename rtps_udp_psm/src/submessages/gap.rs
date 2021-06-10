use rust_rtps_pim::messages::types::SubmessageKindPIM;

use crate::{EntityId, RtpsUdpPsm, SequenceNumber, SequenceNumberSet, SubmessageFlag};

use super::SubmessageHeader;

#[derive(serde::Serialize)]
pub struct GapSubmessage {
    header: SubmessageHeader,
    reader_id: EntityId,
    writer_id: EntityId,
    gap_start: SequenceNumber,
    gap_list: SequenceNumberSet,
}

impl rust_rtps_pim::messages::submessages::GapSubmessage<RtpsUdpPsm> for GapSubmessage {
    fn new(
        endianness_flag: SubmessageFlag,
        reader_id: EntityId,
        writer_id: EntityId,
        gap_start: SequenceNumber,
        gap_list: SequenceNumberSet,
    ) -> Self {
        let flags = [endianness_flag].into();

        let submessage_length = 16 + gap_list.len();

        let header = SubmessageHeader {
            submessage_id: RtpsUdpPsm::GAP.into(),
            flags,
            submessage_length,
        };
        Self {
            header,
            reader_id,
            writer_id,
            gap_start,
            gap_list,
        }
    }

    fn endianness_flag(&self) -> SubmessageFlag {
        self.header.flags.is_bit_set(0)
    }

    fn reader_id(&self) -> &EntityId {
        &self.reader_id
    }

    fn writer_id(&self) -> &EntityId {
        &self.writer_id
    }

    fn gap_start(&self) -> &SequenceNumber {
        &self.gap_start
    }

    fn gap_list(&self) -> &SequenceNumberSet {
        &self.gap_list
    }
}

impl rust_rtps_pim::messages::Submessage<RtpsUdpPsm> for GapSubmessage {
    fn submessage_header(&self) -> SubmessageHeader {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_rtps_pim::messages::submessage_elements::SequenceNumberSetSubmessageElementType;
    use rust_serde_cdr::serializer::RtpsMessageSerializer;
    use serde::Serialize;

    fn get_serializer() -> RtpsMessageSerializer<Vec<u8>> {
        RtpsMessageSerializer {
            writer: Vec::<u8>::new(),
        }
    }

    #[test]
    fn serialize() {
        let endianness_flag = true;
        let reader_id = [1, 2, 3, 4].into();
        let writer_id = [6, 7, 8, 9].into();
        let gap_start = 5.into();
        let gap_list = SequenceNumberSet::new(&10.into(), &[]);
        let submessage: GapSubmessage = rust_rtps_pim::messages::submessages::GapSubmessage::new(
            endianness_flag,
            reader_id,
            writer_id,
            gap_start,
            gap_list,
        );

        let mut serializer = get_serializer();
        submessage.serialize(&mut serializer).unwrap();
        #[rustfmt::skip]
        assert_eq!(serializer.writer, vec![
                0x08_u8, 0b_0000_0001, 28, 0, // Submessage header
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // gapStart: SequenceNumber: high
                5, 0, 0, 0, // gapStart: SequenceNumber: low
                0, 0, 0, 0, // gapList: SequenceNumberSet: bitmapBase: high
               10, 0, 0, 0, // gapList: SequenceNumberSet: bitmapBase: low
                0, 0, 0, 0, // gapList: SequenceNumberSet: numBits (ULong)
            ]
        );
        assert_eq!(
            serializer.writer.len() as u16 - 4,
            submessage.header.submessage_length
        )
    }
}
