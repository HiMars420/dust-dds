use std::convert::TryFrom;

use rtps_pim::{
    messages::{
        submessage_elements::{
            EntityIdSubmessageElement, Parameter, ParameterListSubmessageElement,
            SequenceNumberSubmessageElement, SerializedDataSubmessageElement,
        },
        submessages::DataSubmessage,
        types::ParameterId,
    },
    structure::types::{
        ChangeKind, Guid, GuidPrefix, InstanceHandle, SequenceNumber, ENTITYID_UNKNOWN,
    },
};

use crate::{
    implementation::data_representation_inline_qos::{
        parameter_id_values::PID_STATUS_INFO,
        types::{STATUS_INFO_DISPOSED_FLAG, STATUS_INFO_UNREGISTERED_FLAG},
    },
    return_type::DdsError,
};

#[derive(Debug, PartialEq)]
pub struct RtpsParameter {
    parameter_id: ParameterId,
    value: Vec<u8>,
}

impl RtpsParameter {
    pub fn new(parameter_id: ParameterId, value: Vec<u8>) -> Self {
        Self {
            parameter_id,
            value,
        }
    }

    pub fn parameter_id(&self) -> ParameterId {
        self.parameter_id
    }

    pub fn value(&self) -> &[u8] {
        self.value.as_ref()
    }
}

pub struct RtpsCacheChangeImpl {
    kind: ChangeKind,
    writer_guid: Guid,
    sequence_number: SequenceNumber,
    instance_handle: InstanceHandle,
    data: Vec<u8>,
    inline_qos: Vec<RtpsParameter>,
}

impl PartialEq for RtpsCacheChangeImpl {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.writer_guid == other.writer_guid
            && self.sequence_number == other.sequence_number
            && self.instance_handle == other.instance_handle
    }
}

impl TryFrom<(GuidPrefix, &DataSubmessage<Vec<Parameter<'_>>, &[u8]>)> for RtpsCacheChangeImpl {
    type Error = DdsError;

    fn try_from(
        value: (GuidPrefix, &DataSubmessage<Vec<Parameter<'_>>, &[u8]>),
    ) -> Result<Self, Self::Error> {
        let (source_guid_prefix, data) = value;
        let writer_guid = Guid::new(source_guid_prefix, data.writer_id.value);

        let instance_handle = [0; 16];
        let sequence_number = data.writer_sn.value;
        let data_value = data.serialized_payload.value.to_vec();

        let inline_qos: Vec<RtpsParameter> = data
            .inline_qos
            .parameter
            .iter()
            .map(|p| RtpsParameter {
                parameter_id: p.parameter_id,
                value: p.value.to_vec(),
            })
            .collect();

        let kind = match (data.data_flag, data.key_flag) {
            (true, false) => Ok(ChangeKind::Alive),
            (false, true) => {
                if let Some(p) = inline_qos
                    .iter()
                    .find(|&x| x.parameter_id == ParameterId(PID_STATUS_INFO))
                {
                    let mut deserializer =
                        cdr::Deserializer::<_, _, cdr::LittleEndian>::new(p.value(), cdr::Infinite);
                    let status_info = serde::Deserialize::deserialize(&mut deserializer).unwrap();
                    match status_info {
                        STATUS_INFO_DISPOSED_FLAG => Ok(ChangeKind::NotAliveDisposed),
                        STATUS_INFO_UNREGISTERED_FLAG => Ok(ChangeKind::NotAliveUnregistered),
                        _ => Err(DdsError::PreconditionNotMet(
                            "Unknown status info value".to_string(),
                        )),
                    }
                } else {
                    Err(DdsError::PreconditionNotMet(
                        "Missing mandatory StatusInfo parameter".to_string(),
                    ))
                }
            }
            _ => Err(DdsError::PreconditionNotMet(
                "Invalid data submessage data and key flag combination".to_string(),
            )),
        }?;

        Ok(RtpsCacheChangeImpl {
            kind,
            writer_guid,
            instance_handle,
            sequence_number,
            data: data_value,
            inline_qos,
        })
    }
}

impl<'a> From<&'a RtpsCacheChangeImpl> for DataSubmessage<Vec<Parameter<'a>>, &'a [u8]> {
    fn from(val: &'a RtpsCacheChangeImpl) -> Self {
        let endianness_flag = true;
        let inline_qos_flag = true;
        let (data_flag, key_flag) = match val.kind() {
            ChangeKind::Alive => (true, false),
            ChangeKind::NotAliveDisposed | ChangeKind::NotAliveUnregistered => (false, true),
            _ => todo!(),
        };
        let non_standard_payload_flag = false;
        let reader_id = EntityIdSubmessageElement {
            value: ENTITYID_UNKNOWN,
        };
        let writer_id = EntityIdSubmessageElement {
            value: val.writer_guid().entity_id(),
        };
        let writer_sn = SequenceNumberSubmessageElement {
            value: val.sequence_number(),
        };
        let inline_qos = ParameterListSubmessageElement {
            parameter: val
                .inline_qos()
                .iter()
                .map(|p| Parameter {
                    parameter_id: p.parameter_id,
                    length: p.value.len() as i16,
                    value: p.value.as_ref(),
                })
                .collect(),
        };
        let serialized_payload = SerializedDataSubmessageElement {
            value: val.data_value(),
        };
        DataSubmessage {
            endianness_flag,
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        }
    }
}

impl RtpsCacheChangeImpl {
    pub fn new(
        kind: ChangeKind,
        writer_guid: Guid,
        instance_handle: InstanceHandle,
        sequence_number: SequenceNumber,
        data_value: Vec<u8>,
        inline_qos: Vec<RtpsParameter>,
    ) -> Self {
        Self {
            kind,
            writer_guid,
            sequence_number,
            instance_handle,
            data: data_value,
            inline_qos,
        }
    }
}

impl RtpsCacheChangeImpl {
    pub fn kind(&self) -> ChangeKind {
        self.kind
    }

    pub fn writer_guid(&self) -> Guid {
        self.writer_guid
    }

    pub fn instance_handle(&self) -> InstanceHandle {
        self.instance_handle
    }

    pub fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub fn data_value(&self) -> &[u8] {
        self.data.as_ref()
    }

    pub fn inline_qos(&self) -> &[RtpsParameter] {
        &self.inline_qos
    }
}

#[derive(Default)]
pub struct RtpsHistoryCacheImpl {
    changes: Vec<RtpsCacheChangeImpl>,
}

impl RtpsHistoryCacheImpl {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }
}

impl RtpsHistoryCacheImpl {
    pub fn changes(&self) -> &[RtpsCacheChangeImpl] {
        &self.changes
    }
}

impl RtpsHistoryCacheImpl {
    pub fn add_change(&mut self, change: RtpsCacheChangeImpl) {
        self.changes.push(change);
    }

    pub fn remove_change<F>(&mut self, mut f: F)
    where
        F: FnMut(&RtpsCacheChangeImpl) -> bool,
    {
        self.changes.retain(|cc| !f(cc));
    }

    pub fn get_seq_num_min(&self) -> Option<SequenceNumber> {
        self.changes.iter().map(|cc| cc.sequence_number).min()
    }

    pub fn get_seq_num_max(&self) -> Option<SequenceNumber> {
        self.changes.iter().map(|cc| cc.sequence_number).max()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rtps_pim::structure::types::GUID_UNKNOWN;

    #[test]
    fn remove_change() {
        let mut hc = RtpsHistoryCacheImpl::new();
        let change = RtpsCacheChangeImpl::new(
            rtps_pim::structure::types::ChangeKind::Alive,
            GUID_UNKNOWN,
            [0; 16],
            1,
            vec![],
            vec![],
        );
        hc.add_change(change);
        hc.remove_change(|cc| cc.sequence_number() == 1);
        assert!(hc.changes().is_empty());
    }

    #[test]
    fn get_seq_num_min() {
        let mut hc = RtpsHistoryCacheImpl::new();
        let change1 = RtpsCacheChangeImpl::new(
            rtps_pim::structure::types::ChangeKind::Alive,
            GUID_UNKNOWN,
            [0; 16],
            1,
            vec![],
            vec![],
        );
        let change2 = RtpsCacheChangeImpl::new(
            rtps_pim::structure::types::ChangeKind::Alive,
            GUID_UNKNOWN,
            [0; 16],
            2,
            vec![],
            vec![],
        );
        hc.add_change(change1);
        hc.add_change(change2);
        assert_eq!(hc.get_seq_num_min(), Some(1));
    }

    #[test]
    fn get_seq_num_max() {
        let mut hc = RtpsHistoryCacheImpl::new();
        let change1 = RtpsCacheChangeImpl::new(
            rtps_pim::structure::types::ChangeKind::Alive,
            GUID_UNKNOWN,
            [0; 16],
            1,
            vec![],
            vec![],
        );
        let change2 = RtpsCacheChangeImpl::new(
            rtps_pim::structure::types::ChangeKind::Alive,
            GUID_UNKNOWN,
            [0; 16],
            2,
            vec![],
            vec![],
        );
        hc.add_change(change1);
        hc.add_change(change2);
        assert_eq!(hc.get_seq_num_max(), Some(2));
    }
}
