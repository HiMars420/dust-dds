impl UdpPsmMapping for InfoSource {
    fn compose(&self, writer: &mut impl std::io::Write) -> RtpsSerdesResult<()> {
        let endianness = self.endianness_flag.into();
        let unused = submessage_elements::Long(0);
        self.submessage_header().compose(writer)?;
        unused.serialize(writer, endianness)?;
        self.protocol_version.serialize(writer, endianness)?;
        self.vendor_id.serialize(writer, endianness)?;
        self.guid_prefix.serialize(writer, endianness)?;
        Ok(())
    }

    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        let header = SubmessageHeader::parse(bytes)?;
        let endianness_flag = header.flags()[0];
        let endianness = Endianness::from(endianness_flag);
        let _unused = submessage_elements::ULong::deserialize(&bytes[4..8], endianness)?;
        let protocol_version = submessage_elements::ProtocolVersion::deserialize(&bytes[8..10], endianness)?;
        let vendor_id = submessage_elements::VendorId::deserialize(&bytes[10..12], endianness)?;
        let guid_prefix = submessage_elements::GuidPrefix::deserialize(&bytes[12..24], endianness)?;        

        Ok(InfoSource {
            endianness_flag,
            protocol_version,
            vendor_id,
            guid_prefix,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::constants;

    #[test]
    fn parse_heartbeat_frag_submessage() {
        let expected = InfoSource {
            endianness_flag: true,    
            protocol_version: submessage_elements::ProtocolVersion(constants::PROTOCOL_VERSION_2_4),
            vendor_id: submessage_elements::VendorId(constants::VENDOR_ID),
            guid_prefix: submessage_elements::GuidPrefix([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        };
        let bytes = vec![
            0x0c, 0b00000001, 20, 0x0, // Submessgae Header
            0x00, 0x00, 0x00, 0x00, // unused
             2,  4, 99, 99, // protocol_version | vendor_id
             1,  2,  3,  4, // guid_prefix
             5,  6,  7,  8, // guid_prefix 
             9, 10, 11, 12, // guid_prefix
        ];
        let result = InfoSource::parse(&bytes).unwrap();
        assert_eq!(expected, result);
    }

    
    #[test]
    fn compose_heartbeat_frag_submessage() {
        let message = InfoSource {
            endianness_flag: true,    
            protocol_version: submessage_elements::ProtocolVersion(constants::PROTOCOL_VERSION_2_4),
            vendor_id: submessage_elements::VendorId(constants::VENDOR_ID),
            guid_prefix: submessage_elements::GuidPrefix([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        };
        let expected = vec![
            0x0c, 0b00000001, 20, 0x0, // Submessgae Header
            0x00, 0x00, 0x00, 0x00, // unused
             2,  4, 99, 99, // protocol_version | vendor_id
             1,  2,  3,  4, // guid_prefix
             5,  6,  7,  8, // guid_prefix 
             9, 10, 11, 12, // guid_prefix
        ];
        let mut writer = Vec::new();
        message.compose(&mut writer).unwrap();
        assert_eq!(expected, writer);
    }

}
