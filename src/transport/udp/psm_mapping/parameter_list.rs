use std::io::Write;
use crate::messages::submessages::submessage_elements::Short;
use crate::messages::parameter_list::Parameter;

use super::{TransportEndianness, UdpPsmMappingResult, SizeCheck};
use super::submessage_elements::{serialize_short, deserialize_short};


fn serialize_parameter(parameter: &Parameter, writer: &mut impl Write, endianness: TransportEndianness) -> UdpPsmMappingResult<()> {
    serialize_short(&Short(parameter.parameter_id()), writer, endianness)?;
    serialize_short(&Short(parameter.length()), writer, endianness)?;
    writer.write(parameter.value().as_slice())?;
    let padding = parameter.length() as usize - parameter.value().len();
    for _ in 0..padding {
        writer.write(&[0_u8])?;
    }
    Ok(())
}

fn octets_parameter(parameter: &Parameter) -> usize {       
    4 + parameter.length() as usize
}    

fn deserialize_parameter(bytes: &[u8], endianness: TransportEndianness) -> UdpPsmMappingResult<Parameter> {
    bytes.check_size_bigger_equal_than(4)?;
    let parameter_id = deserialize_short(&bytes[0..2], endianness)?.0;
    let length = deserialize_short(&bytes[2..4], endianness)?;
    let bytes_end = (length.0 + 4) as usize;
    bytes.check_size_bigger_equal_than(bytes_end)?;
    let value = Vec::from(&bytes[4..bytes_end]);
    Ok(Parameter::from_raw(parameter_id, value))
}    

// impl SubmessageElement for ParameterList {
//     fn serialize(&self, writer: &mut impl Write, endianness: Endianness) -> RtpsSerdesResult<()> {
//          for param in self.parameter.iter() {
//             Parameter{parameter_id: param.parameter_id(), length: param.length(), value: param.value(endianness)}.serialize(writer, endianness)?;
//         }       
//         Short(ParameterList::PID_SENTINEL).serialize(writer, endianness)?;
//         writer.write(&[0,0])?; // Sentinel length 0
//         Ok(())
//     }

//     fn octets(&self) -> usize {
//         let mut s = 4; //sentinel
//         self.parameter.iter().for_each(|param| {s += 2 /*param.parameter_id().octets()*/ + 2 /*param.length().octets()*/ + (param.length().0 as usize) });
//         s
//     }   

//     fn deserialize(bytes: &[u8], endianness: Endianness) -> RtpsSerdesResult<Self> {
//         bytes.check_size_bigger_equal_than(2)?;
//         let mut parameter_start_index: usize = 0;
//         let mut parameters = ParameterList::new();
//         loop {
//             let parameter = Parameter::deserialize(&bytes[parameter_start_index..], endianness)?;          
//             if &parameter.parameter_id == &ParameterList::PID_SENTINEL {
//                 break;
//             }            
//             parameter_start_index += parameter.octets();
//             parameters.push(parameter);
//         }
//         Ok(parameters)
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline_qos_types::{StatusInfo, KeyHash, };
    use crate::messages::types::{ParameterId, };
    use serde::{Serialize, Deserialize, };
    use crate::messages::parameter_list::Pid;
    use crate::serialized_payload::CdrEndianness;
    // use super::super::serdes::RtpsSerdesError;

    #[allow(overflowing_literals)]
    const PID_VENDOR_TEST_0 : ParameterId = 0x0000 | 0x8000;
    #[allow(overflowing_literals)]
    const PID_VENDOR_TEST_1 : ParameterId = 0x0001 | 0x8000;
    #[allow(overflowing_literals)]
    const PID_VENDOR_TEST_3 : ParameterId = 0x0003 | 0x8000;
    #[allow(overflowing_literals)]
    const PID_VENDOR_TEST_4 : ParameterId = 0x0004 | 0x8000;
    #[allow(overflowing_literals)]
    const PID_VENDOR_TEST_5 : ParameterId = 0x0005 | 0x8000;
    #[allow(overflowing_literals)]
    const PID_VENDOR_TEST_SHORT : ParameterId = 0x0006 | 0x8000;

     // /////////////////////// ParameterList Tests ////////////////////////
    
     #[derive(Debug, PartialEq, Serialize, Deserialize)]
     pub struct VendorTest0(pub [u8; 0]);
     impl Pid for VendorTest0 {
         fn pid() -> ParameterId {
             PID_VENDOR_TEST_0
         }
     }
     
     #[derive(Debug, PartialEq, Serialize, Deserialize)]
     pub struct VendorTest1(pub [u8; 1]);
     impl Pid for VendorTest1 {
         fn pid() -> ParameterId {
             PID_VENDOR_TEST_1
         }
     }
     
     #[derive(Debug, PartialEq, Serialize, Deserialize)]
     pub struct VendorTest3(pub [u8; 3]);
     impl Pid for VendorTest3 {
         fn pid() -> ParameterId {
             PID_VENDOR_TEST_3
         }
     }
     
     #[derive(Debug, PartialEq, Serialize)]
     pub struct VendorTest4(pub [u8; 4]);
     impl Pid for VendorTest4 {
         fn pid() -> ParameterId {
             PID_VENDOR_TEST_4
         }
     }
     
     #[derive(Debug, PartialEq, Serialize, Deserialize)]
     pub struct VendorTest5(pub [u8; 5]);
     impl Pid for VendorTest5 {
         fn pid() -> ParameterId {
             PID_VENDOR_TEST_5
         }
     }
 
     #[derive(Debug, PartialEq, Serialize, Deserialize)]
     pub struct VendorTestShort(pub i16);
     impl Pid for VendorTestShort {
         fn pid() -> ParameterId {
             PID_VENDOR_TEST_SHORT
         }
     }
 
     #[test]
     fn serialize_deserialize_parameter() {
         let parameter = Parameter::new(&VendorTest3([1, 2, 3]), CdrEndianness::LittleEndian);
     
         let expected = vec![
             0x03, 0x80, 4, 0, //ParameterID, length
             1, 2, 3, 0, //VendorTest value       
         ];
         let mut writer = Vec::new();
         serialize_parameter(&parameter, &mut writer, TransportEndianness::LittleEndian).unwrap();
         assert_eq!(expected, writer);

         let bytes = vec![
             0x03, 0x80, 4, 0, //ParameterID, length
             1, 2, 3, 0, //VendorTest value       
         ];    
         let expected = VendorTest3([1, 2, 3]);    
         let parameter = deserialize_parameter(&bytes, TransportEndianness::LittleEndian).unwrap();
         let result = parameter.get(CdrEndianness::LittleEndian).unwrap();
     
         assert_eq!(expected, result);
     }
     
     #[test]
     fn deserialize_parameter_liitle_endian() {  
         let bytes = vec![
             0x71, 0x0, 4, 0, 
             1, 2, 3, 4,
         ];
         let expected = Parameter::new(&StatusInfo([1, 2, 3, 4]), CdrEndianness::LittleEndian);
         let result = deserialize_parameter(&bytes, TransportEndianness::LittleEndian).unwrap();   
         assert_eq!(expected, result);
     }    
 
     #[test]
     fn serialize_parameter_short() {
         let parameter = Parameter::new(&VendorTestShort(-1000), CdrEndianness::LittleEndian);
         let expected = vec![
             0x06, 0x80, 4, 0, 
             0x18, 0xFC, 0, 0,
         ];
         let mut writer = Vec::new();
         serialize_parameter(&parameter, &mut writer, TransportEndianness::LittleEndian).unwrap();
         assert_eq!(expected, writer);
 
         let parameter = Parameter::new(&VendorTestShort(-1000), CdrEndianness::BigEndian);
         let expected = vec![
             0x80, 0x06, 0, 4, 
             0xFC, 0x18, 0, 0,
         ];
         let mut writer = Vec::new();
         serialize_parameter(&parameter, &mut writer, TransportEndianness::BigEndian).unwrap();
         assert_eq!(expected, writer);
     }
 
     #[test]
     fn deserialize_parameter_short() {
         let expected = VendorTestShort(-1000);
         let bytes = vec![
             0x06, 0x80, 4, 0, 
             0x18, 0xFC, 0, 0,
         ];
         let parameter = deserialize_parameter(&bytes, TransportEndianness::LittleEndian).unwrap();
         let result = parameter.get(CdrEndianness::LittleEndian).unwrap();
         assert_eq!(expected, result);
 
         let expected = VendorTestShort(-1000);
         let bytes = vec![
             0x80, 0x06, 0, 4, 
             0xFC, 0x18, 0, 0,
         ];
         let parameter = deserialize_parameter(&bytes, TransportEndianness::BigEndian).unwrap();
         let result = parameter.get(CdrEndianness::BigEndian).unwrap();
         assert_eq!(expected, result);
     }
 
     #[test]
     fn parameter_serialize_little_endian() {
         let parameter = Parameter::new(&VendorTest3([1, 2, 3]), CdrEndianness::LittleEndian);
         let expected_bytes = vec![
             0x03, 0x80, 4, 0, 
             1, 2, 3, 0,
         ];
         let expected_octets = expected_bytes.len();
         let mut result_bytes = Vec::new();
         serialize_parameter(&parameter, &mut result_bytes, TransportEndianness::LittleEndian).unwrap();
         let result_octets = octets_parameter(&parameter);
         assert_eq!(expected_bytes, result_bytes);
         assert_eq!(expected_octets, result_octets);
     
         let parameter = Parameter::new(&VendorTest0([]), CdrEndianness::LittleEndian);
         let expected_bytes = vec![0x00, 0x80, 0, 0, ];
         let expected_octets = expected_bytes.len();
         let mut result_bytes = Vec::new();
         serialize_parameter(&parameter, &mut result_bytes, TransportEndianness::LittleEndian).ok();
         let result_octets = octets_parameter(&parameter);
         assert_eq!(expected_bytes, result_bytes);
         assert_eq!(expected_octets, result_octets);
     
         let parameter = Parameter::new(&VendorTest4([1,2,3,4]), CdrEndianness::LittleEndian);
         let expected_bytes = vec![
             0x04, 0x80, 4, 0, 
             1, 2, 3, 4,
         ];
         let expected_octets = expected_bytes.len();
         let mut result_bytes = Vec::new();
         serialize_parameter(&parameter, &mut result_bytes, TransportEndianness::LittleEndian).ok();
         let result_octets = octets_parameter(&parameter);
         assert_eq!(expected_bytes, result_bytes);
         assert_eq!(expected_octets, result_octets);
     
         let parameter = Parameter::new(&VendorTest5([1,2,3,4,5]), CdrEndianness::LittleEndian);
         let expected_bytes = vec![
             0x05, 0x80, 8, 0, 
             1, 2, 3, 4,
             5, 0, 0, 0, 
         ];
         let expected_octets = expected_bytes.len();
         let mut result_bytes = Vec::new();
         serialize_parameter(&parameter, &mut result_bytes, TransportEndianness::LittleEndian).ok();
         let result_octets = octets_parameter(&parameter);
         assert_eq!(expected_bytes, result_bytes);
         assert_eq!(expected_octets, result_octets);
     }
         
    
 
    //  #[test]
    //  fn parameter_list_deserialize_liitle_endian() {
    //      let endianness = Endianness::LittleEndian;
    //      let bytes = vec![
    //          0x03, 0x80, 4, 0, // ParameterID, length 
    //          1, 2, 3, 0, // value
    //          0x05, 0x80, 8, 0, // ParameterID, length 
    //          10, 20, 30, 40, // value
    //          50, 0, 0, 0, // value
    //          0x01, 0x00, 0, 0, // Sentinel
    //      ];  
    //      let expected1 = VendorTest3([1, 2, 3]);
    //      let expected2 = VendorTest5([10, 20, 30, 40, 50]);
     
    //      // Note that the complete list cannot simply be checked for equality here
    //      // this is because the value of each parameter needs to be interpreted for that
    //      // otherwise the value containes the padding 0's 
    //      let result = ParameterList::deserialize(&bytes, endianness).unwrap(); 
    //      let result1 = result.find::<VendorTest3>(endianness).unwrap();
    //      let result2 = result.find::<VendorTest5>(endianness).unwrap();
    //      assert_eq!(result1, expected1);
    //      assert_eq!(result2, expected2);
    //  }
 
    //  #[test]
    //  fn serialize_parameter_list() {
    //      let endianness = Endianness::LittleEndian;
    //      use crate::inline_qos_types::KeyHash;
    //      let key_hash = KeyHash([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    //      let status_info = StatusInfo([101, 102, 103, 104]);
    //      let vendor1 = VendorTest1([b'X']);
    //      let vendor5 = VendorTest5([2;5]);
    //      let parameter_list = ParameterList{parameter: vec![Rc::new(key_hash), Rc::new(status_info), Rc::new(vendor1), Rc::new(vendor5)]};
     
    //      let mut writer = Vec::new();
    //      parameter_list.serialize(&mut writer, endianness).unwrap();
     
    //      let expected = vec![
    //          0x70, 0x00, 16, 0, //ParameterID, length
    //          1, 2, 3, 4, //key_hash
    //          5, 6, 7, 8,  //key_hash
    //          9, 10, 11, 12,  //key_hash
    //          13, 14, 15, 16, //key_hash
    //          0x71, 0x00, 4, 0, //ParameterID, length
    //          101, 102, 103, 104, //status_info
    //          0x01, 0x80, 4, 0, //ParameterID, length
    //          b'X', 0, 0, 0, //vendor1
    //          0x05, 0x80, 8, 0, //ParameterID, length
    //          2, 2, 2, 2, //vendor5
    //          2, 0, 0, 0, //vendor5
    //          1, 0, 0, 0 // Sentinel
    //      ];
    //      assert_eq!(expected, writer);
    //  }
     
    //  #[test]
    //  fn deserialize_parameter_list() {
    //      let endianness = Endianness::LittleEndian;
    //      use crate::inline_qos_types::KeyHash;
    //      let key_hash = KeyHash([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    //      let status_info = StatusInfo([101, 102, 103, 104]);
     
    //      let mut expected = ParameterList::new();
    //      expected.push(key_hash);
    //      expected.push(status_info);
 
    //      let bytes = vec![
    //          0x70, 0x00, 16, 0, //ParameterID, length
    //          1, 2, 3, 4, //key_hash
    //          5, 6, 7, 8,  //key_hash
    //          9, 10, 11, 12,  //key_hash
    //          13, 14, 15, 16, //key_hash
    //          0x71, 0x00, 4, 0, //ParameterID, length
    //          101, 102, 103, 104, //status_info
    //          1, 0, 0, 0 // Sentinel
    //      ];
         
    //      let result = ParameterList::deserialize(&bytes, endianness).unwrap();
    //      assert_eq!(expected, result);
    // }
}