use rust_rtps_pim::{
    behavior::writer::reader_proxy::{RTPSReaderProxy, RTPSReaderProxyOperations},
    structure::types::{EntityId, Locator, SequenceNumber, GUID},
};

pub struct RTPSReaderProxyImpl {
    remote_reader_guid: GUID,
    remote_group_entity_id: EntityId,
    unicast_locator_list: Vec<Locator>,
    multicast_locator_list: Vec<Locator>,
    expects_inline_qos: bool,
    is_active: bool,
    _last_sent_sequence_number: SequenceNumber,
}

impl RTPSReaderProxy for RTPSReaderProxyImpl {
    fn remote_reader_guid(&self) -> &GUID {
        &self.remote_reader_guid
    }

    fn remote_group_entity_id(&self) -> &EntityId {
        &self.remote_group_entity_id
    }

    fn unicast_locator_list(&self) -> &[Locator] {
        &self.unicast_locator_list
    }

    fn multicast_locator_list(&self) -> &[Locator] {
        &self.multicast_locator_list
    }

    fn expects_inline_qos(&self) -> bool {
        self.expects_inline_qos
    }

    fn is_active(&self) -> bool {
        self.is_active
    }
}

impl RTPSReaderProxyOperations for RTPSReaderProxyImpl {
    type SequenceNumberVector = Vec<SequenceNumber>;

    fn new(
        remote_reader_guid: GUID,
        remote_group_entity_id: EntityId,
        unicast_locator_list: &[Locator],
        multicast_locator_list: &[Locator],
        expects_inline_qos: bool,
        is_active: bool,
    ) -> Self {
        Self {
            remote_reader_guid,
            remote_group_entity_id,
            unicast_locator_list: unicast_locator_list.into_iter().cloned().collect(),
            multicast_locator_list: multicast_locator_list.into_iter().cloned().collect(),
            expects_inline_qos,
            is_active,
            _last_sent_sequence_number: 0.into(),
        }
    }

    fn acked_changes_set(&mut self, _committed_seq_num: SequenceNumber) {
        todo!()
    }

    fn next_requested_change(&mut self) -> Option<SequenceNumber> {
        todo!()
    }

    fn next_unsent_change(&mut self) -> Option<SequenceNumber> {
        todo!()
    }

    fn unsent_changes(&self) -> Self::SequenceNumberVector {
        todo!()
    }

    fn requested_changes(&self) -> Self::SequenceNumberVector {
        todo!()
    }

    fn requested_changes_set(&mut self, _req_seq_num_set: Self::SequenceNumberVector) {
        todo!()
    }

    fn unacked_changes(&self) -> Self::SequenceNumberVector {
        todo!()
    }
}
