
use cdr::{LittleEndian, BigEndian, Infinite};
use std::rc::Rc;

use super::types;
use super::submessages::submessage_elements::Short;

#[derive(Debug, Copy, Clone)]
pub enum CdrEndianness {
    LittleEndian,
    BigEndian,
}

pub trait Pid {
    fn pid() -> types::ParameterId;
}
//  /////////// ParameterList ///////////
pub trait ParameterOps : std::fmt::Debug{
    fn parameter_id(&self) -> super::types::ParameterId;

    fn length(&self) -> Short;

    fn value(&self, endianness: CdrEndianness) -> Vec<u8>;
}

impl<T> ParameterOps for T
    where T: Pid + serde::Serialize + std::fmt::Debug
{
    fn parameter_id(&self) -> super::types::ParameterId {
        T::pid()
    }

    fn length(&self) -> Short {
        // rounded up to multple of 4 (that is besides the length of the value may not be a multiple of 4)
        Short((cdr::size::calc_serialized_data_size(self) + 3 & !3) as i16)
    }

    fn value(&self, endianness: CdrEndianness) -> Vec<u8> {
        match endianness {
            CdrEndianness::LittleEndian => cdr::ser::serialize_data::<_,_,LittleEndian>(&self, Infinite).unwrap(),       
            CdrEndianness::BigEndian => cdr::ser::serialize_data::<_,_,BigEndian>(&self, Infinite).unwrap(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Parameter {
    parameter_id: super::types::ParameterId,
    length: Short, // length is rounded up to multple of 4
    value: Vec<u8>,
}

impl Parameter {
    pub fn new(input: &(impl ParameterOps + ?Sized) , endianness: CdrEndianness) -> Self {
        Self {
            parameter_id: input.parameter_id(),
            length: input.length(),
            value: input.value(endianness),
        }
    }

    pub fn get<'de, T: Pid + serde::Deserialize<'de>>(&self, endianness: CdrEndianness) -> Option<T> {
        if self.parameter_id() == T::pid() {
            Some(match endianness {
                CdrEndianness::LittleEndian => cdr::de::deserialize_data::<T, LittleEndian>(&self.value).ok()?,
                CdrEndianness::BigEndian => cdr::de::deserialize_data::<T, BigEndian>(&self.value).ok()?,
            })
        } else {
            None
        }
    }
}



impl ParameterOps for Parameter {
    fn parameter_id(&self) -> super::types::ParameterId {
        self.parameter_id
    }

    fn length(&self) -> Short {
        self.length
    }

    fn value(&self, _endianness: CdrEndianness) -> Vec<u8> {
        self.value.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ParameterList {
    parameter: Vec<Rc<dyn ParameterOps>>,
}

impl PartialEq for ParameterList{
    fn eq(&self, other: &Self) -> bool {
        self.parameter.iter().zip(other.parameter.iter())
            .find(|(a,b)| 
                (a.parameter_id() != b.parameter_id()) && 
                (a.length() != b.length()) && 
                (a.value(CdrEndianness::LittleEndian) != b.value(CdrEndianness::LittleEndian)))
            .is_none()
    }
}

impl ParameterList {

    const PID_SENTINEL : super::types::ParameterId = 0x0001;

    pub fn new() -> Self {
        Self {parameter: Vec::new()}
    }

    pub fn push<T: ParameterOps + 'static>(&mut self, value: T) {
        self.parameter.push(Rc::new(value));
    }

    pub fn find<'de, T>(&self, endianness: CdrEndianness) -> Option<T>
        where T: Pid + serde::Deserialize<'de>
    {
        let parameter = self.parameter.iter().find(|&x| x.parameter_id() == T::pid())?;
        Some(match endianness {
            CdrEndianness::LittleEndian => cdr::de::deserialize_data::<T, LittleEndian>(&parameter.value(endianness)).ok()?,
            CdrEndianness::BigEndian => cdr::de::deserialize_data::<T, BigEndian>(&parameter.value(endianness)).ok()?,
        })
    }

    pub fn find_all<'de, T>(&self, endianness: CdrEndianness) -> Vec<T>
        where T: Pid + serde::Deserialize<'de>
    {
            self.parameter.iter()
            .filter(|&x| x.parameter_id() == T::pid())
            .map(|parameter| match endianness {
                CdrEndianness::LittleEndian => cdr::de::deserialize_data::<T, LittleEndian>(&parameter.value(endianness)).unwrap(),
                CdrEndianness::BigEndian => cdr::de::deserialize_data::<T, BigEndian>(&parameter.value(endianness)).unwrap(),
            })
            .collect()
    }

    pub fn remove<T>(&mut self) 
        where T: Pid + ParameterOps
    {
        self.parameter.retain(|x| x.parameter_id() != T::pid());
    }

    pub fn len(&self) -> usize {
        self.parameter.len()
    }
}

