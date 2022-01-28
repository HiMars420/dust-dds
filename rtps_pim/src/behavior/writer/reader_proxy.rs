use crate::structure::types::{EntityId, Guid, Locator, SequenceNumber};

pub trait RtpsReaderProxyConstructor {
    fn new(
        remote_reader_guid: Guid,
        remote_group_entity_id: EntityId,
        unicast_locator_list: &[Locator],
        multicast_locator_list: &[Locator],
        expects_inline_qos: bool,
    ) -> Self;
}

pub trait RtpsReaderProxyAttributes {
    fn remote_reader_guid(&self) -> &Guid;
    fn remote_group_entity_id(&self) -> &EntityId;
    fn unicast_locator_list(&self) -> &[Locator];
    fn multicast_locator_list(&self) -> &[Locator];
    fn expects_inline_qos(&self) -> &bool;
    fn is_active(&self) -> &bool;
}

pub trait RtpsReaderProxyOperations {
    type ChangeForReaderType;
    type ChangeForReaderListType;

    fn acked_changes_set(&mut self, committed_seq_num: SequenceNumber);
    fn next_requested_change(&mut self) -> Option<Self::ChangeForReaderType>;
    fn next_unsent_change(&mut self) -> Option<Self::ChangeForReaderType>;
    fn unsent_changes(&self) -> Self::ChangeForReaderListType;
    fn requested_changes(&self) -> Self::ChangeForReaderListType;
    fn requested_changes_set(&mut self, req_seq_num_set: &[SequenceNumber]);
    fn unacked_changes(&self) -> Self::ChangeForReaderListType;
}
