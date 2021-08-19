use std::{io::Write, iter::FromIterator, marker::PhantomData};

use byteorder::ByteOrder;
use rust_rtps_pim::{messages::{submessage_elements::{Parameter, ParameterListSubmessageElement}, types::ParameterId}, structure::types::Locator};

use crate::{
    deserialize::{self, Deserialize},
    serialize::{self, Serialize},
};

const PID_SENTINEL: ParameterId = ParameterId(1);
const SENTINEL: Parameter = Parameter {
    parameter_id: PID_SENTINEL,
    length: 0,
    value: &[],
};

impl Serialize for ParameterId {
    fn serialize<W: Write, B: ByteOrder>(&self, mut writer: W) -> serialize::Result {
        self.0.serialize::<_, B>(&mut writer)
    }
}
impl<'de> Deserialize<'de> for ParameterId {
    fn deserialize<B: ByteOrder>(buf: &mut &'de[u8]) -> deserialize::Result<Self> {
        Ok(Self(Deserialize::deserialize::<B>(buf)?))
    }
}

impl<'a> Serialize for Parameter<'_> {
    fn serialize<W: Write, B: ByteOrder>(&self, mut writer: W) -> serialize::Result {
        self.parameter_id.serialize::<_, B>(&mut writer)?;
        self.length.serialize::<_, B>(&mut writer)?;
        self.value.serialize::<_, B>(&mut writer)?;
        let padding: &[u8] = match self.value.len() % 4 {
            1 => &[0; 3],
            2 => &[0; 2],
            3 => &[0; 1],
            _ => &[],
        };
        padding.serialize::<_, B>(&mut writer)
    }
}

impl<'de:'a, 'a> Deserialize<'de> for Parameter<'a> {
    fn deserialize<B: ByteOrder>(buf: &mut &'de[u8]) -> deserialize::Result<Self> {
        let parameter_id = crate::deserialize::Deserialize::deserialize::<B>(buf)?;
        let length = crate::deserialize::Deserialize::deserialize::<B>(buf)?;
        let (value, following) = buf.split_at(length as usize);
        *buf = following;
        Ok(Self {
            parameter_id,
            length,
            value,
        })
    }
}

impl<T> Serialize for ParameterListSubmessageElement<'_, T>
where
    for<'a> &'a T: IntoIterator<Item = &'a Parameter<'a>>,
{
    fn serialize<W: Write, B: ByteOrder>(&self, mut writer: W) -> serialize::Result {
        for parameter in &self.parameter {
            parameter.serialize::<_, B>(&mut writer)?;
        }
        SENTINEL.serialize::<_, B>(&mut writer)
    }
}

impl<'de:'a, 'a, T> Deserialize<'de> for ParameterListSubmessageElement<'a, T>
where
     T: FromIterator<Parameter<'a>>,
{
    fn deserialize<B: ByteOrder>(buf: &mut &'de [u8]) -> deserialize::Result<Self> {
        const MAX_PARAMETERS: usize = 2_usize.pow(16);

        let mut parameter = vec![];

        for _ in 0..MAX_PARAMETERS {
            let parameter_i: Parameter = Deserialize::deserialize::<B>(buf)?;

            if parameter_i == SENTINEL {
                break;
            } else {
                parameter.push(Parameter::from(parameter_i));
            }
        }
        Ok(Self { parameter: T::from_iter(parameter.into_iter()), phantom: PhantomData })
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use super::*;
    use crate::deserialize::from_bytes_le;
    use crate::serialize::to_bytes_le;

    #[test]
    fn serialize_parameter() {
        let parameter = Parameter::new(ParameterId(2), &[5, 6, 7, 8]);
        #[rustfmt::skip]
        assert_eq!(to_bytes_le(&parameter).unwrap(), vec![
            0x02, 0x00, 4, 0, // Parameter | length
            5, 6, 7, 8,       // value
        ]);
    }

    #[test]
    fn serialize_parameter_non_multiple_4() {
        let parameter = Parameter::new(ParameterId(2), &[5, 6, 7]);
        #[rustfmt::skip]
        assert_eq!(to_bytes_le(&parameter).unwrap(), vec![
            0x02, 0x00, 4, 0, // Parameter | length
            5, 6, 7, 0,       // value
        ]);
    }

    #[test]
    fn serialize_parameter_zero_size() {
        let parameter = Parameter::new(ParameterId(2), &[]);
        assert_eq!(
            to_bytes_le(&parameter).unwrap(),
            vec![
            0x02, 0x00, 0, 0, // Parameter | length
        ]
        );
    }

    #[test]
    fn deserialize_parameter_non_multiple_of_4() {
        let expected = Parameter::new(ParameterId(2), &[5, 6, 7, 8, 9, 10, 11, 0]);
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x02, 0x00, 8, 0, // Parameter | length
            5, 6, 7, 8,       // value
            9, 10, 11, 0,     // value
        ]).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter() {
        let expected = Parameter::new(ParameterId(2), &[5, 6, 7, 8, 9, 10, 11, 12]);
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x02, 0x00, 8, 0, // Parameter | length
            5, 6, 7, 8,       // value
            9, 10, 11, 12,       // value
        ]).unwrap();
        assert_eq!(expected, result);
    }


    #[test]
    fn deserialize_parameter_list() {
        let expected = ParameterListSubmessageElement {
            parameter: vec![
                Parameter::new(ParameterId(0x02), &[15, 16, 17, 18]),
                Parameter::new(ParameterId(0x03), &[25, 26, 27, 28]),
            ],
            phantom: PhantomData
        };
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x02, 0x00, 4, 0, // Parameter ID | length
            15, 16, 17, 18,        // value
            0x03, 0x00, 4, 0, // Parameter ID | length
            25, 26, 27, 28,        // value
            0x01, 0x00, 0, 0, // Sentinel: Parameter ID | length
        ]).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter_list_with_long_parameter_including_sentinel() {
        #[rustfmt::skip]
        let parameter_value_expected = &[
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
        ];

        let expected = ParameterListSubmessageElement {
            parameter: vec![Parameter::new(ParameterId(0x32), parameter_value_expected)],
            phantom: PhantomData
        };
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x32, 0x00, 24, 0x00, // Parameter ID | length
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
        ]).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_parameter_list_with_multiple_parameters_with_same_id() {
        #[rustfmt::skip]
        let parameter_value_expected1 = &[
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01,
        ];
        let parameter_value_expected2 = &[
            0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
            0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
        ];

        let expected = ParameterListSubmessageElement {
            parameter: vec![
                Parameter::new(ParameterId(0x32), parameter_value_expected1),
                Parameter::new(ParameterId(0x32), parameter_value_expected2),
            ],
            phantom: PhantomData
        };
        #[rustfmt::skip]
        let result = from_bytes_le(&[
            0x32, 0x00, 24, 0x00, // Parameter ID | length
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x01, 0x01, 0x01, 0x01, // Parameter value
            0x32, 0x00, 24, 0x00, // Parameter ID | length
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x01, 0x00, 0x00, 0x00, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x02, 0x02, 0x02, 0x02, // Parameter value
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, Length: 0
        ]).unwrap();
        assert_eq!(expected, result);
    }
}
