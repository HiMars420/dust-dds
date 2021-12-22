use super::types::{ChangeKind, Guid, InstanceHandle, SequenceNumber};

pub trait RtpsCacheChangeConstructor {
    type DataType: ?Sized;
    type ParameterListType: ?Sized;

    fn new(
        kind: &ChangeKind,
        writer_guid: &Guid,
        instance_handle: &InstanceHandle,
        sequence_number: &SequenceNumber,
        data_value: &Self::DataType,
        inline_qos: &Self::ParameterListType,
    ) -> Self;
}

pub trait RtpsCacheChangeAttributes {
    type DataType: ?Sized;
    type ParameterListType: ?Sized;

    fn kind(&self) -> &ChangeKind;
    fn writer_guid(&self) -> &Guid;
    fn instance_handle(&self) -> &InstanceHandle;
    fn sequence_number(&self) -> &SequenceNumber;
    fn data_value(&self) -> &Self::DataType;
    fn inline_qos(&self) -> &Self::ParameterListType;
}
