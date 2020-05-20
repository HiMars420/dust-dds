use std::convert::TryInto;
use std::slice::Iter;
use std::ops::Index;
use std::collections::BTreeMap;
use std::io::Write;
use num_derive::FromPrimitive;

use crate::serdes::{RtpsSerialize, RtpsDeserialize, RtpsParse, RtpsCompose, EndianessFlag, RtpsSerdesResult, RtpsSerdesError, PrimitiveSerdes, SizeCheckers, SizeSerializer};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ushort(pub u16);

impl RtpsSerialize for Ushort
{
    fn serialize(&self, writer: &mut impl std::io::Write, endianness: EndianessFlag) -> RtpsSerdesResult<()>{
        let value = self.0;
        writer.write(&PrimitiveSerdes::serialize_u16(value, endianness))?;
        Ok(())
    }
}

impl From<Ushort> for usize {
    fn from(value: Ushort) -> Self {
        value.0 as usize
    }
}

impl RtpsDeserialize for Ushort {
    fn deserialize(bytes: &[u8], endianness: EndianessFlag) -> RtpsSerdesResult<Self> { 
        let value = PrimitiveSerdes::deserialize_u16(bytes[0..2].try_into()?, endianness);
        Ok(Ushort(value))
    }
}


#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct EntityKey(pub [u8;3]);

impl RtpsSerialize for EntityKey
{
    fn serialize(&self, writer: &mut impl std::io::Write, _endianness: EndianessFlag) -> RtpsSerdesResult<()>{
        writer.write(&self.0)?;

        Ok(())
    }
}

impl RtpsParse for EntityKey{
    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        SizeCheckers::check_size_equal(bytes, 3)?;

        Ok(EntityKey(bytes[0..3].try_into()?))
    }
}

#[derive(FromPrimitive, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum EntityKind {
    UserDefinedUnknown = 0x00,
    UserDefinedWriterWithKey = 0x02,
    UserDefinedWriterNoKey = 0x03,
    UserDefinedReaderWithKey = 0x04,
    UserDefinedReaderNoKey = 0x07,
    UserDefinedWriterGroup = 0x08,
    UserDefinedReaderGroup = 0x09,
    BuiltInUnknown = 0xc0,
    BuiltInParticipant = 0xc1,
    BuiltInWriterWithKey = 0xc2,
    BuiltInWriterNoKey = 0xc3,
    BuiltInReaderWithKey = 0xc4,
    BuiltInReaderNoKey = 0xc7,
    BuiltInWriterGroup = 0xc8,
    BuiltInReaderGroup = 0xc9,
}

impl RtpsSerialize for EntityKind
{
    fn serialize(&self, writer: &mut impl std::io::Write, _endianness: EndianessFlag) -> RtpsSerdesResult<()>{
        let entity_kind_u8 = *self as u8;
        writer.write(&[entity_kind_u8])?;

        Ok(())
    }
}

impl RtpsParse for EntityKind{

    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        SizeCheckers::check_size_equal(bytes, 1 /*expected_size*/)?;
        Ok(num::FromPrimitive::from_u8(bytes[0]).ok_or(RtpsSerdesError::InvalidEnumRepresentation)?)
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct EntityId {
    entity_key: EntityKey,
    entity_kind: EntityKind,
}

impl EntityId {
    pub fn new(entity_key: EntityKey, entity_kind: EntityKind) -> EntityId {
        EntityId {
            entity_key,
            entity_kind,
        }
    }
}

impl RtpsSerialize for EntityId {
    fn serialize(&self, writer: &mut impl std::io::Write, endianness: EndianessFlag) -> RtpsSerdesResult<()>{
        self.entity_key.serialize(writer, endianness)?;
        self.entity_kind.serialize(writer, endianness)
    }
}


impl RtpsParse for EntityId{
    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        SizeCheckers::check_size_equal(bytes, 4 /*expected_size*/)?;
        let entity_key = EntityKey::parse(&bytes[0..3])?;
        let entity_kind = EntityKind::parse(&[bytes[3]])?;

        Ok(EntityId::new(entity_key, entity_kind))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct SequenceNumber(pub i64);

impl std::ops::Sub<i64> for SequenceNumber {
    type Output = SequenceNumber;

    fn sub(self, rhs: i64) -> Self::Output {
        SequenceNumber(self.0 - rhs)
    }
}

impl RtpsSerialize for SequenceNumber
{
    fn serialize(&self, writer: &mut impl std::io::Write, endianness: EndianessFlag) -> RtpsSerdesResult<()>{
        let msb = PrimitiveSerdes::serialize_i32((self.0 >> 32) as i32, endianness);
        let lsb = PrimitiveSerdes::serialize_u32((self.0 & 0x0000_0000_FFFF_FFFF) as u32, endianness);

        writer.write(&msb)?;
        writer.write(&lsb)?;

        Ok(())
    }
}

impl RtpsDeserialize for SequenceNumber {
    fn deserialize(bytes: &[u8], endianness: EndianessFlag) -> RtpsSerdesResult<Self> {
        SizeCheckers::check_size_equal(bytes, 8)?;

        let msb = PrimitiveSerdes::deserialize_i32(bytes[0..4].try_into()?, endianness);
        let lsb = PrimitiveSerdes::deserialize_u32(bytes[4..8].try_into()?, endianness);

        let sequence_number = ((msb as i64) << 32) + lsb as i64;

        Ok(SequenceNumber(sequence_number))
    }
}

pub enum TopicKind {
    NoKey,
    WithKey,
}

pub enum ReliabilityKind {
    BestEffort,
    Reliable,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub enum ChangeKind {
    Alive,
    AliveFiltered,
    NotAliveDisposed,
    NotAliveUnregistered,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Time {
    pub seconds: u32,
    pub fraction: u32,
}

impl Time {
    fn new (seconds: u32, fraction: u32) -> Self {
        Time {
            seconds,
            fraction,
        }
    }
}
 
impl RtpsSerialize for Time 
{
    fn serialize(&self, writer: &mut impl std::io::Write, endianness: EndianessFlag) -> RtpsSerdesResult<()>{
        let seconds_bytes = PrimitiveSerdes::serialize_u32(self.seconds, endianness);
        let fraction_bytes = PrimitiveSerdes::serialize_u32(self.fraction, endianness);

        writer.write(&seconds_bytes)?;
        writer.write(&fraction_bytes)?;

        Ok(())
    }
}

impl RtpsDeserialize for Time {
    fn deserialize(bytes: &[u8], endianness: EndianessFlag) -> RtpsSerdesResult<Self> {
        SizeCheckers::check_size_equal(bytes, 8)?;

        let seconds = PrimitiveSerdes::deserialize_u32(bytes[0..4].try_into()?, endianness);
        let fraction = PrimitiveSerdes::deserialize_u32(bytes[4..8].try_into()?, endianness);

        Ok(Time::new(seconds, fraction))
    }
}
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Count(pub i32);

impl RtpsSerialize for Count {
    fn serialize(&self, writer: &mut impl std::io::Write, endianness: EndianessFlag) -> RtpsSerdesResult<()> {
        writer.write(&PrimitiveSerdes::serialize_i32(self.0, endianness))?;

        Ok(())
    }
}

impl RtpsDeserialize for Count {
    fn deserialize(bytes: &[u8], endianness: EndianessFlag) -> RtpsSerdesResult<Self> {
        let value = PrimitiveSerdes::deserialize_i32(bytes.try_into()?, endianness);

        Ok(Count(value))
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Duration {
    pub seconds: i32,
    pub fraction: u32,
}

#[derive(Debug, PartialEq)]
pub struct KeyHash([u8; 16]);

impl KeyHash {
    pub fn new(value: [u8;16]) -> Self {
        KeyHash(value)
    }

    pub fn get_value(&self) -> &[u8;16] {
        &self.0
    }
}

impl RtpsSerialize for KeyHash 
{
    fn serialize(&self, writer: &mut impl std::io::Write, _endianess: EndianessFlag) -> RtpsSerdesResult<()> {
        writer.write(&self.0)?;

        Ok(())
    }


}

#[derive(Debug, PartialEq)]
pub struct StatusInfo([u8;4]);

impl StatusInfo {
    pub fn new(value: [u8;4]) -> Self {
        StatusInfo(value)
    }

    pub fn get_value(&self) -> &[u8;4] {
        &self.0
    }
}

impl RtpsSerialize for StatusInfo 
{
    fn serialize(&self, writer: &mut impl std::io::Write, _endianess: EndianessFlag) -> RtpsSerdesResult<()> {
        writer.write(&self.0)?;

        Ok(())
    }


}

pub trait Parameter
where
    Self: std::marker::Sized
{
    fn new_from(parameter_id: u16, value: &[u8]) -> Option<Self>;

    fn parameter_id(&self) -> u16;

    fn value(&self) -> &[u8];
}

#[derive(Hash, Clone, Debug, PartialEq, Eq)]
pub struct ParameterList<T: Parameter>(Vec<T>);

impl<T: Parameter> ParameterList<T> {
    const PID_PAD: u16 = 0x0000;
    const PID_SENTINEL: u16 = 0x0001;

    pub fn new() -> Self {
        ParameterList(Vec::new())
    }

    pub fn new_from_vec(value: Vec<T>) -> Self {
        ParameterList(value)
    }

    pub fn iter(&self) -> Iter<'_,T>{
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, value: T) {
        self.0.push(value);
    }

    pub fn is_valid(&self) -> bool {
        todo!()
    }
}

impl<T> Index<usize> for ParameterList<T> 
where
    T: Parameter
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.0.index(index)
    }
}

impl<T> RtpsSerialize for ParameterList<T> 
where
    T: RtpsSerialize + Parameter,
{
    fn serialize(&self, writer: &mut impl std::io::Write, endianness: EndianessFlag) -> RtpsSerdesResult<()> {
        for item in self.iter() {
            item.serialize(writer, endianness)?;
        }

        writer.write(&PrimitiveSerdes::serialize_u16(Self::PID_SENTINEL, endianness))?;
        writer.write(&[0,0])?;

        Ok(())
    }
}

impl<T> RtpsSerialize for T 
where
    T: Parameter
{
    fn serialize(&self, writer: &mut impl std::io::Write, endianness: EndianessFlag) -> RtpsSerdesResult<()> {
        let mut size_serializer =  SizeSerializer::new();

        writer.write(&PrimitiveSerdes::serialize_u16(self.parameter_id(), endianness))?;
        
        //TODO: The size needs to be rounded to multiples of 4 and include padding
        size_serializer.write(self.value())?;
        writer.write(&PrimitiveSerdes::serialize_u16(size_serializer.get_size() as u16, endianness))?;

        writer.write(self.value())?;

        Ok(())
    }
}

impl<T: Parameter> RtpsDeserialize for ParameterList<T> 
{
    fn deserialize(bytes: &[u8], endianness: EndianessFlag) -> RtpsSerdesResult<Self> {
        SizeCheckers::check_size_bigger_equal_than(bytes, 4)?;

        let mut parameter_start_index: usize = 0;
        let mut parameter_list = ParameterList::<T>::new();

        loop {
            let parameter_id_first_index = parameter_start_index + 0;
            let parameter_id_last_index = parameter_start_index + 1;
            let parameter_size_first_index = parameter_start_index + 2;
            let parameter_size_last_index = parameter_start_index + 3;
 
            let parameter_id_u16 = PrimitiveSerdes::deserialize_u16(bytes[parameter_id_first_index..=parameter_id_last_index].try_into()?, endianness);
            let parameter_size = PrimitiveSerdes::deserialize_u16(bytes[parameter_size_first_index..=parameter_size_last_index].try_into()?, endianness) as usize;

            if parameter_id_u16 == Self::PID_SENTINEL {
                break;
            }

            let parameter_value_first_index = parameter_start_index + 4;
            let parameter_value_last_index = parameter_value_first_index + parameter_size;

            SizeCheckers::check_size_bigger_equal_than(bytes,parameter_value_last_index)?;

            // For the new_from do a non_inclusive retrieval of the bytes
            if let Some(parameter) = T::new_from(parameter_id_u16, &bytes[parameter_value_first_index..parameter_value_last_index]) {
                parameter_list.push(parameter);
            }

            parameter_start_index = parameter_value_last_index;

        }

        Ok(parameter_list)
    }
}

#[derive(PartialEq, Debug)]
struct RepresentationIdentifier([u8; 2]);

#[derive(PartialEq, Debug)]
struct RepresentationOptions([u8; 2]);

#[derive(PartialEq, Debug)]
struct SerializedPayloadHeader {
    representation_identifier: RepresentationIdentifier,
    representation_options: RepresentationOptions,
}

#[derive(PartialEq, Debug)]
struct StandardSerializedPayload {
    header: SerializedPayloadHeader,
    data: Vec<u8>,
}

impl RtpsSerialize for StandardSerializedPayload {
    fn serialize(&self, _writer: &mut impl std::io::Write, _endianness: EndianessFlag) -> RtpsSerdesResult<()> { todo!() }
    fn octets(&self) -> usize { todo!() }
}

impl RtpsDeserialize for StandardSerializedPayload {
    fn deserialize(_bytes: &[u8], _endianness: EndianessFlag) -> RtpsSerdesResult<Self> { 
        todo!() 
    }
}


#[derive(PartialEq, Debug)]
pub struct SerializedPayload(pub Vec<u8>);

impl RtpsCompose for SerializedPayload {
    fn compose(&self, writer: &mut impl std::io::Write) -> RtpsSerdesResult<()> { 
        writer.write(self.0.as_slice())?;
        Ok(())
    }
}

impl RtpsParse for SerializedPayload {
    fn parse(bytes: &[u8]) -> RtpsSerdesResult<Self> {
        Ok(SerializedPayload(Vec::from(bytes)))
    }
}

#[derive(PartialEq, Debug, Clone, Copy, Hash, Eq)]
pub struct ProtocolVersion {
    pub major: u8,
    pub minor: u8,
}

#[derive(PartialEq, Hash, Eq, Debug, Copy, Clone)]
pub struct Locator {
    pub kind: i32,
    pub port: u32,
    pub address: [u8; 16],
}

impl Locator {
    pub fn new(kind: i32, port: u32, address: [u8; 16]) -> Locator {
        Locator {
            kind,
            port,
            address,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct GUID {
    prefix: GuidPrefix,
    entity_id: EntityId,
}

impl GUID {
    pub fn new(prefix: GuidPrefix, entity_id: EntityId) -> GUID {
        GUID { prefix, entity_id }
    }

    pub fn prefix(&self) -> &GuidPrefix {
        &self.prefix
    }

    pub fn entity_id(&self) -> &EntityId {
        &self.entity_id
    }
}

#[derive(PartialEq, Debug, Eq, Hash)]
pub struct BuiltInEndPointSet {
    value: u32,
}

pub enum BuiltInEndPoints {
    ParticipantAnnouncer = 0,
    ParticipantDetector = 1,
    PublicationsAnnouncer = 2,
    PublicationsDetector = 3,
    SubscriptionsAnnouncer = 4,
    SubscriptionsDetector = 5,

    /*
    The following have been deprecated in version 2.4 of the
    specification. These bits should not be used by versions of the
    protocol equal to or newer than the deprecated version unless
    they are used with the same meaning as in versions prior to the
    deprecated version.
    @position(6) DISC_BUILTIN_ENDPOINT_PARTICIPANT_PROXY_ANNOUNCER,
    @position(7) DISC_BUILTIN_ENDPOINT_PARTICIPANT_PROXY_DETECTOR,
    @position(8) DISC_BUILTIN_ENDPOINT_PARTICIPANT_STATE_ANNOUNCER,
    @position(9) DISC_BUILTIN_ENDPOINT_PARTICIPANT_STATE_DETECTOR,
    */
    ParticipantMessageDataWriter = 10,
    ParticipantMessageDataReader = 11,

    /*
    Bits 12-15 have been reserved by the DDS-Xtypes 1.2 Specification
    and future revisions thereof.
    Bits 16-27 have been reserved by the DDS-Security 1.1 Specification
    and future revisions thereof.
    */
    TopicsAnnouncer = 28,
    TopicsDetector = 29,
}

impl BuiltInEndPointSet {
    pub fn new(value: u32) -> Self {
        BuiltInEndPointSet { value }
    }

    pub fn has(&self, endpoint: BuiltInEndPoints) -> bool {
        let bit_position = endpoint as u8;
        let bitmask = 1 << bit_position;
        (self.value & bitmask) >> bit_position == 1
    }
}

pub type InstanceHandle = [u8; 16];
pub type VendorId = [u8; 2];
pub type LocatorList = Vec<Locator>;
pub type GuidPrefix = [u8; 12];


pub type SequenceNumberSet = BTreeMap<SequenceNumber, bool>;
pub type FragmentNumber = u32;
pub type FragmentNumberSet = Vec<(FragmentNumber, bool)>;

#[cfg(test)]
mod tests {
    use super::*;

    ///////////////////////// Entity Key Tests ////////////////////////
    #[test]
    fn test_entity_key_serialization_deserialization_big_endian() {
        let mut vec = Vec::new();
        let test_entity_key = EntityKey([5,20,250]);

        
        const TEST_ENTITY_KEY_BIG_ENDIAN : [u8;3] = [5,20,250];
        test_entity_key.serialize(&mut vec, EndianessFlag::BigEndian).unwrap();
        assert_eq!(vec, TEST_ENTITY_KEY_BIG_ENDIAN);
        assert_eq!(EntityKey::parse(&vec).unwrap(), test_entity_key);
    }

    #[test]
    fn test_entity_key_serialization_deserialization_little_endian() {
        let mut vec = Vec::new();
        let test_entity_key = EntityKey([5,20,250]);

        
        const TEST_ENTITY_KEY_BIG_ENDIAN : [u8;3] = [5,20,250];
        test_entity_key.serialize(&mut vec, EndianessFlag::LittleEndian).unwrap();
        assert_eq!(vec, TEST_ENTITY_KEY_BIG_ENDIAN);
        assert_eq!(EntityKey::parse(&vec).unwrap(), test_entity_key);
    }

    #[test]
    fn test_invalid_entity_key_deserialization() {
        let too_big_vec = vec![1,2,3,4];

        let expected_error = EntityKey::parse(&too_big_vec);
        match expected_error {
            Err(RtpsSerdesError::WrongSize) => assert!(true),
            _ => assert!(false),
        };

        let too_small_vec = vec![1,2,3,4];

        let expected_error = EntityKey::parse(&too_small_vec);
        match expected_error {
            Err(RtpsSerdesError::WrongSize) => assert!(true),
            _ => assert!(false),
        };
    }


    ///////////////////////// Entity Kind Tests ////////////////////////

    #[test]
    fn test_entity_kind_serialization_deserialization_big_endian() {
        let mut vec = Vec::new();
        let test_entity_kind = EntityKind::BuiltInWriterWithKey;

        
        const TEST_ENTITY_KIND_BIG_ENDIAN : [u8;1] = [0xc2];
        test_entity_kind.serialize(&mut vec, EndianessFlag::BigEndian).unwrap();
        assert_eq!(vec, TEST_ENTITY_KIND_BIG_ENDIAN);
        assert_eq!(EntityKind::parse(&vec).unwrap(), test_entity_kind);
    }

    #[test]
    fn test_entity_kind_serialization_deserialization_little_endian() {
        let mut vec = Vec::new();
        let test_entity_kind = EntityKind::BuiltInWriterWithKey;

        
        const TEST_ENTITY_KIND_LITTLE_ENDIAN : [u8;1] = [0xc2];
        test_entity_kind.serialize(&mut vec, EndianessFlag::LittleEndian).unwrap();
        assert_eq!(vec, TEST_ENTITY_KIND_LITTLE_ENDIAN);
        assert_eq!(EntityKind::parse(&vec).unwrap(), test_entity_kind);
    }

    #[test]
    fn test_invalid_entity_kind_deserialization() {
        let too_big_vec = vec![1,2,3,4];

        let expected_error = EntityKind::parse(&too_big_vec);
        match expected_error {
            Err(RtpsSerdesError::WrongSize) => assert!(true),
            _ => assert!(false),
        };

        let wrong_vec = vec![0xf3];

        let expected_error = EntityKind::parse(&wrong_vec);
        match expected_error {
            Err(RtpsSerdesError::InvalidEnumRepresentation) => assert!(true),
            _ => assert!(false),
        };
    }

    ///////////////////////// Entity Id Tests ////////////////////////
    #[test]
    fn test_entity_id_serialization_deserialization_big_endian() {
        let mut vec = Vec::new();
        let test_entity_id = constants::ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER;

        const TEST_ENTITY_ID_BIG_ENDIAN : [u8;4] = [0, 0x02, 0x00, 0xc4];
        test_entity_id.serialize(&mut vec, EndianessFlag::BigEndian).unwrap();
        assert_eq!(vec, TEST_ENTITY_ID_BIG_ENDIAN);
        assert_eq!(EntityId::parse(&vec).unwrap(), test_entity_id);
    }

    #[test]
    fn test_entity_id_serialization_deserialization_little_endian() {
        let mut vec = Vec::new();
        let test_entity_id = constants::ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER;

        const TEST_ENTITY_ID_LITTLE_ENDIAN : [u8;4] = [0, 0x02, 0x00, 0xc4];
        test_entity_id.serialize(&mut vec, EndianessFlag::LittleEndian).unwrap();
        assert_eq!(vec, TEST_ENTITY_ID_LITTLE_ENDIAN);
        assert_eq!(EntityId::parse(&vec).unwrap(), test_entity_id);
    }

    #[test]
    fn test_invalid_entity_id_deserialization() {
        let too_big_vec = vec![1,2,3,4,5,5];

        let expected_error = EntityId::parse(&too_big_vec);
        match expected_error {
            Err(RtpsSerdesError::WrongSize) => assert!(true),
            _ => assert!(false),
        };

        let wrong_vec = vec![1,2,3,0xf3];

        let expected_error = EntityId::parse(&wrong_vec);
        match expected_error {
            Err(RtpsSerdesError::InvalidEnumRepresentation) => assert!(true),
            _ => assert!(false),
        };
    }


    ///////////////////////// Time Tests ////////////////////////
    #[test]
    fn test_time_serialization_deserialization_big_endian() {
        let mut vec = Vec::new();
        let test_time = Time::new(1234567, 98765432);

        
        const TEST_TIME_BIG_ENDIAN : [u8;8] = [0x00, 0x12, 0xD6, 0x87, 0x05, 0xE3, 0x0A, 0x78];
        test_time.serialize(&mut vec, EndianessFlag::BigEndian).unwrap();
        assert_eq!(vec, TEST_TIME_BIG_ENDIAN);
        assert_eq!(Time::deserialize(&vec, EndianessFlag::BigEndian).unwrap(), test_time);
    }

    #[test]
    fn test_time_serialization_deserialization_little_endian() {
        let mut vec = Vec::new();
        let test_time = Time::new(1234567, 98765432);
        
        const TEST_TIME_LITTLE_ENDIAN : [u8;8] = [0x87, 0xD6, 0x12, 0x00, 0x78, 0x0A, 0xE3, 0x05];
        test_time.serialize(&mut vec, EndianessFlag::LittleEndian).unwrap();
        assert_eq!(vec, TEST_TIME_LITTLE_ENDIAN);
        assert_eq!(Time::deserialize(&vec, EndianessFlag::LittleEndian).unwrap(), test_time);
    }

    #[test]
    fn test_invalid_time_deserialization() {
        let wrong_vec = vec![1,2,3,4];

        let expected_error = Time::deserialize(&wrong_vec, EndianessFlag::LittleEndian);
        match expected_error {
            Err(RtpsSerdesError::WrongSize) => assert!(true),
            _ => assert!(false),
        };
    }

    ///////////////////////// Sequence Number Tests ////////////////////////
    #[test]
    fn test_sequence_number_serialization_deserialization_big_endian() {
        let mut vec = Vec::new();
        let test_sequence_number = SequenceNumber(1987612345679);

        
        const TEST_SEQUENCE_NUMBER_BIG_ENDIAN : [u8;8] = [0x00, 0x00, 0x01, 0xCE, 0xC6, 0xED, 0x85, 0x4F];
        test_sequence_number.serialize(&mut vec, EndianessFlag::BigEndian).unwrap();
        assert_eq!(vec, TEST_SEQUENCE_NUMBER_BIG_ENDIAN);
        assert_eq!(SequenceNumber::deserialize(&vec, EndianessFlag::BigEndian).unwrap(), test_sequence_number);
    }

    #[test]
    fn test_sequence_number_serialization_deserialization_little_endian() {
        let mut vec = Vec::new();
        let test_sequence_number = SequenceNumber(1987612345679);

        
        const TEST_SEQUENCE_NUMBER_LITTLE_ENDIAN : [u8;8] = [0xCE, 0x01, 0x00, 0x00, 0x4F, 0x85, 0xED, 0xC6];
        test_sequence_number.serialize(&mut vec, EndianessFlag::LittleEndian).unwrap();
        assert_eq!(vec, TEST_SEQUENCE_NUMBER_LITTLE_ENDIAN);
        assert_eq!(SequenceNumber::deserialize(&vec, EndianessFlag::LittleEndian).unwrap(), test_sequence_number);
    }

    #[test]
    fn test_sequence_number_serialization_deserialization_multiple_combinations() {
        let mut vec = Vec::new();
        
        {
            let test_sequence_number_i64_max = SequenceNumber(std::i64::MAX);
            test_sequence_number_i64_max.serialize(&mut vec, EndianessFlag::LittleEndian).unwrap();
            assert_eq!(SequenceNumber::deserialize(&vec, EndianessFlag::LittleEndian).unwrap(), test_sequence_number_i64_max);
            vec.clear();

            test_sequence_number_i64_max.serialize(&mut vec, EndianessFlag::BigEndian).unwrap();
            assert_eq!(SequenceNumber::deserialize(&vec, EndianessFlag::BigEndian).unwrap(), test_sequence_number_i64_max);
            vec.clear();
        }

        {
            let test_sequence_number_i64_min = SequenceNumber(std::i64::MIN);
            test_sequence_number_i64_min.serialize(&mut vec, EndianessFlag::LittleEndian).unwrap();
            assert_eq!(SequenceNumber::deserialize(&vec, EndianessFlag::LittleEndian).unwrap(), test_sequence_number_i64_min);
            vec.clear();

            test_sequence_number_i64_min.serialize(&mut vec, EndianessFlag::BigEndian).unwrap();
            assert_eq!(SequenceNumber::deserialize(&vec, EndianessFlag::BigEndian).unwrap(), test_sequence_number_i64_min);
            vec.clear();
        }

        {
            let test_sequence_number_zero = SequenceNumber(0);
            test_sequence_number_zero.serialize(&mut vec, EndianessFlag::LittleEndian).unwrap();
            assert_eq!(SequenceNumber::deserialize(&vec, EndianessFlag::LittleEndian).unwrap(), test_sequence_number_zero);
            vec.clear();

            test_sequence_number_zero.serialize(&mut vec, EndianessFlag::BigEndian).unwrap();
            assert_eq!(SequenceNumber::deserialize(&vec, EndianessFlag::BigEndian).unwrap(), test_sequence_number_zero);
            vec.clear();
        }
    }

    #[test]
    fn test_invalid_sequence_number_deserialization() {
        let wrong_vec = vec![1,2,3,4];

        let expected_error = SequenceNumber::deserialize(&wrong_vec, EndianessFlag::LittleEndian);
        match expected_error {
            Err(RtpsSerdesError::WrongSize) => assert!(true),
            _ => assert!(false),
        };
    }

    #[test]
    fn test_builtin_endpoint_set_participant_announcer() {
        assert_eq!(
            BuiltInEndPointSet::new(0).has(BuiltInEndPoints::ParticipantAnnouncer),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(1).has(BuiltInEndPoints::ParticipantAnnouncer),
            true
        );
        assert_eq!(
            BuiltInEndPointSet::new(16).has(BuiltInEndPoints::ParticipantAnnouncer),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(15).has(BuiltInEndPoints::ParticipantAnnouncer),
            true
        );
    }

    #[test]
    fn test_builtin_endpoint_set_participant_detector() {
        assert_eq!(
            BuiltInEndPointSet::new(0).has(BuiltInEndPoints::ParticipantDetector),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(2).has(BuiltInEndPoints::ParticipantDetector),
            true
        );
        assert_eq!(
            BuiltInEndPointSet::new(16).has(BuiltInEndPoints::ParticipantDetector),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(15).has(BuiltInEndPoints::ParticipantDetector),
            true
        );
    }

    #[test]
    fn test_builtin_endpoint_set_publications_announcer() {
        assert_eq!(
            BuiltInEndPointSet::new(0).has(BuiltInEndPoints::PublicationsAnnouncer),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(4).has(BuiltInEndPoints::PublicationsAnnouncer),
            true
        );
        assert_eq!(
            BuiltInEndPointSet::new(16).has(BuiltInEndPoints::PublicationsAnnouncer),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(15).has(BuiltInEndPoints::PublicationsAnnouncer),
            true
        );
    }

    #[test]
    fn test_builtin_endpoint_set_publications_detector() {
        assert_eq!(
            BuiltInEndPointSet::new(0).has(BuiltInEndPoints::PublicationsDetector),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(8).has(BuiltInEndPoints::PublicationsDetector),
            true
        );
        assert_eq!(
            BuiltInEndPointSet::new(16).has(BuiltInEndPoints::PublicationsDetector),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(15).has(BuiltInEndPoints::PublicationsDetector),
            true
        );
    }

    #[test]
    fn test_builtin_endpoint_set_subscriptions_announcer() {
        assert_eq!(
            BuiltInEndPointSet::new(0).has(BuiltInEndPoints::SubscriptionsAnnouncer),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(16).has(BuiltInEndPoints::SubscriptionsAnnouncer),
            true
        );
        assert_eq!(
            BuiltInEndPointSet::new(32).has(BuiltInEndPoints::SubscriptionsAnnouncer),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(31).has(BuiltInEndPoints::SubscriptionsAnnouncer),
            true
        );
    }

    #[test]
    fn test_builtin_endpoint_set_subscriptions_detector() {
        assert_eq!(
            BuiltInEndPointSet::new(0).has(BuiltInEndPoints::SubscriptionsDetector),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(32).has(BuiltInEndPoints::SubscriptionsDetector),
            true
        );
        assert_eq!(
            BuiltInEndPointSet::new(31).has(BuiltInEndPoints::SubscriptionsDetector),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(63).has(BuiltInEndPoints::SubscriptionsDetector),
            true
        );
    }

    #[test]
    fn test_builtin_endpoint_participant_message_data_writer() {
        assert_eq!(
            BuiltInEndPointSet::new(0).has(BuiltInEndPoints::ParticipantMessageDataWriter),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(1024).has(BuiltInEndPoints::ParticipantMessageDataWriter),
            true
        );
        assert_eq!(
            BuiltInEndPointSet::new(1023).has(BuiltInEndPoints::ParticipantMessageDataWriter),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(2047).has(BuiltInEndPoints::ParticipantMessageDataWriter),
            true
        );
    }

    #[test]
    fn test_builtin_endpoint_participant_message_data_reader() {
        assert_eq!(
            BuiltInEndPointSet::new(0).has(BuiltInEndPoints::ParticipantMessageDataReader),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(2048).has(BuiltInEndPoints::ParticipantMessageDataReader),
            true
        );
        assert_eq!(
            BuiltInEndPointSet::new(2047).has(BuiltInEndPoints::ParticipantMessageDataReader),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(4095).has(BuiltInEndPoints::ParticipantMessageDataReader),
            true
        );
    }

    #[test]
    fn test_builtin_endpoint_topics_announcer() {
        assert_eq!(
            BuiltInEndPointSet::new(0).has(BuiltInEndPoints::TopicsAnnouncer),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(268435456).has(BuiltInEndPoints::TopicsAnnouncer),
            true
        );
        assert_eq!(
            BuiltInEndPointSet::new(268435455).has(BuiltInEndPoints::TopicsAnnouncer),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(536870911).has(BuiltInEndPoints::TopicsAnnouncer),
            true
        );
    }

    #[test]
    fn test_builtin_endpoint_topics_detector() {
        assert_eq!(
            BuiltInEndPointSet::new(0).has(BuiltInEndPoints::TopicsDetector),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(536870912).has(BuiltInEndPoints::TopicsDetector),
            true
        );
        assert_eq!(
            BuiltInEndPointSet::new(536870911).has(BuiltInEndPoints::TopicsDetector),
            false
        );
        assert_eq!(
            BuiltInEndPointSet::new(1073741823).has(BuiltInEndPoints::TopicsDetector),
            true
        );
    }
}

pub mod constants {
    use super::{VendorId, EntityId, EntityKey, EntityKind, Time, Duration};

    pub const VENDOR_ID: VendorId = [99,99];


    pub const ENTITYID_UNKNOWN: EntityId = EntityId {
        entity_key: EntityKey([0, 0, 0x00]),
        entity_kind: EntityKind::UserDefinedUnknown,
    };

    pub const ENTITYID_PARTICIPANT: EntityId = EntityId {
        entity_key: EntityKey([0, 0, 0x01]),
        entity_kind: EntityKind::BuiltInParticipant,
    };

    pub const ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER: EntityId = EntityId {
        entity_key: EntityKey([0, 0, 0x02]),
        entity_kind: EntityKind::BuiltInWriterWithKey,
    };

    pub const ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR: EntityId = EntityId {
        entity_key: EntityKey([0, 0, 0x02]),
        entity_kind: EntityKind::BuiltInReaderWithKey,
    };

    pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER: EntityId = EntityId {
        entity_key: EntityKey([0, 0, 0x03]),
        entity_kind: EntityKind::BuiltInWriterWithKey,
    };

    pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR: EntityId = EntityId {
        entity_key: EntityKey([0, 0, 0x03]),
        entity_kind: EntityKind::BuiltInReaderWithKey,
    };

    pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER: EntityId = EntityId {
        entity_key: EntityKey([0, 0, 0x04]),
        entity_kind: EntityKind::BuiltInWriterWithKey,
    };

    pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR: EntityId = EntityId {
        entity_key: EntityKey([0, 0, 0x04]),
        entity_kind: EntityKind::BuiltInReaderWithKey,
    };

    pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER: EntityId = EntityId {
        entity_key: EntityKey([0, 0x01, 0x00]),
        entity_kind: EntityKind::BuiltInWriterWithKey,
    };

    pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_DETECTOR: EntityId = EntityId {
        entity_key: EntityKey([0, 0x01, 0x00]),
        entity_kind: EntityKind::BuiltInReaderWithKey,
    };

    pub const ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER: EntityId = EntityId {
        entity_key: EntityKey([0, 0x02, 0x00]),
        entity_kind: EntityKind::BuiltInWriterWithKey,
    };

    pub const ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER: EntityId = EntityId {
        entity_key: EntityKey([0, 0x02, 0x00]),
        entity_kind: EntityKind::BuiltInReaderWithKey,
    };

    pub const DURATION_ZERO: Duration = Duration {
        seconds: 0,
        fraction: 0,
    };

    pub const DURATION_INFINITE: Duration = Duration {
        seconds: std::i32::MAX,
        fraction: std::u32::MAX,
    };

    const TIME_ZERO: Time = Time {
        seconds: 0,
        fraction: 0,
    };

    const TIME_INFINITE: Time = Time {
        seconds: std::u32::MAX,
        fraction: std::u32::MAX - 1,
    };

    const TIME_INVALID: Time = Time {
        seconds: std::u32::MAX,
        fraction: std::u32::MAX,
    };
}
