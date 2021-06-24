use crate::messages::submessage_elements::ParameterListSubmessageElementPIM;

use super::types::{ChangeKind, DataPIM, InstanceHandlePIM, SequenceNumber, GUIDPIM};

pub trait RTPSCacheChange<PSM>
where
    PSM: InstanceHandlePIM
        + DataPIM
        + GUIDPIM
        + ParameterListSubmessageElementPIM,
{
    fn kind(&self) -> ChangeKind;
    fn writer_guid(&self) -> &PSM::GUIDType;
    fn instance_handle(&self) -> &PSM::InstanceHandleType;
    fn sequence_number(&self) -> &SequenceNumber;
    fn data_value(&self) -> &PSM::DataType;
    fn inline_qos(&self) -> &PSM::ParameterListSubmessageElementType;
}
