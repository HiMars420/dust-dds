impl UdpPsmMapping for Data {
    fn compose(&self, writer: &mut impl std::io::Write) -> RtpsSerdesResult<()> {
        let endianness = Endianness::from(self.endianness_flag);
        let extra_flags = submessage_elements::UShort(0);
        let octecs_to_inline_qos_size = self.reader_id.octets() + self.writer_id.octets() + self.writer_sn.octets();
        let octecs_to_inline_qos = submessage_elements::UShort(octecs_to_inline_qos_size as u16);
        self.submessage_header().compose(writer)?;
        extra_flags.serialize(writer, endianness)?;
        octecs_to_inline_qos.serialize(writer, endianness)?;
        self.reader_id.serialize(writer, endianness)?;
        self.writer_id.serialize(writer, endianness)?;
        self.writer_sn.serialize(writer, endianness)?;
        
        if self.inline_qos_flag {
            self.inline_qos.serialize(writer, endianness)?;
        }
        if self.data_flag || self.key_flag {
            self.serialized_payload.serialize(writer, endianness)?;
        }

        Ok(())
    }

    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> { 
        let header = SubmessageHeader::parse(bytes)?;
        let flags = header.flags();
        // X|X|X|N|K|D|Q|E
        /*E*/ let endianness_flag = flags[0];
        /*Q*/ let inline_qos_flag = flags[1];
        /*D*/ let data_flag = flags[2];
        /*K*/ let key_flag = flags[3];
        /*N*/ let non_standard_payload_flag = flags[4];

        let endianness = Endianness::from(endianness_flag);

        const HEADER_SIZE : usize = 8;
        let octets_to_inline_qos = usize::from(submessage_elements::UShort::deserialize(&bytes[6..8], endianness)?.0) + HEADER_SIZE /* header and extra flags*/;
        let reader_id = submessage_elements::EntityId::deserialize(&bytes[8..12], endianness)?;        
        let writer_id = submessage_elements::EntityId::deserialize(&bytes[12..16], endianness)?;
        let writer_sn = submessage_elements::SequenceNumber::deserialize(&bytes[16..24], endianness)?;
        let (inline_qos, inline_qos_octets) = if inline_qos_flag {
            let inline_qos = ParameterList::deserialize(&bytes[octets_to_inline_qos..], endianness)?;
            let inline_qos_octets = inline_qos.octets();
            (inline_qos, inline_qos_octets)
        } else { 
            let inline_qos = ParameterList::new();
            (inline_qos, 0)
        };
        let end_of_submessage = usize::from(header.submessage_length()) + header.octets();
        let serialized_payload = if data_flag || key_flag || non_standard_payload_flag {
            let octets_to_serialized_payload = octets_to_inline_qos + inline_qos_octets;
            submessage_elements::SerializedData::deserialize(&bytes[octets_to_serialized_payload..end_of_submessage], endianness)?
        } else {
            submessage_elements::SerializedData(Vec::new())
        };


        Ok(Data {
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline_qos_types::KeyHash;
    use crate::types::constants::{ENTITYID_UNKNOWN, ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER, };

    // E: EndiannessFlag - Indicates endianness.
    // Q: InlineQosFlag - Indicates to the Reader the presence of a ParameterList containing QoS parameters that should be used to interpret the message.
    // D: DataFlag - Indicates to the Reader that the dataPayload submessage element contains the serialized value of the data-object.
    // K: KeyFlag - Indicates to the Reader that the dataPayload submessage element contains the serialized value of the key of the data-object. 
    // N: NonStandardPayloadFlag  -Indicates to the Reader that the serializedPayload submessage element is not formatted according to Section 10.
    // X|X|X|N|K|D|Q|E
    #[test]
    fn test_data_contructor() {
        let data = Data::new(
            Endianness::LittleEndian, 
            ENTITYID_UNKNOWN, 
            ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER, 
            1, 
            Some(ParameterList::new()),
            Payload::Data(vec![])
        );
        assert_eq!(data.endianness_flag, true);
        assert_eq!(data.inline_qos_flag, true);
        assert_eq!(data.data_flag, true);
        assert_eq!(data.key_flag, false);
        assert_eq!(data.non_standard_payload_flag, false);
    }
    #[test]
    fn test_compose_data_submessage_without_inline_qos_without_data() {
        let data = Data {
            endianness_flag: true,
            inline_qos_flag: false,
            data_flag: false,
            key_flag: false,
            non_standard_payload_flag: false,
            reader_id: submessage_elements::EntityId(ENTITYID_UNKNOWN),
            writer_id: submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            writer_sn: submessage_elements::SequenceNumber(1),
            inline_qos: ParameterList::new(), 
            serialized_payload: submessage_elements::SerializedData(Vec::new()), 
        };
        let expected = vec![
            0x15_u8, 0b00000001, 20, 0x0, // Submessgae Header
            0x00, 0x00,  16, 0x0, // ExtraFlags, octetsToInlineQos (little indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
        ];
        let mut result = Vec::new();
        data.compose(&mut result).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_compose_data_submessage_with_inline_qos_without_data() {
        let endianness = Endianness::LittleEndian;
        let key_hash = KeyHash([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
        let mut inline_qos = ParameterList::new();
        inline_qos.push(key_hash);
        
        let data = Data {
            endianness_flag: endianness.into(),
            inline_qos_flag: true,
            data_flag: false,
            key_flag: false,
            non_standard_payload_flag: false,
            reader_id: submessage_elements::EntityId(ENTITYID_UNKNOWN),
            writer_id: submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            writer_sn: submessage_elements::SequenceNumber(1),
            inline_qos: inline_qos,
            serialized_payload: submessage_elements::SerializedData(Vec::new()), 
        };
        let expected = vec![
            0x15_u8, 0b00000011, 44, 0x0, // Submessgae Header
            0x00, 0x00,  16, 0x0, // ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
            0x70, 0x00, 0x10, 0x00, // [Inline QoS] parameterId, length
            1, 2, 3, 4,             // [Inline QoS] Key hash
            5, 6, 7, 8,             // [Inline QoS] Key hash
            9, 10, 11, 12,          // [Inline QoS] Key hash
            13, 14, 15, 16,         // [Inline QoS] Key hash
            0x01, 0x00, 0x00, 0x00  // [Inline QoS] PID_SENTINEL
        ];
        let mut result = Vec::new();
        data.compose(&mut result).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_compose_data_submessage_with_inline_qos_with_data() {
        let endianness = Endianness::LittleEndian;
        let key_hash = KeyHash([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
        let mut inline_qos = ParameterList::new();
        inline_qos.push(key_hash);
        
        let serialized_payload = submessage_elements::SerializedData(vec![1_u8, 2, 3]);

        let data = Data {
            endianness_flag: endianness.into(),
            inline_qos_flag: true,
            data_flag: true,
            key_flag: false,
            non_standard_payload_flag: false,
            reader_id: submessage_elements::EntityId(ENTITYID_UNKNOWN),
            writer_id: submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            writer_sn: submessage_elements::SequenceNumber(1),
            inline_qos: inline_qos, 
            serialized_payload: serialized_payload,
        };
        let expected = vec![
            0x15_u8, 0b00000111, 47, 0x0, // Submessgae Header
            0x00, 0x00,  16, 0x0, // ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
            0x70, 0x00, 0x10, 0x00, // [Inline QoS] parameterId, length
            1, 2, 3, 4,             // [Inline QoS] Key hash
            5, 6, 7, 8,             // [Inline QoS] Key hash
            9, 10, 11, 12,          // [Inline QoS] Key hash
            13, 14, 15, 16,         // [Inline QoS] Key hash
            0x01, 0x00, 0x00, 0x00, // [Inline QoS] PID_SENTINEL
            1, 2, 3,             // [Serialized Payload]
        ];
        let mut result = Vec::new();
        data.compose(&mut result).unwrap();
        assert_eq!(expected, result);
    }


    #[test]
    fn test_parse_data_submessage_without_inline_qos_without_data() {
        let expected = Data {
            endianness_flag: true,
            inline_qos_flag: false,
            data_flag: false,
            key_flag: false,
            non_standard_payload_flag: false,
            reader_id: submessage_elements::EntityId(ENTITYID_UNKNOWN),
            writer_id: submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            writer_sn: submessage_elements::SequenceNumber(1),
            inline_qos: ParameterList::new(), 
            serialized_payload: submessage_elements::SerializedData(Vec::new()), 
        };
        let bytes = vec![
            0x15_u8, 0b00000001, 20, 0x0, // Submessgae Header
            0x00, 0x00,  16, 0x0, // ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
        ];
        let result = Data::parse(&bytes).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_data_submessage_without_inline_qos_with_non_standard_payload() {       
        let serialized_payload = submessage_elements::SerializedData(vec![1_u8, 2, 3, 4]);

        let expected = Data {
            endianness_flag: true,
            inline_qos_flag: false,
            data_flag: false,
            key_flag: false,
            non_standard_payload_flag: true,
            reader_id: submessage_elements::EntityId(ENTITYID_UNKNOWN),
            writer_id: submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            writer_sn: submessage_elements::SequenceNumber(1),
            inline_qos: ParameterList::new(), 
            serialized_payload: serialized_payload, 
        };
        let bytes = vec![
            0x15_u8, 0b00010001, 24, 0x0, // Submessgae Header
            0x00, 0x00,  16, 0x0, // ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
            1, 2, 3, 4,             // [Serialized Payload]
        ];
        let result = Data::parse(&bytes).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_data_submessage_with_inline_qos_with_data() {
        let endianness = Endianness::LittleEndian;
        let key_hash = KeyHash([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
        let mut inline_qos = ParameterList::new();
        inline_qos.push(key_hash);

        
        let serialized_payload = submessage_elements::SerializedData(vec![1_u8, 2, 3]);

        let expected = Data {
            endianness_flag: endianness.into(),
            inline_qos_flag: true,
            data_flag: false,
            key_flag: true,
            non_standard_payload_flag: false,
            reader_id: submessage_elements::EntityId(ENTITYID_UNKNOWN),
            writer_id: submessage_elements::EntityId(ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER),
            writer_sn: submessage_elements::SequenceNumber(1),
            inline_qos: inline_qos, 
            serialized_payload: serialized_payload, 
        };
        let bytes = vec![
            0x15_u8, 0b00001011, 47, 0x0, // Submessgae Header
            0x00, 0x00,  16, 0x0, // ExtraFlags, octetsToInlineQos (liitle indian)
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] EntityId readerId => ENTITYID_UNKNOWN
            0x00, 0x01, 0x00, 0xc2, // [Data Submessage] EntityId writerId
            0x00, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN
            0x01, 0x00, 0x00, 0x00, // [Data Submessage] SequenceNumber writerSN => 1
            0x70, 0x00, 0x10, 0x00, // [Inline QoS] parameterId, length
            1, 2, 3, 4,             // [Inline QoS] Key hash
            5, 6, 7, 8,             // [Inline QoS] Key hash
            9, 10, 11, 12,          // [Inline QoS] Key hash
            13, 14, 15, 16,         // [Inline QoS] Key hash
            0x01, 0x00, 0x00, 0x00, // [Inline QoS] PID_SENTINEL
            1, 2, 3,              // [Serialized Payload]            
            99, 99, 99, 99          // Rubbish Data
        ];
        let result = Data::parse(&bytes).unwrap();
        assert_eq!(expected, result);
    }
}
