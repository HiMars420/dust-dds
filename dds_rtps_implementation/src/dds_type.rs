use rust_rtps_pim::structure::types::{DataType, InstanceHandleType};

pub trait DDSType<PSM: InstanceHandleType + DataType> {
    fn type_name() -> &'static str;

    fn has_key() -> bool;

    fn key(&self) -> PSM::InstanceHandle;

    fn serialize(&self) -> PSM::Data;

    fn deserialize(data: Vec<u8>) -> Self;
}
