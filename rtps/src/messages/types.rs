use core::convert::{TryFrom, TryInto};

use super::submessages::{submessage_elements::Parameter, Serialize};

/// This files shall only contain the types as listed in the DDSI-RTPS Version 2.3
/// Table 8.13 - Types used to define RTPS messages
///

pub mod constants {
    use super::ProtocolId;
    use super::Time;

    pub const TIME_ZERO: Time = Time {
        seconds: 0,
        fraction: 0,
    };
    pub const TIME_INFINITE: Time = Time {
        seconds: core::u32::MAX,
        fraction: core::u32::MAX - 1,
    };
    pub const TIME_INVALID: Time = Time {
        seconds: core::u32::MAX,
        fraction: core::u32::MAX,
    };
    pub const PROTOCOL_RTPS: ProtocolId = [b'R', b'T', b'P', b'S'];
}

pub type ProtocolId = [u8; 4];

pub type SubmessageFlag = bool;

impl Serialize for [SubmessageFlag; 8] {
    fn serialize(
        &self,
        buf: &mut [u8],
        _protocol_version: crate::types::ProtocolVersion,
    ) -> Result<usize, ()> {
        let mut flags = 0u8;
        for i in 0..8 {
            if self[i] {
                flags |= 0b00000001 << i;
            }
        }
        buf[0] = flags;
        Ok(1)
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum SubmessageKind {
    Pad,
    AckNack,
    Heartbeat,
    Gap,
    InfoTimestamp,
    InfoSource,
    InfoReplyIP4,
    InfoDestination,
    InfoReply,
    NackFrag,
    HeartbeatFrag,
    Data,
    DataFrag,
}

impl Serialize for SubmessageKind {
    fn serialize(
        &self,
        buf: &mut [u8],
        _protocol_version: crate::types::ProtocolVersion,
    ) -> Result<usize, ()> {
        let value = match self {
            SubmessageKind::Pad => 0x01,
            SubmessageKind::AckNack => 0x06,
            SubmessageKind::Heartbeat => 0x07,
            SubmessageKind::Gap => 0x08,
            SubmessageKind::InfoTimestamp => 0x09,
            SubmessageKind::InfoSource => 0x0c,
            SubmessageKind::InfoReplyIP4 => 0x0d,
            SubmessageKind::InfoDestination => 0x0e,
            SubmessageKind::InfoReply => 0x0f,
            SubmessageKind::NackFrag => 0x12,
            SubmessageKind::HeartbeatFrag => 0x13,
            SubmessageKind::Data => 0x15,
            SubmessageKind::DataFrag => 0x16,
        };

        buf[0] = value;

        Ok(1)
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Time {
    seconds: u32,
    fraction: u32,
}
impl Time {
    pub fn new(seconds: u32, fraction: u32) -> Self {
        Self { seconds, fraction }
    }
    pub fn seconds(&self) -> u32 {
        self.seconds
    }
    pub fn fraction(&self) -> u32 {
        self.fraction
    }
}

pub type Count = i32;

pub type FragmentNumber = u32;

pub type ParameterId = i16;

// /////////// KeyHash and StatusInfo /////////////

pub const PID_KEY_HASH: ParameterId = 0x0070;
pub const PID_STATUS_INFO: ParameterId = 0x0071;
#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub struct KeyHash(pub [u8; 16]);

impl From<KeyHash> for Parameter {
    fn from(input: KeyHash) -> Self {
        Parameter::new(PID_KEY_HASH, input.0.into())
    }
}

impl TryFrom<Parameter> for KeyHash {
    type Error = ();
    fn try_from(parameter: Parameter) -> Result<Self, Self::Error> {
        if parameter.parameter_id() == PID_KEY_HASH {
            Ok(KeyHash(
                parameter
                    .value()
                    .get(0..16)
                    .ok_or(())?
                    .try_into()
                    .map_err(|_| ())?,
            ))
        } else {
            Err(())
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub struct StatusInfo(pub [u8; 4]);

impl StatusInfo {
    pub const DISPOSED_FLAG_MASK: u8 = 0b0000_0001;
    pub const UNREGISTERED_FLAG_MASK: u8 = 0b0000_0010;
    pub const FILTERED_FLAG_MASK: u8 = 0b0000_0100;

    pub fn disposed_flag(&self) -> bool {
        self.0[3] & StatusInfo::DISPOSED_FLAG_MASK == StatusInfo::DISPOSED_FLAG_MASK
    }

    pub fn unregistered_flag(&self) -> bool {
        self.0[3] & StatusInfo::UNREGISTERED_FLAG_MASK == StatusInfo::UNREGISTERED_FLAG_MASK
    }

    pub fn filtered_flag(&self) -> bool {
        self.0[3] & StatusInfo::FILTERED_FLAG_MASK == StatusInfo::FILTERED_FLAG_MASK
    }
}

impl From<StatusInfo> for Parameter {
    fn from(input: StatusInfo) -> Self {
        Parameter::new(PID_STATUS_INFO, input.0.into())
    }
}

impl TryFrom<Parameter> for StatusInfo {
    type Error = ();
    fn try_from(parameter: Parameter) -> Result<Self, Self::Error> {
        if parameter.parameter_id() == PID_STATUS_INFO {
            Ok(StatusInfo(
                parameter
                    .value()
                    .get(0..4)
                    .ok_or(())?
                    .try_into()
                    .map_err(|_| ())?,
            ))
        } else {
            Err(())
        }
    }
}
