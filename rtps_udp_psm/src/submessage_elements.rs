use serde::ser::SerializeStruct;
use rust_rtps_pim::messages::types::SubmessageFlag;
use crate::psm::RtpsUdpPsm;


#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Octet(pub(crate) u8);

impl Octet {
    pub fn is_bit_set(&self, index: usize) -> bool {
        self.0 & (0b_0000_0001 << index) != 0
    }
}

impl<const N: usize> From<[SubmessageFlag; N]> for Octet {
    fn from(value: [SubmessageFlag; N]) -> Self {
        let mut flags = 0b_0000_0000;
        for (i, &item) in value.iter().enumerate() {
            if item {
                flags |= 0b_0000_0001 << i
            }
        }
        Self(flags)
    }
}
impl<const N: usize> From<Octet> for [SubmessageFlag; N] {
    fn from(_value: Octet) -> Self {
        todo!()
    }
}
impl From<Octet> for u8 {
    fn from(value: Octet) -> Self {
        value.0
    }
}
impl From<u8> for Octet {
    fn from(value: u8) -> Self {
        Self(value)
    }
}


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UShort(pub(crate) u16);

impl rust_rtps_pim::messages::submessage_elements::UShortSubmessageElementType for UShort {
    fn new(value: u16) -> Self {
        Self(value)
    }

    fn value(&self) -> &u16 {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Long(pub(crate) i32);

impl rust_rtps_pim::messages::submessage_elements::LongSubmessageElementType for Long {
    fn new(value: i32) -> Self {
        Self(value)
    }

    fn value(&self) -> &i32 {
        &self.0
    }
}

impl From<[u8; 4]> for Long {
    fn from(value: [u8; 4]) -> Self {
        Self(i32::from_le_bytes(value))
    }
}

impl Into<[u8; 4]> for Long {
    fn into(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct ULong(pub(crate) u32);

impl rust_rtps_pim::messages::submessage_elements::ULongSubmessageElementType for ULong {
    fn new(value: u32) -> Self {
        Self(value)
    }

    fn value(&self) -> &u32 {
        &self.0
    }
}

impl From<[u8; 4]> for ULong {
    fn from(value: [u8; 4]) -> Self {
        Self(u32::from_le_bytes(value))
    }
}

impl Into<[u8; 4]> for ULong {
    fn into(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct GuidPrefix(pub(crate) [u8; 12]);

impl rust_rtps_pim::messages::submessage_elements::GuidPrefixSubmessageElementType for GuidPrefix {
    fn new(value: &rust_rtps_pim::structure::types::GuidPrefix) -> Self {
        Self(value.clone())
    }

    fn value(&self) -> &rust_rtps_pim::structure::types::GuidPrefix {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EntityId(pub(crate) rust_rtps_pim::structure::types::EntityId);

impl rust_rtps_pim::messages::submessage_elements::EntityIdSubmessageElementType for EntityId {
    fn new(value: &rust_rtps_pim::structure::types::EntityId) -> Self {
        Self(value.clone())
    }

    fn value(&self) -> &rust_rtps_pim::structure::types::EntityId {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SequenceNumber {
    pub(crate) high: i32,
    pub(crate) low: u32,
}

impl From<SequenceNumber> for i64 {
    fn from(value: SequenceNumber) -> Self {
        ((value.high as i64) << 32) + value.low as i64
    }
}
impl From<i64> for SequenceNumber {
    fn from(value: i64) -> Self {
        Self {
            high: (value >> 32) as i32,
            low: value as u32,
        }
    }
}

impl rust_rtps_pim::messages::submessage_elements::SequenceNumberSubmessageElementType for SequenceNumber {
    fn new(value: rust_rtps_pim::structure::types::SequenceNumber) -> Self {
        value.into()
    }

    fn value(&self) -> rust_rtps_pim::structure::types::SequenceNumber {
        (*self).into()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceNumberSet {
    base: SequenceNumber,
    num_bits: ULong,
    bitmap: [i32; 8],
}

impl SequenceNumberSet {
    pub fn len(&self) -> u16 {
        let number_of_bitmap_elements = ((self.num_bits.0 + 31) / 32) as usize; // aka "M"
        12 /*bitmapBase + numBits */ + 4 * number_of_bitmap_elements /* bitmap[0] .. bitmap[M-1] */ as u16
    }
}

impl serde::Serialize for SequenceNumberSet {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = 2 + self.bitmap.len();

        let mut state = serializer.serialize_struct("SequenceNumberSet", len)?;
        state.serialize_field("bitmapBase", &self.base)?;
        state.serialize_field("numBits", &self.num_bits)?;
        const BITMAP_NAMES: [&str; 8] = [
            "bitmap[0]",
            "bitmap[1]",
            "bitmap[2]",
            "bitmap[3]",
            "bitmap[4]",
            "bitmap[5]",
            "bitmap[6]",
            "bitmap[7]",
        ];
        let number_of_bitmap_elements = ((self.num_bits.0 + 31) / 32) as usize; // aka "M"
        for i in 0..number_of_bitmap_elements {
            state.serialize_field(BITMAP_NAMES[i], &self.bitmap[i])?;
        }
        state.end()
    }
}

struct SequenceNumberSetVisitor;

impl<'de> serde::de::Visitor<'de> for SequenceNumberSetVisitor {
    type Value = SequenceNumberSet;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("SequenceNumberSet Submessage Element")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let base: SequenceNumber = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let num_bits: ULong = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        let num_bitmaps = (num_bits.0 + 31) / 32; //In standard refered to as "M"
        let mut bitmap = [0; 8];
        for i in 0..num_bitmaps as usize {
            let bitmap_i = seq
                .next_element()?
                .ok_or_else(|| serde::de::Error::invalid_length(i + 2, &self))?;
            bitmap[i] = bitmap_i;
        }
        Ok(SequenceNumberSet {
            base,
            num_bits,
            bitmap,
        })
    }
}

impl<'de> serde::Deserialize<'de> for SequenceNumberSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const MAX_BITMAPS: usize = 8;
        const OTHER_FIELDS: usize = 2; /* base + num_bits */
        const MAX_FIELDS: usize = MAX_BITMAPS + OTHER_FIELDS;
        deserializer.deserialize_tuple(MAX_FIELDS, SequenceNumberSetVisitor)
    }
}

impl rust_rtps_pim::messages::submessage_elements::SequenceNumberSetSubmessageElementType for SequenceNumberSet {
    type IntoIter = std::vec::IntoIter<rust_rtps_pim::structure::types::SequenceNumber>;

    fn new(
        base: rust_rtps_pim::structure::types::SequenceNumber,
        set: &[rust_rtps_pim::structure::types::SequenceNumber],
    ) -> Self {
        let mut bitmap = [0; 8];
        let mut num_bits = 0;
        for sequence_number in set.iter() {
            let delta_n = (sequence_number - base) as u32;
            let bitmap_num = delta_n / 32;
            bitmap[bitmap_num as usize] |= 1 << (31 - delta_n % 32);
            if delta_n + 1 > num_bits {
                num_bits = delta_n + 1;
            }
        }
        Self {
            base: base.into(),
            num_bits: ULong(num_bits),
            bitmap,
        }
    }

    fn base(&self) -> rust_rtps_pim::structure::types::SequenceNumber {
        self.base.into()
    }

    fn set(&self) -> Self::IntoIter {
        let mut set = vec![];
        for delta_n in 0..self.num_bits.0 as usize {
            if (self.bitmap[delta_n / 32] & (1 << (31 - delta_n % 32)))
                == (1 << (31 - delta_n % 32))
            {
                let seq_num = Into::<rust_rtps_pim::structure::types::SequenceNumber>::into(self.base) + delta_n as rust_rtps_pim::structure::types::SequenceNumber;
                set.push(seq_num);
            }
        }
        set.into_iter()
    }
}

pub type InstanceHandle = i32;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProtocolVersionC {
    pub major: u8,
    pub minor: u8,
}

impl rust_rtps_pim::messages::submessage_elements::ProtocolVersionSubmessageElementType
    for ProtocolVersionC
{
    type ProtocolVersionType = ProtocolVersionC;

    const PROTOCOLVERSION_1_0: Self::ProtocolVersionType = Self{major: 1, minor: 0};
    const PROTOCOLVERSION_1_1: Self::ProtocolVersionType = Self{major: 1, minor: 1};
    const PROTOCOLVERSION_2_0: Self::ProtocolVersionType = Self{major: 2, minor: 0};
    const PROTOCOLVERSION_2_1: Self::ProtocolVersionType = Self{major: 2, minor: 1};
    const PROTOCOLVERSION_2_2: Self::ProtocolVersionType = Self{major: 2, minor: 2};
    const PROTOCOLVERSION_2_3: Self::ProtocolVersionType = Self{major: 2, minor: 3};
    const PROTOCOLVERSION_2_4: Self::ProtocolVersionType = Self{major: 2, minor: 4};
    fn new(value: &Self::ProtocolVersionType) -> Self {
        todo!()
    }

    fn value(&self) -> &Self::ProtocolVersionType {
        todo!()
    }
}

#[derive(Debug, PartialEq, serde::Deserialize)]
pub struct SerializedData<'a>(pub &'a [u8]);

impl<'a> SerializedData<'a> {
    pub fn len(&self) -> u16 {
        self.0.len() as u16
    }
}

impl<'a> rust_rtps_pim::messages::submessage_elements::SerializedDataSubmessageElementType<'_>
    for SerializedData<'a>
{
    type Value = &'a [u8];

    fn new(value: Self::Value) -> Self {
        Self(value)
    }

    fn value(&self) -> &Self::Value {
        &self.0
    }


}

impl<'a> serde::Serialize for SerializedData<'a> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.0)
    }
}

impl<'a>
    rust_rtps_pim::messages::submessage_elements::SerializedDataFragmentSubmessageElementType<'a>
    for SerializedData<'a>
{
    fn new(value: &'a [u8]) -> Self {
        Self(value.into())
    }

    fn value(&self) -> &[u8] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VendorId(pub(crate) [u8; 2]);

impl rust_rtps_pim::messages::submessage_elements::VendorIdSubmessageElementType for VendorId {
    fn new(value: &rust_rtps_pim::structure::types::VendorId) -> Self {
        Self(value.clone())
    }

    fn value(&self) -> &rust_rtps_pim::structure::types::VendorId {
        &self.0
    }
}


#[derive(Clone, Copy)]
pub struct Time {
    pub seconds: u32,
    pub fraction: u32,
}

impl<'a> rust_rtps_pim::messages::submessage_elements::TimestampSubmessageElementType<RtpsUdpPsm>
    for Time
{
    fn new(value: &Time) -> Self {
        value.clone()
    }

    fn value(&self) -> &Time {
        self
    }
}

#[derive(Debug, PartialEq, Clone, Copy, serde::Serialize)]
pub struct Count(pub(crate) i32);

impl<'a> rust_rtps_pim::messages::submessage_elements::CountSubmessageElementType<RtpsUdpPsm>
    for Count
{
    fn new(value: &Count) -> Self {
        value.clone()
    }

    fn value(&self) -> &Count {
        self
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct FragmentNumber(pub(crate) u32);

impl rust_rtps_pim::messages::submessage_elements::FragmentNumberSubmessageElementType
    for FragmentNumber
{
    fn new(value: &rust_rtps_pim::messages::types::FragmentNumber) -> Self {
        Self(value.clone())
    }

    fn value(&self) -> &rust_rtps_pim::messages::types::FragmentNumber {
        &self.0
    }
}

impl From<u32> for FragmentNumber {
    fn from(_: u32) -> Self {
        todo!()
    }
}

impl Into<u32> for FragmentNumber {
    fn into(self) -> u32 {
        todo!()
    }
}

pub struct FragmentNumberSet(Vec<FragmentNumber>);

impl rust_rtps_pim::messages::submessage_elements::FragmentNumberSetSubmessageElementType
    for FragmentNumberSet
{
    fn new(
        _base: &rust_rtps_pim::messages::types::FragmentNumber,
        _set: &[rust_rtps_pim::messages::types::FragmentNumber],
    ) -> Self {
        todo!()
    }

    fn base(&self) -> &rust_rtps_pim::messages::types::FragmentNumber {
        &0
    }

    fn set(&self) -> &[rust_rtps_pim::messages::types::FragmentNumber] {
        todo!()
        // self
    }
}

pub type GroupDigest = [u8; 4];

#[derive(Clone, Copy)]
pub struct Duration {
    pub seconds: i32,
    pub fraction: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Vector(Vec<u8>);
impl serde::Serialize for Vector {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.0.as_slice())
    }
}

impl From<Vec<u8>> for Vector {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Parameter {
    pub parameter_id: rust_rtps_pim::messages::types::ParameterId,
    pub length: i16,
    pub value: Vector,
}

impl Parameter {
    pub fn new(parameter_id: rust_rtps_pim::messages::types::ParameterId, value: Vector) -> Self {
        Self {
            parameter_id,
            length: value.0.len() as i16,
            value,
        }
    }

    pub fn len(&self) -> u16 {
        4 + self.value.0.len() as u16
    }
}

impl rust_rtps_pim::messages::submessage_elements::ParameterType for Parameter {
    fn parameter_id(&self) -> rust_rtps_pim::messages::types::ParameterId {
        self.parameter_id
    }

    fn length(&self) -> i16 {
        self.length
    }

    fn value(&self) -> &[u8] {
        &self.value.0
    }
}

impl serde::Serialize for Parameter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Parameter", 3)?;
        state.serialize_field("ParameterId", &self.parameter_id)?;
        state.serialize_field("length", &self.length)?;
        state.serialize_field("value", &self.value)?;
        state.end()
    }
}

struct ParameterVisitor;

impl<'de> serde::de::Visitor<'de> for ParameterVisitor {
    type Value = Parameter;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Parameter of the ParameterList Submessage Element")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let paramter_id: rust_rtps_pim::messages::types::ParameterId = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let data_length: u16 = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        let mut data = vec![];
        for _ in 0..data_length {
            data.push(
                seq.next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?,
            );
        }
        Ok(Parameter::new(paramter_id, data.into()))
    }
}

impl<'de> serde::Deserialize<'de> for Parameter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const MAX_BYTES: usize = 2 ^ 16;
        deserializer.deserialize_tuple(MAX_BYTES, ParameterVisitor {})
    }
}
const PID_SENTINEL: rust_rtps_pim::messages::types::ParameterId = 1;
static SENTINEL: Parameter = Parameter {
    parameter_id: PID_SENTINEL,
    length: 0,
    value: Vector(vec![]),
};

#[derive(Debug, PartialEq, Clone)]
pub struct ParameterList {
    pub(crate) parameter: Vec<Parameter>,
}
impl serde::Serialize for ParameterList {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = self.parameter.len();
        let mut state = serializer.serialize_struct("ParameterList", len)?;
        for parameter in &self.parameter {
            state.serialize_field("parameter", &parameter)?;
        }
        state.serialize_field("sentinel", &SENTINEL)?;
        state.end()
    }
}

struct ParameterListVisitor;

impl<'de> serde::de::Visitor<'de> for ParameterListVisitor {
    type Value = ParameterList;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("ParameterList Submessage Element")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut parameters = vec![];
        for _ in 0..seq.size_hint().unwrap() {
            let parameter: Parameter = seq
                .next_element()?
                .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
            if parameter == SENTINEL {
                return Ok(ParameterList {
                    parameter: parameters.into(),
                });
            } else {
                parameters.push(parameter);
            }
        }
        todo!()
    }
}

impl<'de, 'a> serde::Deserialize<'de> for ParameterList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const MAX_PARAMETERS: usize = 2 ^ 16;
        deserializer.deserialize_tuple(MAX_PARAMETERS, ParameterListVisitor {})
    }
}

impl ParameterList {
    pub fn len(&self) -> u16 {
        self.parameter.iter().map(|p| p.len()).sum()
    }
}

impl rust_rtps_pim::messages::submessage_elements::ParameterListSubmessageElementType
    for ParameterList
{
    type Parameter = Parameter;

    fn new(_parameter: &[Self::Parameter]) -> Self {
        //let vec: Vec<Parameter> = parameter.iter().map(|x| x.clone()).collect();
        todo!()
    }

    fn parameter(&self) -> &[Self::Parameter] {
        &self.parameter
    }

    fn empty() -> Self{
        ParameterList {
            parameter: Vec::new(),
        }
    }
}

pub struct LocatorList(Vec<rust_rtps_pim::structure::types::Locator>);

impl rust_rtps_pim::messages::submessage_elements::LocatorListSubmessageElementType
    for LocatorList
{
    fn new(_value: &[rust_rtps_pim::structure::types::Locator]) -> Self {
        // Self(value)
        todo!()
    }

    fn value(&self) -> &[rust_rtps_pim::structure::types::Locator] {
        &self.0
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use rust_rtps_pim::messages::{
        submessage_elements::{
            SequenceNumberSetSubmessageElementType, SequenceNumberSubmessageElementType,
        },
    };
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
    fn octet_from_submessage_flags() {
        let result: Octet = [true, false, true].into();
        assert_eq!(result, Octet(0b_0000_0101));
    }

    #[test]
    fn octet_from_submessage_flags_empty() {
        let result: Octet = [].into();
        assert_eq!(result, Octet(0b_0000_0000));
    }
    #[test]
    #[should_panic]
    fn octet_from_submessage_flags_overflow() {
        let _: Octet = [true; 9].into();
    }

    #[test]
    fn octet_is_set_bit() {
        let flags = Octet(0b_0000_0001);
        assert_eq!(flags.is_bit_set(0), true);

        let flags = Octet(0b_0000_0000);
        assert_eq!(flags.is_bit_set(0), false);

        let flags = Octet(0b_0000_0010);
        assert_eq!(flags.is_bit_set(1), true);

        let flags = Octet(0b_1000_0011);
        assert_eq!(flags.is_bit_set(7), true);
    }
    #[test]
    fn serialize_octet() {
        assert_eq!(serialize(Octet(5)), vec![5]);
    }
    #[test]
    fn deserialize_octet() {
        let result: Octet = deserialize(&[5]);
        assert_eq!(result, Octet(5));
    }

    #[test]
    fn serialize_parameter() {
        let parameter = Parameter::new(2, vec![5, 6, 7, 8].into());
        #[rustfmt::skip]
        assert_eq!(serialize(parameter), vec![
            0x02, 0x00, 4, 0, // Parameter | length
            5, 6, 7, 8,       // value
        ]);
    }

    #[test]
    fn serialize_parameter_list() {
        let parameter = ParameterList {
            parameter: vec![
                Parameter::new(2, vec![51, 61, 71, 81].into()),
                Parameter::new(3, vec![52, 62, 72, 82].into()),
            ]
            .into(),
        };
        #[rustfmt::skip]
        assert_eq!(serialize(parameter), vec![
            0x02, 0x00, 4, 0, // Parameter ID | length
            51, 61, 71, 81,   // value
            0x03, 0x00, 4, 0, // Parameter ID | length
            52, 62, 72, 82,   // value
            0x01, 0x00, 0, 0, // Sentinel: PID_SENTINEL | PID_PAD
        ]);
    }

    #[test]
    fn deserialize_parameter() {
        let expected = Parameter::new(0x02, vec![5, 6, 7, 8].into());
        #[rustfmt::skip]
        let result = deserialize(&[
            0x02, 0x00, 4, 0, // Parameter | length
            5, 6, 7, 8,       // value
        ]);
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter_list() {
        let expected = ParameterList {
            parameter: vec![
                Parameter::new(0x02, vec![15, 16, 17, 18].into()),
                Parameter::new(0x03, vec![25, 26, 27, 28].into()),
            ]
            .into(),
        };
        #[rustfmt::skip]
        let result: ParameterList = deserialize(&[
            0x02, 0x00, 4, 0, // Parameter ID | length
            15, 16, 17, 18,        // value
            0x03, 0x00, 4, 0, // Parameter ID | length
            25, 26, 27, 28,        // value
            0x01, 0x00, 0, 0, // Sentinel: Parameter ID | length
            9, 9, 9,    // Following data
        ]);
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_serialized_data() {
        let data = SerializedData(&[1, 2]);
        assert_eq!(serialize(data), vec![1, 2]);
    }

    #[test]
    fn sequence_number_set_submessage_element_type_constructor() {
        let expected = SequenceNumberSet {
            base: SequenceNumber::new(2),
            num_bits: ULong(0),
            bitmap: [0; 8],
        };
        assert_eq!(SequenceNumberSet::new(2, &[]), expected);

        let expected = SequenceNumberSet {
            base: SequenceNumber::new(2),
            num_bits: ULong(1),
            bitmap: [0b_10000000_00000000_00000000_00000000_u32 as i32, 0, 0, 0, 0, 0, 0, 0],
        };
        assert_eq!(SequenceNumberSet::new(2, &[2]), expected);


        let expected = SequenceNumberSet {
            base: SequenceNumber::new(2),
            num_bits: ULong(256),
            bitmap: [0b_10000000_00000000_00000000_00000000_u32 as i32, 0, 0, 0, 0, 0, 0, 0b_00000000_00000000_00000000_00000001],
        };
        assert_eq!(SequenceNumberSet::new(2, &[2, 257]), expected);
    }

    #[test]
    fn sequence_number_set_submessage_element_type_getters() {
        let sequence_number_set = SequenceNumberSet {
            base: SequenceNumber::new(2),
            num_bits: ULong(0),
            bitmap: [0; 8],
        };
        assert_eq!(sequence_number_set.base(), 2);
        assert!(sequence_number_set.set().eq(Vec::<i64>::new()));

        let sequence_number_set = SequenceNumberSet {
            base: SequenceNumber::new(2),
            num_bits: ULong(100),
            bitmap: [0b_10000000_00000000_00000000_00000000_u32 as i32, 0, 0, 0, 0, 0, 0, 0],
        };
        assert_eq!(sequence_number_set.base(), 2);
        assert!(sequence_number_set.set().eq(vec![2]));

        let sequence_number_set = SequenceNumberSet {
            base: SequenceNumber::new(2),
            num_bits: ULong(256),
            bitmap: [0b_10000000_00000000_00000000_00000000_u32 as i32, 0, 0, 0, 0, 0, 0, 0b_00000000_00000000_00000000_00000001],
        };
        assert_eq!(sequence_number_set.base(), 2);
        assert!(sequence_number_set.set().eq(vec![2, 257]));
    }


    #[test]
    fn serialize_sequence_number_max_gap() {
        let sequence_number_set = SequenceNumberSet::new(2, &[2, 257]);
        #[rustfmt::skip]
        assert_eq!(serialize(sequence_number_set), vec![
            0, 0, 0, 0, // bitmapBase: high (long)
            2, 0, 0, 0, // bitmapBase: low (unsigned long)
            0, 1, 0, 0, // numBits (ULong)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_1000_0000, // bitmap[0] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[1] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[2] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[3] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[4] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[5] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[6] (long)
            0b_000_0001, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[7] (long)
        ]);
    }

    #[test]
    fn serialize_sequence_number_set_empty() {
        let sequence_number_set = SequenceNumberSet::new(2, &[]);
        #[rustfmt::skip]
        assert_eq!(serialize(sequence_number_set), vec![
            0, 0, 0, 0, // bitmapBase: high (long)
            2, 0, 0, 0, // bitmapBase: low (unsigned long)
            0, 0, 0, 0, // numBits (ULong)
        ]);
    }

    #[test]
    fn deserialize_sequence_number_set_empty() {
        let expected = SequenceNumberSet::new(2, &[]);
        #[rustfmt::skip]
        let result = deserialize(&[
            0, 0, 0, 0, // bitmapBase: high (long)
            2, 0, 0, 0, // bitmapBase: low (unsigned long)
            0, 0, 0, 0, // numBits (ULong)
        ]);
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_sequence_number_set_max_gap() {
        let expected = SequenceNumberSet::new(2, &[2, 257]);
        #[rustfmt::skip]
        let result = deserialize(&[
            0, 0, 0, 0, // bitmapBase: high (long)
            2, 0, 0, 0, // bitmapBase: low (unsigned long)
            0, 1, 0, 0, // numBits (ULong)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_1000_0000, // bitmap[0] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[1] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[2] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[3] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[4] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[5] (long)
            0b_000_0000, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[6] (long)
            0b_000_0001, 0b_0000_0000, 0b_0000_0000, 0b_0000_0000, // bitmap[7] (long)

        ]);
        assert_eq!(expected, result);
    }
}