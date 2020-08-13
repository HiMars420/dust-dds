use std::cmp::Ordering;

use crate::types::{ChangeKind, InstanceHandle, SequenceNumber, GUID, };
use crate::messages::ParameterList;

#[derive(Debug, Clone)]
pub struct CacheChange {
    kind: ChangeKind,
    writer_guid: GUID,
    instance_handle: InstanceHandle,
    sequence_number: SequenceNumber,
    data_value: Option<Vec<u8>>,
    inline_qos: Option<ParameterList>,
}

impl CacheChange {
    pub fn new(
        kind: ChangeKind,
        writer_guid: GUID,
        instance_handle: InstanceHandle,
        sequence_number: SequenceNumber,
        data_value: Option<Vec<u8>>,
        inline_qos: Option<ParameterList>,
    ) -> CacheChange {
        CacheChange {
            kind,
            writer_guid,
            instance_handle,
            sequence_number,
            inline_qos,
            data_value,
        }
    }

    pub fn change_kind(&self) -> &ChangeKind {
        &self.kind
    }

    pub fn writer_guid(&self) -> &GUID {
        &self.writer_guid
    }

    pub fn instance_handle(&self) -> &InstanceHandle {
        &self.instance_handle
    }

    pub fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub fn inline_qos(&self) -> &Option<ParameterList> {
        &self.inline_qos
    }

    pub fn data_value(&self) -> Option<&Vec<u8>> {
        match &self.data_value {
            Some(data_value) => Some(data_value),
            None => None,
        }
    }

    // pub fn clone_without_data(&self) -> Self {
    //     match *self {
    //         CacheChange {
    //             kind: ref __self_0_0,
    //             writer_guid: ref __self_0_1,
    //             instance_handle: ref __self_0_2,
    //             sequence_number: ref __self_0_3,
    //             data_value: ref __self_0_5,
    //             inline_qos: ref __self_0_4,
    //         } => {
    //     CacheChange {
    //        kind: *__self_0_0,
    //         writer_guid: * __self_0_1,
    //         instance_handle: * __self_0_2,
    //         sequence_number: * __self_0_3,
    //         data_value: None,
    //         inline_qos: None,
    //     }}}
    // }
}


impl PartialEq for CacheChange {
    fn eq(&self, other: &Self) -> bool {
        self.writer_guid == other.writer_guid
            && self.instance_handle == other.instance_handle
            && self.sequence_number == other.sequence_number
    }
}

impl Eq for CacheChange {}

impl Ord for CacheChange {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sequence_number.cmp(&other.sequence_number)
    }
}

impl PartialOrd for CacheChange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.sequence_number.cmp(&other.sequence_number))
    }
}

impl ::core::hash::Hash for CacheChange {
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        match *self {
            CacheChange {
                kind: ref __self_0_0,
                writer_guid: ref __self_0_1,
                instance_handle: ref __self_0_2,
                sequence_number: ref __self_0_3,
                data_value: ref __self_0_5,
                inline_qos: ref __self_0_4,
            } => {
                ::core::hash::Hash::hash(&(*__self_0_0), state);
                ::core::hash::Hash::hash(&(*__self_0_1), state);
                ::core::hash::Hash::hash(&(*__self_0_2), state);
                ::core::hash::Hash::hash(&(*__self_0_3), state);
                // ::core::hash::Hash::hash(&(*__self_0_4), state)
                // Explicitly ignore the data_value field
                // ::core::hash::Hash::hash(&(*__self_0_5), state)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    
}
