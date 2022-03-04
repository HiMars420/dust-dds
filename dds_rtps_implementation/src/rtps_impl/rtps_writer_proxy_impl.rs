use rust_rtps_pim::{
    behavior::reader::writer_proxy::{
        RtpsWriterProxyAttributes, RtpsWriterProxyConstructor, RtpsWriterProxyOperations,
    },
    structure::types::{EntityId, Guid, Locator, SequenceNumber},
};

#[derive(Debug, PartialEq)]
pub struct RtpsWriterProxyImpl {
    remote_writer_guid: Guid,
    unicast_locator_list: Vec<Locator>,
    multicast_locator_list: Vec<Locator>,
    data_max_size_serialized: Option<i32>,
    remote_group_entity_id: EntityId,
}

impl RtpsWriterProxyConstructor for RtpsWriterProxyImpl {
    fn new(
        remote_writer_guid: Guid,
        unicast_locator_list: &[Locator],
        multicast_locator_list: &[Locator],
        data_max_size_serialized: Option<i32>,
        remote_group_entity_id: EntityId,
    ) -> Self {
        Self {
            remote_writer_guid,
            unicast_locator_list: unicast_locator_list.to_vec(),
            multicast_locator_list: multicast_locator_list.to_vec(),
            data_max_size_serialized,
            remote_group_entity_id,
        }
    }
}

impl RtpsWriterProxyAttributes for RtpsWriterProxyImpl {
    fn remote_writer_guid(&self) -> Guid {
        self.remote_writer_guid
    }

    fn unicast_locator_list(&self) -> &[Locator] {
        todo!()
    }

    fn multicast_locator_list(&self) -> &[Locator] {
        todo!()
    }

    fn data_max_size_serialized(&self) -> Option<i32> {
        todo!()
    }

    fn remote_group_entity_id(&self) -> EntityId {
        todo!()
    }
}

impl RtpsWriterProxyOperations for RtpsWriterProxyImpl {
    fn available_changes_max(&self) -> SequenceNumber {
        todo!()
    }

    fn irrelevant_change_set(&mut self, _a_seq_num: SequenceNumber) {
        todo!()
    }

    fn lost_changes_update(&mut self, _first_available_seq_num: SequenceNumber) {
        todo!()
    }

    fn missing_changes(&self) -> &[SequenceNumber] {
        todo!()
    }

    fn missing_changes_update(&mut self, _last_available_seq_num: SequenceNumber) {
        todo!()
    }

    fn received_change_set(&mut self, _a_seq_num: SequenceNumber) {
        // FIND change FROM this.changes_from_writer
        //     SUCH-THAT change.sequenceNumber == a_seq_num;
        // change.status := RECEIVED
        println!("/!\\ RtpsWriterProxyImpl: received_change_set from RtpsWriterProxyOperations not implemented");
    }
}
