use std::{
    io::{BufRead, Read},
    marker::PhantomData,
};

use byteorder::{ByteOrder, ReadBytesExt};
use rust_dds_api::return_type::{DDSError, DDSResult};
use serde::Serialize;

use crate::dds_type::{BigEndian, Endianness, LittleEndian};

const PID_SENTINEL: u16 = 1;

pub struct ParameterSerializer<W, E>
where
    W: std::io::Write,
    E: Endianness,
{
    serializer: cdr::Serializer<W, E::Endianness>,
    phantom: PhantomData<E>,
}

impl<W, E> ParameterSerializer<W, E>
where
    W: std::io::Write,
    E: Endianness,
{
    pub fn new(writer: W) -> Self {
        Self {
            serializer: cdr::Serializer::<_, E::Endianness>::new(writer),
            phantom: PhantomData,
        }
    }

    pub fn serialize_payload_header(&mut self) -> DDSResult<()> {
        E::REPRESENTATION_IDENTIFIER
            .serialize(&mut self.serializer)
            .map_err(|err| DDSError::PreconditionNotMet(err.to_string()))?;
        E::REPRESENTATION_OPTIONS
            .serialize(&mut self.serializer)
            .map_err(|err| DDSError::PreconditionNotMet(err.to_string()))?;
        Ok(())
    }

    pub fn serialize_parameter<T>(&mut self, parameter_id: u16, value: &T) -> DDSResult<()>
    where
        T: serde::Serialize,
    {
        let length_without_padding = cdr::size::calc_serialized_data_size(&value) as i16;
        let padding_length = (4 - length_without_padding) & 3;
        let length = length_without_padding + padding_length;

        parameter_id
            .serialize(&mut self.serializer)
            .map_err(|err| DDSError::PreconditionNotMet(err.to_string()))?;

        length
            .serialize(&mut self.serializer)
            .map_err(|err| DDSError::PreconditionNotMet(err.to_string()))?;

        value
            .serialize(&mut self.serializer)
            .map_err(|err| DDSError::PreconditionNotMet(err.to_string()))?;

        for _ in 0..padding_length {
            0_u8.serialize(&mut self.serializer)
                .map_err(|err| DDSError::PreconditionNotMet(err.to_string()))?;
        }
        Ok(())
    }

    pub fn serialize_sentinel(&mut self) -> DDSResult<()> {
        PID_SENTINEL.serialize(&mut self.serializer).map_err(|err| DDSError::PreconditionNotMet(err.to_string()))?;
        [0_u8, 0].serialize(&mut self.serializer).map_err(|err| DDSError::PreconditionNotMet(err.to_string()))?;
        Ok(())
    }
}


#[derive(Debug, PartialEq)]
pub struct Parameter<'a> {
    parameter_id: u16,
    value: &'a [u8],
}

impl<'de: 'a, 'a> Parameter<'a> {
    fn read<B: ByteOrder>(buf: &mut &'de [u8]) -> Result<Self, std::io::Error> {
        let parameter_id = buf.read_u16::<B>()?;
        let length = buf.read_i16::<B>()?;
        let (value, following) = buf.split_at(length as usize);
        *buf = following;
        Ok(Self {
            parameter_id,
            value,
        })
    }
}

#[derive(Debug, PartialEq)]
enum RepresentationIdentifier {
    PlCdrBe,
    PlCdrLe,
}

impl RepresentationIdentifier {
    pub fn read(buf: &mut &[u8]) -> Result<Self, std::io::Error> {
        let mut representation_identifier = [0; 2];
        buf.read(&mut representation_identifier)?;
        match representation_identifier {
            BigEndian::REPRESENTATION_IDENTIFIER => Ok(RepresentationIdentifier::PlCdrBe),
            LittleEndian::REPRESENTATION_IDENTIFIER => Ok(RepresentationIdentifier::PlCdrLe),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid representation identifier",
            )),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ParameterList<'a> {
    parameter: Vec<Parameter<'a>>,
    representation_identifier: RepresentationIdentifier,
}

impl<'de: 'a, 'a> ParameterList<'a> {
    pub fn read(buf: &mut &'de [u8]) -> Result<Self, std::io::Error> {
        let representation_identifier = RepresentationIdentifier::read(buf)?;
        // ignore representation_options
        buf.consume(2);

        let mut parameter = vec![];
        loop {
            let parameter_i = match representation_identifier {
                RepresentationIdentifier::PlCdrBe => Parameter::read::<byteorder::BigEndian>(buf)?,
                RepresentationIdentifier::PlCdrLe => {
                    Parameter::read::<byteorder::LittleEndian>(buf)?
                }
            };
            if parameter_i.parameter_id == PID_SENTINEL {
                break;
            } else {
                parameter.push(parameter_i);
            }
        }
        Ok(Self {
            parameter,
            representation_identifier,
        })
    }
}

impl<'de> ParameterList<'de> {
    pub fn get<T: serde::Deserialize<'de>>(&self, parameter_id: u16) -> Result<T, std::io::Error> {
        for parameter in self.parameter.iter() {
            if parameter.parameter_id == parameter_id {
                return Ok(self.deserialize_parameter(parameter)?);
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Parameter with id {} not found", parameter_id),
        ))
    }

    pub fn get_list<T: serde::Deserialize<'de>>(
        &self,
        parameter_id: u16,
    ) -> Result<Vec<T>, std::io::Error> {
        let mut result = vec![];
        for parameter in self.parameter.iter() {
            if parameter.parameter_id == parameter_id {
                if let Ok(result_i) = self.deserialize_parameter(parameter) {
                    result.push(result_i);
                }
            }
        }
        Ok(result)
    }

    fn deserialize_parameter<T: serde::Deserialize<'de>>(
        &self,
        parameter: &Parameter,
    ) -> Result<T, std::io::Error> {
        Ok(match self.representation_identifier {
            RepresentationIdentifier::PlCdrBe => {
                let mut deserializer = cdr::Deserializer::<_, _, byteorder::BigEndian>::new(
                    parameter.value,
                    cdr::Infinite,
                );
                serde::Deserialize::deserialize(&mut deserializer).map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())
                })?
            }
            RepresentationIdentifier::PlCdrLe => {
                let mut deserializer = cdr::Deserializer::<_, _, byteorder::LittleEndian>::new(
                    parameter.value,
                    cdr::Infinite,
                );
                serde::Deserialize::deserialize(&mut deserializer).map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())
                })?
            }
        })
    }
}
