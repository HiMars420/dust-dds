use std::io::Write;

use byteorder::ByteOrder;
use rust_rtps_pim::messages::{types::SubmessageFlag, RtpsSubmessageHeader};

use crate::{
    submessage_elements::{flags_to_byte, is_bit_set, CountUdp, EntityIdUdp, SequenceNumberUdp},
    submessage_header::{SubmessageHeaderUdp, HEARTBEAT},
};

#[derive(Debug, PartialEq)]
pub struct HeartbeatSubmessageUdp {
    pub header: SubmessageHeaderUdp,
    reader_id: EntityIdUdp,
    writer_id: EntityIdUdp,
    first_sn: SequenceNumberUdp,
    last_sn: SequenceNumberUdp,
    count: CountUdp,
}

impl crate::serialize::Serialize for HeartbeatSubmessageUdp {
    fn serialize<W: Write, B: ByteOrder>(&self, mut writer: W) -> crate::serialize::Result {
        self.header.serialize::<_, B>(&mut writer)?;
        self.reader_id.serialize::<_, B>(&mut writer)?;
        self.writer_id.serialize::<_, B>(&mut writer)?;
        self.first_sn.serialize::<_, B>(&mut writer)?;
        self.last_sn.serialize::<_, B>(&mut writer)?;
        self.count.serialize::<_, B>(&mut writer)
    }
}
impl<'de> crate::deserialize::Deserialize<'de> for HeartbeatSubmessageUdp {
    fn deserialize<B>(buf: &mut &'de[u8]) -> crate::deserialize::Result<Self> where B: ByteOrder {
        let header = crate::deserialize::Deserialize::deserialize::<B>(buf)?;
        let reader_id = crate::deserialize::Deserialize::deserialize::<B>(buf)?;
        let writer_id = crate::deserialize::Deserialize::deserialize::<B>(buf)?;
        let first_sn = crate::deserialize::Deserialize::deserialize::<B>(buf)?;
        let last_sn = crate::deserialize::Deserialize::deserialize::<B>(buf)?;
        let count = crate::deserialize::Deserialize::deserialize::<B>(buf)?;
        Ok(Self{ header, reader_id, writer_id, first_sn, last_sn, count })
    }
}

impl<'a> rust_rtps_pim::messages::submessages::HeartbeatSubmessageTrait for HeartbeatSubmessageUdp {
    type EntityIdSubmessageElementType = EntityIdUdp;
    type SequenceNumberSubmessageElementType = SequenceNumberUdp;
    type CountSubmessageElementType = CountUdp;

    fn new(
        endianness_flag: SubmessageFlag,
        final_flag: SubmessageFlag,
        liveliness_flag: SubmessageFlag,
        reader_id: EntityIdUdp,
        writer_id: EntityIdUdp,
        first_sn: SequenceNumberUdp,
        last_sn: SequenceNumberUdp,
        count: CountUdp,
    ) -> Self {
        let flags = flags_to_byte([endianness_flag, final_flag, liveliness_flag]);
        let submessage_length = 28;
        let header = SubmessageHeaderUdp {
            submessage_id: HEARTBEAT,
            flags,
            submessage_length,
        };
        Self {
            header,
            reader_id,
            writer_id,
            first_sn,
            last_sn,
            count,
        }
    }

    fn endianness_flag(&self) -> SubmessageFlag {
        is_bit_set(self.header.flags, 0)
    }

    fn final_flag(&self) -> SubmessageFlag {
        is_bit_set(self.header.flags, 1)
    }

    fn liveliness_flag(&self) -> SubmessageFlag {
        is_bit_set(self.header.flags, 2)
    }

    fn reader_id(&self) -> &EntityIdUdp {
        &self.reader_id
    }

    fn writer_id(&self) -> &EntityIdUdp {
        &self.writer_id
    }

    fn first_sn(&self) -> &SequenceNumberUdp {
        &self.first_sn
    }

    fn last_sn(&self) -> &SequenceNumberUdp {
        &self.last_sn
    }

    fn count(&self) -> &CountUdp {
        &self.count
    }
}

impl rust_rtps_pim::messages::Submessage for HeartbeatSubmessageUdp {
    fn submessage_header(&self) -> RtpsSubmessageHeader {
        todo!()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialize::to_bytes_le;
    use rust_rtps_pim::messages::submessage_elements::SequenceNumberSubmessageElementType;

    #[test]
    fn serialize() {
        let endianness_flag = true;
        let final_flag = false;
        let liveliness_flag = false;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let first_sn = SequenceNumberUdp::new(&1);
        let last_sn = SequenceNumberUdp::new(&3);
        let count = CountUdp(5);
        let submessage: HeartbeatSubmessageUdp =
            rust_rtps_pim::messages::submessages::HeartbeatSubmessageTrait::new(
                endianness_flag,
                final_flag,
                liveliness_flag,
                reader_id,
                writer_id,
                first_sn,
                last_sn,
                count,
            );
        #[rustfmt::skip]
        assert_eq!(
            to_bytes_le(&submessage).unwrap(), vec![
                0x07_u8, 0b_0000_0001, 28, 0, // Submessage header
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // firstSN: SequenceNumber: high
                1, 0, 0, 0, // firstSN: SequenceNumber: low
                0, 0, 0, 0, // lastSN: SequenceNumber: high
                3, 0, 0, 0, // lastSN: SequenceNumber: low
                5, 0, 0, 0, // count: Count: value (long)
            ]
        );
    }
}
