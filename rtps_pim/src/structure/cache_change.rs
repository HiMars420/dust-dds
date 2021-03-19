use crate::{
    messages::submessages::submessage_elements::ParameterList,
    types::{ChangeKind, SequenceNumber, GUID},
};

pub trait RTPSCacheChange {
    type Data;
    type InstanceHandle;
    type ParameterList: ParameterList;

    fn new(
        kind: ChangeKind,
        writer_guid: GUID,
        instance_handle: Self::InstanceHandle,
        sequence_number: SequenceNumber,
        data_value: Self::Data,
        inline_qos: Self::ParameterList,
    ) -> Self;
    fn kind(&self) -> ChangeKind;
    fn writer_guid(&self) -> GUID;
    fn instance_handle(&self) -> &Self::InstanceHandle;
    fn sequence_number(&self) -> SequenceNumber;
    fn data_value(&self) -> &Self::Data;
    fn inline_qos(&self) -> &Self::ParameterList;
}
