use std::collections::BTreeSet;

use crate::types::EntityId;
use crate::messages::RtpsSubmessage;
use crate::messages::submessages::Gap;
use crate::behavior::ReaderProxy;
use crate::behavior::{data_from_cache_change, BEHAVIOR_ENDIANNESS};

use rust_dds_interface::history_cache::HistoryCache;
use rust_dds_interface::types::SequenceNumber;

pub struct BestEffortReaderProxy(ReaderProxy);

impl BestEffortReaderProxy {
    pub fn new(reader_proxy: ReaderProxy) -> Self {
        Self(reader_proxy)
    }

    pub fn produce_messages(&mut self, history_cache: &HistoryCache, writer_entity_id: EntityId, last_change_sequence_number: SequenceNumber) -> Vec<RtpsSubmessage> {
        let mut messages = Vec::new();
        if !self.unsent_changes(last_change_sequence_number).is_empty() {
            self.pushing_state(history_cache, last_change_sequence_number, writer_entity_id, &mut messages);
        }
        messages
    }

    fn pushing_state(&mut self, history_cache: &HistoryCache, last_change_sequence_number: SequenceNumber, writer_entity_id: EntityId, message_queue: &mut Vec<RtpsSubmessage>) {
        while let Some(next_unsent_seq_num) = self.next_unsent_change(last_change_sequence_number) {
            self.transition_t4(history_cache, next_unsent_seq_num, writer_entity_id, message_queue);
        }
    }

    fn transition_t4(&mut self, history_cache: &HistoryCache, next_unsent_seq_num: SequenceNumber, writer_entity_id: EntityId, message_queue: &mut Vec<RtpsSubmessage>) {
        if let Some(cache_change) = history_cache.get_change(next_unsent_seq_num) {
            let reader_id = self.remote_reader_guid.entity_id();
            let data = data_from_cache_change(cache_change, reader_id);
            let mut dst_locator = self.unicast_locator_list.clone();
            dst_locator.extend(&self.unicast_locator_list);
            dst_locator.extend(&self.multicast_locator_list);
            message_queue.push(RtpsSubmessage::Data(data));
        } else {
            let gap = Gap::new(
                BEHAVIOR_ENDIANNESS,
                self.remote_reader_guid.entity_id(), 
                writer_entity_id,
                next_unsent_seq_num,
            BTreeSet::new());
            let mut dst_locator = self.unicast_locator_list.clone();
            dst_locator.extend(&self.unicast_locator_list);
            dst_locator.extend(&self.multicast_locator_list);
            message_queue.push(RtpsSubmessage::Gap(gap));
        }
    }
}

impl std::ops::Deref for BestEffortReaderProxy {
    type Target = ReaderProxy;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for BestEffortReaderProxy {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::constants::{ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER, ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER};
    use crate::types::{GUID, Locator};

    use rust_dds_interface::cache_change::CacheChange;
    use rust_dds_interface::types::ChangeKind;

    #[test]
    fn produce_data_and_gap_messages() {
        let remote_reader_guid = GUID::new([5;12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER);
        let unicast_locator_list = vec![Locator::new_udpv4(7400, [127, 0, 0, 1])];
        let multicast_locator_list = vec![];
        let expects_inline_qos = false;
        let is_active = true;
        let reader_proxy = ReaderProxy::new(remote_reader_guid, unicast_locator_list, multicast_locator_list, expects_inline_qos, is_active);
        let mut best_effort_reader_proxy = BestEffortReaderProxy::new(reader_proxy);

        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;
        let mut history_cache = HistoryCache::default();
        // Run without any change being created or added in the cache. No message should be sent
        let last_change_sequence_number = 0;
        let messages_vec = best_effort_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
        );

        assert!(messages_vec.is_empty());

        // Add one change to the history cache and run with that change as the last one. One Data submessage should be sent
        let writer_guid = GUID::new([5; 12], writer_entity_id);
        let instance_handle = [1; 16];
        let cache_change_seq1 = CacheChange::new(
            ChangeKind::Alive,
            writer_guid.into(),
            instance_handle,
            1,
            Some(vec![1, 2, 3]),
            None,
        );
        let expected_data_submessage = data_from_cache_change(&cache_change_seq1, best_effort_reader_proxy.remote_reader_guid.entity_id());
        history_cache.add_change(cache_change_seq1).unwrap();

        let last_change_sequence_number = 1;
        let mut messages_vec = best_effort_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
        );

        let expected_submessage = RtpsSubmessage::Data(expected_data_submessage);
        let sent_message = messages_vec.pop().unwrap();
        assert!(messages_vec.is_empty());
        assert_eq!(sent_message, expected_submessage);

        // Run with the next sequence number without adding any change to the history cache. One Gap submessage should be sent
        let last_change_sequence_number = 2;
        let mut messages_vec = best_effort_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
        );

        let expected_submessage = RtpsSubmessage::Gap(Gap::new(
            BEHAVIOR_ENDIANNESS,
            best_effort_reader_proxy.remote_reader_guid.entity_id(),
            writer_entity_id,
            2,
            BTreeSet::new(),
        ));
        let sent_message = messages_vec.pop().unwrap();
        assert!(messages_vec.is_empty());
        assert_eq!(sent_message, expected_submessage);

        // Add one change to the history cache skipping one sequence number. One Gap and one Data submessage should be sent
        let cache_change_seq4 = CacheChange::new(
            ChangeKind::Alive,
            writer_guid.into(),
            instance_handle,
            4,
            Some(vec![4, 5, 6]),
            None,
        );
        let expected_data_submessage = data_from_cache_change(&cache_change_seq4, best_effort_reader_proxy.remote_reader_guid.entity_id());
        history_cache.add_change(cache_change_seq4).unwrap();

        let last_change_sequence_number = 4;
        let mut messages_vec = best_effort_reader_proxy.produce_messages(
            &history_cache,
            writer_entity_id,
            last_change_sequence_number,
        );

        let expected_gap_submessage = RtpsSubmessage::Gap(Gap::new(
            BEHAVIOR_ENDIANNESS,
            best_effort_reader_proxy.remote_reader_guid.entity_id(),
            writer_entity_id,
            3,
            BTreeSet::new(),
        ));
        let expected_data_submessage = RtpsSubmessage::Data(expected_data_submessage);

        let sent_message_2 = messages_vec.pop().unwrap();
        let sent_message_1 = messages_vec.pop().unwrap();
        assert!(messages_vec.is_empty());
        assert_eq!(sent_message_1, expected_gap_submessage);
        assert_eq!(sent_message_2, expected_data_submessage);
    }
}