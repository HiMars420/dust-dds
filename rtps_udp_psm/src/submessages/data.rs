use std::io::Write;
use byteorder::ByteOrder;
use rust_rtps_pim::messages::{
    submessages::DataSubmessage,
    types::{SubmessageFlag, },
    RtpsSubmessageHeader, Submessage,
};

use crate::{parameter_list::{ParameterListUdp}, submessage_elements::{EntityIdUdp, SequenceNumberUdp, SerializedDataUdp, flags_to_byte, is_bit_set}, submessage_header::{DATA, SubmessageHeaderUdp}};

#[derive(Debug, PartialEq)]
pub struct DataSubmesageUdp<'a> {
    pub(crate) header: SubmessageHeaderUdp,
    extra_flags: u16,
    octets_to_inline_qos: u16,
    reader_id: EntityIdUdp,
    writer_id: EntityIdUdp,
    writer_sn: SequenceNumberUdp,
    inline_qos: ParameterListUdp<'a>,
    serialized_payload: SerializedDataUdp<'a>,
}

impl<'a> crate::serialize::Serialize for DataSubmesageUdp<'a> {
    fn serialize<W: Write, B: ByteOrder>(&self, mut _writer: W) -> crate::serialize::Result {
        todo!()
    }
}
impl<'a:'de, 'de> crate::deserialize::Deserialize<'de> for DataSubmesageUdp<'a> {
    fn deserialize<B>(_buf: &mut &'de[u8]) -> crate::deserialize::Result<Self> where B: ByteOrder {
        todo!()
    }
}

impl<'a> rust_rtps_pim::messages::submessages::DataSubmessage<'a> for DataSubmesageUdp<'a> {
    type EntityIdSubmessageElementType = EntityIdUdp;
    type SequenceNumberSubmessageElementType = SequenceNumberUdp;
    type ParameterListSubmessageElementType = ParameterListUdp<'a>;
    type SerializedDataSubmessageElementType = SerializedDataUdp<'a>;

    fn new(
        endianness_flag: SubmessageFlag,
        inline_qos_flag: SubmessageFlag,
        data_flag: SubmessageFlag,
        key_flag: SubmessageFlag,
        non_standard_payload_flag: SubmessageFlag,
        reader_id: Self::EntityIdSubmessageElementType,
        writer_id: Self::EntityIdSubmessageElementType,
        writer_sn: Self::SequenceNumberSubmessageElementType,
        inline_qos: Self::ParameterListSubmessageElementType,
        serialized_payload: Self::SerializedDataSubmessageElementType,
    ) -> Self {
        let flags = flags_to_byte([
            endianness_flag,
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
        ]);
        let inline_qos_len = if inline_qos_flag {
            inline_qos.number_of_bytes()
        } else {
            0
        };
        let serialized_payload_len_padded = serialized_payload.len() + 3 &!3; //ceil to multiple of 4
        let submessage_length = 20 + inline_qos_len + serialized_payload_len_padded;
        let header = SubmessageHeaderUdp {
            submessage_id: DATA,
            flags,
            submessage_length: submessage_length as u16,
        };

        DataSubmesageUdp {
            header,
            extra_flags: 0b_0000_0000_0000_0000,
            octets_to_inline_qos: 16,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        }
    }

    fn endianness_flag(&self) -> SubmessageFlag {
        is_bit_set(self.header.flags, 0)
    }

    fn inline_qos_flag(&self) -> SubmessageFlag {
        is_bit_set(self.header.flags, 1)
    }

    fn data_flag(&self) -> SubmessageFlag {
        is_bit_set(self.header.flags, 2)
    }

    fn key_flag(&self) -> SubmessageFlag {
        is_bit_set(self.header.flags, 3)
    }

    fn non_standard_payload_flag(&self) -> SubmessageFlag {
        is_bit_set(self.header.flags, 4)
    }

    fn reader_id(&self) -> &EntityIdUdp {
        &self.reader_id
    }

    fn writer_id(&self) -> &EntityIdUdp {
        &self.writer_id
    }

    fn writer_sn(&self) -> &SequenceNumberUdp {
        &self.writer_sn
    }

    fn inline_qos(&self) -> &Self::ParameterListSubmessageElementType {
        todo!()
    }

    fn serialized_payload(&self) -> &SerializedDataUdp<'a> {
        &self.serialized_payload
    }
}

impl<'a> Submessage for DataSubmesageUdp<'a> {
    fn submessage_header(&self) -> RtpsSubmessageHeader {
        todo!()
    }
}

impl<'a> serde::Serialize for DataSubmesageUdp<'a> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        // let mut len = 6;
        // if self.inline_qos_flag() {
        //     len += 1
        // };
        // if self.data_flag() || self.key_flag() {
        //     len += 1;
        // };
        // let mut state = serializer.serialize_struct("Data", len)?;
        // state.serialize_field("header", &self.header)?;
        // state.serialize_field("extra_flags", &self.extra_flags)?;
        // state.serialize_field("octets_to_inline_qos", &self.octets_to_inline_qos)?;
        // state.serialize_field("reader_id", &self.reader_id)?;
        // state.serialize_field("writer_id", &self.writer_id)?;
        // state.serialize_field("writer_sn", &self.writer_sn)?;
        // if self.inline_qos_flag() {
        //     state.serialize_field("inline_qos", &self.inline_qos)?;
        // }
        // if self.data_flag() || self.key_flag() {
        //     state.serialize_field("serialized_payload", &self.serialized_payload)?;
        //     // Pad to 32bit boundary
        //     let pad_length = (4 - self.serialized_payload.len() % 4) & 0b11; // ceil to the nearest multiple of 4
        //     let pad = vec![0; pad_length as usize];
        //     state.serialize_field("pad", &SerializedDataUdp(pad.as_slice()))?;
        // }

        // state.end()
        todo!()
    }
}

struct DataSubmesageVisitor<'a>(std::marker::PhantomData<&'a ()>);

struct Bytes(usize);
impl<'de> serde::de::DeserializeSeed<'de> for Bytes{
    type Value = &'de [u8];

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de> {
            struct BytesVisitor;

        impl<'a> serde::de::Visitor<'a> for BytesVisitor {
            type Value = &'a [u8];

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a borrowed byte array")
            }

            fn visit_borrowed_bytes<E>(self, v: &'a [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }
        }
        deserializer.deserialize_tuple_struct("", self.0, BytesVisitor)
    }
}

impl<'a, 'de: 'a> serde::de::Visitor<'de> for DataSubmesageVisitor<'a> {
    type Value = DataSubmesageUdp<'a>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("DataSubmesage")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        // let header: SubmessageHeaderUdp = seq
        //     .next_element()?
        //     .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        // let inline_qos_flag = is_bit_set(header.flags, 1);
        // let data_flag = is_bit_set(header.flags, 2);
        // let key_flag = is_bit_set(header.flags, 3);
        // // let non_standard_payload_flag = header.flags.is_bit_set(4);
        // let extra_flags: u16 = seq
        //     .next_element()?
        //     .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        // let octets_to_inline_qos: u16 = seq
        //     .next_element()?
        //     .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
        // let reader_id: EntityIdUdp = seq
        //     .next_element()?
        //     .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;
        // let writer_id: EntityIdUdp = seq
        //     .next_element()?
        //     .ok_or_else(|| serde::de::Error::invalid_length(4, &self))?;
        // let writer_sn: SequenceNumberUdp = seq
        //     .next_element()?
        //     .ok_or_else(|| serde::de::Error::invalid_length(5, &self))?;


        // let inline_qos: ParameterListUdp = if inline_qos_flag {
        //     seq
        //     .next_element()?
        //     .ok_or_else(|| serde::de::Error::invalid_length(6, &self))?
        // } else {
        //     ParameterListUdp { parameter: vec![] }
        // };
        // let inline_qos_len = if inline_qos_flag {
        //     inline_qos.number_of_bytes()
        // } else {
        //     0
        // };

        // let serialized_payload: SerializedDataUdp = if data_flag || key_flag {
        //     let serialized_payload_length =
        //     header.submessage_length as usize - octets_to_inline_qos as usize - 4 - inline_qos_len;

        //     let data: &[u8] = seq.next_element_seed(Bytes(serialized_payload_length))?
        //         .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;

        //     SerializedDataUdp(&data)
        // } else {
        //     SerializedDataUdp(&[])
        // };

        // Ok(DataSubmesageUdp {
        //     header,
        //     extra_flags,
        //     octets_to_inline_qos,
        //     reader_id,
        //     writer_id,
        //     writer_sn,
        //     inline_qos,
        //     serialized_payload,
        // })
        todo!()
    }
}

impl<'a, 'de: 'a> serde::Deserialize<'de> for DataSubmesageUdp<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &'static [&'static str] = &[
            "header",
            "extra_flags",
            "octets_to_inline_qos",
            "reader_id",
            "writer_id",
            "writer_sn",
            "inline_qos",
            "serialized_payload",
        ];
        deserializer.deserialize_struct(
            "DataSubmesage",
            FIELDS,
            DataSubmesageVisitor(std::marker::PhantomData),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::parameter_list::{ParameterUdp};

    use super::*;
    use rust_rtps_pim::messages::submessage_elements::SequenceNumberSubmessageElementType;
    use rust_serde_cdr::{
        deserializer::RtpsMessageDeserializer, serializer::RtpsMessageSerializer,
    };

    fn serialize<T: serde::Serialize>(value: T) -> Vec<u8> {
        let mut serializer = RtpsMessageSerializer {
            writer: Vec::<u8>::new(),
        };
        value.serialize(&mut serializer).unwrap();
        serializer.writer
    }
    fn deserialize<'de, T: serde::Deserialize<'de>>(buffer: &'de [u8]) -> T {
        let mut de = RtpsMessageDeserializer { reader: buffer };
        serde::de::Deserialize::deserialize(&mut de).unwrap()
    }

    #[test]
    fn serialize_no_inline_qos_no_serialized_payload() {
        let endianness_flag = true;
        let inline_qos_flag = false;
        let data_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let writer_sn = SequenceNumberUdp::new(&5);
        let inline_qos = ParameterListUdp {
            parameter: vec![].into(),
        };
        let data = [];
        let serialized_payload = SerializedDataUdp(data[..].into());
        let submessage = DataSubmesageUdp::new(
            endianness_flag,
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        assert_eq!(serialize(submessage), vec![
                0x15_u8, 0b_0000_0001, 20, 0, // Submessage header
                0, 0, 16, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
            ]
        );
    }

    #[test]
    fn serialize_with_inline_qos_no_serialized_payload() {
        let endianness_flag = true;
        let inline_qos_flag = true;
        let data_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let writer_sn = SequenceNumberUdp::new(&5);
        let param1 = ParameterUdp::new(6, &[10, 11, 12, 13]);
        let param2 = ParameterUdp::new(7, &[20, 21, 22, 23]);
        let inline_qos = ParameterListUdp {
            parameter: vec![param1.into(), param2.into()],
        };
        let data = [];
        let serialized_payload = SerializedDataUdp(data[..].into());
        let submessage = DataSubmesageUdp::new(
            endianness_flag,
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        assert_eq!(serialize(submessage), vec![
                0x15, 0b_0000_0011, 40, 0, // Submessage header
                0, 0, 16, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
                6, 0, 4, 0, // inlineQos: parameterId_1, length_1
                10, 11, 12, 13, // inlineQos: value_1[length_1]
                7, 0, 4, 0, // inlineQos: parameterId_2, length_2
                20, 21, 22, 23, // inlineQos: value_2[length_2]
                1, 0, 0, 0, // inlineQos: Sentinel
            ]
        );
    }

    #[test]
    fn serialize_no_inline_qos_with_serialized_payload() {
        let endianness_flag = true;
        let inline_qos_flag = false;
        let data_flag = true;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let writer_sn = SequenceNumberUdp::new(&5);
        let inline_qos = ParameterListUdp {
            parameter: vec![].into(),
        };
        let data = [1_u8, 2, 3, 4];
        let serialized_payload = SerializedDataUdp(data[..].into());
        let submessage = DataSubmesageUdp::new(
            endianness_flag,
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        assert_eq!(serialize(submessage), vec![
                0x15, 0b_0000_0101, 24, 0, // Submessage header
                0, 0, 16, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
                1, 2, 3, 4, // serialized payload
            ]
        );
    }

    #[test]
    fn serialize_no_inline_qos_with_serialized_payload_non_multiple_of_4() {
        let endianness_flag = true;
        let inline_qos_flag = false;
        let data_flag = true;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let writer_sn = SequenceNumberUdp::new(&5);
        let inline_qos = ParameterListUdp {
            parameter: vec![].into(),
        };
        let data = [1_u8, 2, 3];
        let serialized_payload = SerializedDataUdp(data[..].into());
        let submessage = DataSubmesageUdp::new(
            endianness_flag,
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        assert_eq!(serialize(submessage), vec![
                0x15, 0b_0000_0101, 24, 0, // Submessage header
                0, 0, 16, 0, // extraFlags, octetsToInlineQos
                1, 2, 3, 4, // readerId: value[4]
                6, 7, 8, 9, // writerId: value[4]
                0, 0, 0, 0, // writerSN: high
                5, 0, 0, 0, // writerSN: low
                1, 2, 3, 0, // serialized payload
            ]
        );
    }

    #[test]
    fn deserialize_no_inline_qos_no_serialized_payload() {
        let endianness_flag = true;
        let inline_qos_flag = false;
        let data_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let writer_sn = SequenceNumberUdp::new(&5);
        let inline_qos = ParameterListUdp {
            parameter: vec![],
        };
        let serialized_payload = SerializedDataUdp(&[]);
        let expected = DataSubmesageUdp::new(
            endianness_flag,
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        let result = deserialize(&[
            0x15, 0b_0000_0001, 20, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
        ]);
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_no_inline_qos_with_serialized_payload() {
        let endianness_flag = true;
        let inline_qos_flag = false;
        let data_flag = true;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let writer_sn = SequenceNumberUdp::new(&5);
        let inline_qos = ParameterListUdp {
            parameter: vec![].into(),
        };
        let data = [1, 2, 3, 4];
        let serialized_payload = SerializedDataUdp(data[..].into());
        let expected = DataSubmesageUdp::new(
            endianness_flag,
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        let result = deserialize(&[
            0x15, 0b_0000_0101, 24, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            1, 2, 3, 4, // SerializedPayload
        ]);
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_with_inline_qos_no_serialized_payload() {
        let endianness_flag = true;
        let inline_qos_flag = true;
        let data_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let writer_sn = SequenceNumberUdp::new(&5);
        let param1 = ParameterUdp::new(6, &[10, 11, 12, 13]);
        let param2 = ParameterUdp::new(7, &[20, 21, 22, 23]);
        let inline_qos = ParameterListUdp {
            parameter: vec![param1.into(), param2.into()],
        };
        let serialized_payload = SerializedDataUdp([][..].into());
        let expected = DataSubmesageUdp::new(
            endianness_flag,
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        #[rustfmt::skip]
        let result = deserialize(&[
            0x15, 0b_0000_0011, 40, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            6, 0, 4, 0, // inlineQos: parameterId_1, length_1
            10, 11, 12, 13, // inlineQos: value_1[length_1]
            7, 0, 4, 0, // inlineQos: parameterId_2, length_2
            20, 21, 22, 23, // inlineQos: value_2[length_2]
            1, 0, 0, 0, // inlineQos: Sentinel
        ]);
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_no_inline_qos_no_serialized_payload_json() {
        let endianness_flag = true;
        let inline_qos_flag = false;
        let data_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityIdUdp {
            entity_key: [1, 2, 3],
            entity_kind: 4,
        };
        let writer_id = EntityIdUdp {
            entity_key: [6, 7, 8],
            entity_kind: 9,
        };
        let writer_sn = SequenceNumberUdp::new(&5);
        let inline_qos = ParameterListUdp {
            parameter: vec![].into(),
        };
        let data = [];
        let serialized_payload = SerializedDataUdp(data[..].into());
        let submessage = DataSubmesageUdp::new(
            endianness_flag,
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        assert_eq!(
            serde_json::ser::to_string(&submessage).unwrap(),
            r#"{"header":{"submessage_id":21,"flags":1,"submessage_length":20},"extra_flags":0,"octets_to_inline_qos":16,"reader_id":{"entity_key":[1,2,3],"entity_kind":4},"writer_id":{"entity_key":[6,7,8],"entity_kind":9},"writer_sn":{"high":0,"low":5}}"#
        );
    }


    // #[test]
    // fn deserialize_no_inline_qos_no_serialized_payload_json() {
    //     let endianness_flag = true;
    //     let inline_qos_flag = false;
    //     let data_flag = false;
    //     let key_flag = false;
    //     let non_standard_payload_flag = false;
    //     let reader_id = EntityIdUdp {
    //         entity_key: [Octet(1), Octet(2), Octet(3)],
    //         entity_kind: Octet(4),
    //     };
    //     let writer_id = EntityIdUdp {
    //         entity_key: [Octet(6), Octet(7), Octet(8)],
    //         entity_kind: Octet(9),
    //     };
    //     let writer_sn = SequenceNumberUdp::new(&5);
    //     let inline_qos = ParameterListUdp {
    //         parameter: vec![].into(),
    //     };
    //     let data = [];
    //     let serialized_payload = SerializedDataUdp(data[..].into());
    //     let expected = DataSubmesageUdp::new(
    //         endianness_flag,
    //         inline_qos_flag,
    //         data_flag,
    //         key_flag,
    //         non_standard_payload_flag,
    //         reader_id,
    //         writer_id,
    //         writer_sn,
    //         inline_qos,
    //         serialized_payload,
    //     );
    //     let json_str = r#"{"header":{"submessage_id":21,"flags":1,"submessage_length":20},"extra_flags":0,"octets_to_inline_qos":16,"reader_id":{"entity_key":[1,2,3],"entity_kind":4},"writer_id":{"entity_key":[6,7,8],"entity_kind":9},"writer_sn":{"high":0,"low":5}}"#;
    //     let result:DataSubmesageUdp  = serde_json::de::from_str(json_str).unwrap();

    //     assert_eq!(expected, result);
    // }
}
