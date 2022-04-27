use rtps_pim::{
    behavior::stateful_reader_behavior::FromDataSubmessageAndGuidPrefix,
    messages::{
        submessage_elements::{
            EntityIdSubmessageElement, Parameter, ParameterListSubmessageElement,
            SequenceNumberSubmessageElement, SerializedDataSubmessageElement,
        },
        submessages::DataSubmessage,
        types::ParameterId,
    },
    structure::{
        cache_change::{RtpsCacheChangeAttributes, RtpsCacheChangeConstructor},
        history_cache::{
            RtpsHistoryCacheAttributes, RtpsHistoryCacheConstructor, RtpsHistoryCacheOperations,
        },
        types::{ChangeKind, Guid, GuidPrefix, InstanceHandle, SequenceNumber, ENTITYID_UNKNOWN},
    },
};

#[derive(Debug, PartialEq)]
pub struct RtpsParameter {
    parameter_id: ParameterId,
    value: Vec<u8>,
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

impl FromDataSubmessageAndGuidPrefix<Vec<Parameter<'_>>, &[u8]> for RtpsCacheChangeImpl {
    fn from(
        source_guid_prefix: GuidPrefix,
        data: &DataSubmessage<Vec<Parameter<'_>>, &[u8]>,
    ) -> Self {
        let writer_guid = Guid::new(source_guid_prefix, data.writer_id.value);
        let kind = match (data.data_flag, data.key_flag) {
            (true, false) => ChangeKind::Alive,
            (false, true) => ChangeKind::NotAliveDisposed,
            _ => todo!(),
        };
        let instance_handle = 0;
        let sequence_number = data.writer_sn.value;
        let data_value = data.serialized_payload.value.to_vec();

        let inline_qos = data
            .inline_qos
            .parameter
            .iter()
            .map(|p| RtpsParameter {
                parameter_id: p.parameter_id,
                value: p.value.to_vec(),
            })
            .collect();
        RtpsCacheChangeImpl {
            kind,
            writer_guid,
            instance_handle,
            sequence_number,
            data: data_value,
            inline_qos,
        }
    }
}

impl<'a> Into<DataSubmessage<Vec<Parameter<'a>>, &'a [u8]>> for &'a RtpsCacheChangeImpl {
    fn into(self) -> DataSubmessage<Vec<Parameter<'a>>, &'a [u8]> {
        let endianness_flag = true;
        let inline_qos_flag = true;
        let (data_flag, key_flag) = match self.kind() {
            ChangeKind::Alive => (true, false),
            ChangeKind::NotAliveDisposed | ChangeKind::NotAliveUnregistered => (false, true),
            _ => todo!(),
        };
        let non_standard_payload_flag = false;
        let reader_id = EntityIdSubmessageElement {
            value: ENTITYID_UNKNOWN,
        };
        let writer_id = EntityIdSubmessageElement {
            value: self.writer_guid().entity_id(),
        };
        let writer_sn = SequenceNumberSubmessageElement {
            value: self.sequence_number(),
        };
        let inline_qos = ParameterListSubmessageElement {
            parameter: self
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
            value: self.data_value().as_ref(),
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

impl RtpsCacheChangeConstructor for RtpsCacheChangeImpl {
    type DataType = Vec<u8>;
    type ParameterListType = Vec<RtpsParameter>;

    fn new(
        kind: ChangeKind,
        writer_guid: Guid,
        instance_handle: InstanceHandle,
        sequence_number: SequenceNumber,
        data_value: Self::DataType,
        inline_qos: Self::ParameterListType,
    ) -> Self {
        Self {
            kind,
            writer_guid,
            sequence_number,
            instance_handle,
            data: data_value,
            inline_qos: inline_qos.into(),
        }
    }
}

impl RtpsCacheChangeAttributes for &RtpsCacheChangeImpl {
    type DataType = [u8];
    type ParameterListType = Vec<RtpsParameter>;

    fn kind(&self) -> ChangeKind {
        self.kind
    }

    fn writer_guid(&self) -> Guid {
        todo!()
    }

    fn instance_handle(&self) -> InstanceHandle {
        todo!()
    }

    fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    fn data_value(&self) -> &Self::DataType {
        todo!()
    }

    fn inline_qos(&self) -> &Self::ParameterListType {
        todo!()
    }
}

impl RtpsCacheChangeAttributes for RtpsCacheChangeImpl {
    type DataType = [u8];
    type ParameterListType = [RtpsParameter];

    fn kind(&self) -> ChangeKind {
        self.kind
    }

    fn writer_guid(&self) -> Guid {
        self.writer_guid
    }

    fn instance_handle(&self) -> InstanceHandle {
        self.instance_handle
    }

    fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    fn data_value(&self) -> &Self::DataType {
        self.data.as_ref()
    }

    fn inline_qos(&self) -> &Self::ParameterListType {
        &self.inline_qos
    }
}

pub struct RtpsHistoryCacheImpl {
    changes: Vec<RtpsCacheChangeImpl>,
}

impl RtpsHistoryCacheConstructor for RtpsHistoryCacheImpl {
    fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }
}

impl RtpsHistoryCacheAttributes for RtpsHistoryCacheImpl {
    type CacheChangeType = RtpsCacheChangeImpl;

    fn changes(&self) -> &[Self::CacheChangeType] {
        &self.changes
    }
}

impl RtpsHistoryCacheOperations for RtpsHistoryCacheImpl {
    type CacheChangeType = RtpsCacheChangeImpl;

    fn add_change(&mut self, change: Self::CacheChangeType) {
        self.changes.push(change);
    }

    fn remove_change<F>(&mut self, mut f: F)
    where
        F: FnMut(&Self::CacheChangeType) -> bool,
    {
        self.changes.retain(|cc| !f(cc));
    }

    fn get_seq_num_min(&self) -> Option<SequenceNumber> {
        self.changes
            .iter()
            .map(|cc| cc.sequence_number)
            .min()
            .clone()
    }

    fn get_seq_num_max(&self) -> Option<SequenceNumber> {
        self.changes
            .iter()
            .map(|cc| cc.sequence_number)
            .max()
            .clone()
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
            0,
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
            0,
            1,
            vec![],
            vec![],
        );
        let change2 = RtpsCacheChangeImpl::new(
            rtps_pim::structure::types::ChangeKind::Alive,
            GUID_UNKNOWN,
            0,
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
            0,
            1,
            vec![],
            vec![],
        );
        let change2 = RtpsCacheChangeImpl::new(
            rtps_pim::structure::types::ChangeKind::Alive,
            GUID_UNKNOWN,
            0,
            2,
            vec![],
            vec![],
        );
        hc.add_change(change1);
        hc.add_change(change2);
        assert_eq!(hc.get_seq_num_max(), Some(2));
    }
}
