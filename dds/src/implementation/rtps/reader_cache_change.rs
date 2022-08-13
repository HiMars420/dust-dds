use crate::dcps_psm::{InstanceHandle, Time, ViewStateKind};

use super::{
    history_cache::RtpsParameter,
    types::{ChangeKind, Guid, SequenceNumber},
};

#[derive(Debug, Clone)]

pub struct RtpsReaderCacheChange {
    kind: ChangeKind,
    writer_guid: Guid,
    sequence_number: SequenceNumber,
    instance_handle: InstanceHandle,
    data: Vec<u8>,
    inline_qos: Vec<RtpsParameter>,
    source_timestamp: Option<Time>,
    view_state: ViewStateKind,
}

impl PartialEq for RtpsReaderCacheChange {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.writer_guid == other.writer_guid
            && self.sequence_number == other.sequence_number
            && self.instance_handle == other.instance_handle
    }
}

impl RtpsReaderCacheChange {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: ChangeKind,
        writer_guid: Guid,
        instance_handle: InstanceHandle,
        sequence_number: SequenceNumber,
        data_value: Vec<u8>,
        inline_qos: Vec<RtpsParameter>,
        source_timestamp: Option<Time>,
        view_state: ViewStateKind,
    ) -> Self {
        Self {
            kind,
            writer_guid,
            sequence_number,
            instance_handle,
            data: data_value,
            inline_qos,
            source_timestamp,
            view_state,
        }
    }

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

    pub fn source_timestamp(&self) -> &Option<Time> {
        &self.source_timestamp
    }

    pub fn view_state(&self) -> ViewStateKind {
        self.view_state
    }
}
