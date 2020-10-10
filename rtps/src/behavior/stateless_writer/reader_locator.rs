use std::sync::Mutex;
use std::collections::{VecDeque, BTreeSet};

use crate::types::{Locator, SequenceNumber, EntityId};
use crate::types::constants::ENTITYID_UNKNOWN;
use crate::structure::HistoryCache;
use crate::messages::RtpsSubmessage;
use crate::messages::submessages::Gap;
use crate::behavior::{data_from_cache_change, BEHAVIOR_ENDIANNESS};

pub struct ReaderLocator {
    //requested_changes: HashSet<CacheChange>,
    // unsent_changes: SequenceNumber,
    locator: Locator,
    writer_entity_id: EntityId,
    expects_inline_qos: bool,

    highest_sequence_number_sent: SequenceNumber,

    send_messages: Mutex<VecDeque<RtpsSubmessage>>,
}

impl ReaderLocator {
    pub fn new(locator: Locator, writer_entity_id: EntityId, expects_inline_qos: bool) -> Self {
        Self {
            locator,
            writer_entity_id,
            expects_inline_qos,
            highest_sequence_number_sent:0,
            send_messages: Mutex::new(VecDeque::new()),
        }
    }

    pub fn unsent_changes_reset(&mut self) {
        self.highest_sequence_number_sent = 0;
    }

    pub fn unsent_changes(&self, last_change_sequence_number: SequenceNumber) -> BTreeSet<SequenceNumber> {
        let mut unsent_changes_set = BTreeSet::new();

        // The for loop is made with the underlying sequence number type because it is not possible to implement the Step trait on Stable yet
        for unsent_sequence_number in
            self.highest_sequence_number_sent + 1 ..= last_change_sequence_number
        {
            unsent_changes_set.insert(unsent_sequence_number);
        }

        unsent_changes_set
    }

    pub fn next_unsent_change(&mut self, last_change_sequence_number: SequenceNumber) -> Option<SequenceNumber> {
        let next_unsent_sequence_number = self.highest_sequence_number_sent + 1;
        if next_unsent_sequence_number > last_change_sequence_number {
            None
        } else {
            self.highest_sequence_number_sent = next_unsent_sequence_number;
            Some(next_unsent_sequence_number)
        }
    }

    pub fn run(&mut self, history_cache: &HistoryCache, last_change_sequence_number: SequenceNumber) {
        if !self.unsent_changes(last_change_sequence_number).is_empty() {
            self.pushing_state(history_cache, last_change_sequence_number);
        }
    }

    fn pushing_state(&mut self, history_cache: &HistoryCache, last_change_sequence_number: SequenceNumber) {
        // This state is only valid if there are unsent changes
        assert!(!self.unsent_changes(last_change_sequence_number).is_empty());
    
        while let Some(next_unsent_seq_num) = self.next_unsent_change(last_change_sequence_number) {
            self.transition_t4(history_cache, next_unsent_seq_num);
        }
    }

    fn transition_t4(&self, history_cache: &HistoryCache, next_unsent_seq_num: SequenceNumber) {
        if let Some(cache_change) = history_cache
            .changes().iter().find(|cc| cc.sequence_number() == next_unsent_seq_num)
        {
            let data = data_from_cache_change(cache_change, ENTITYID_UNKNOWN);
            self.send_messages.lock().unwrap().push_back(RtpsSubmessage::Data(data));
        } else {
            let gap = Gap::new(
                BEHAVIOR_ENDIANNESS,
                ENTITYID_UNKNOWN, 
                self.writer_entity_id,
                next_unsent_seq_num,
            BTreeSet::new());

            self.send_messages.lock().unwrap().push_back(RtpsSubmessage::Gap(gap));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::constants::ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;

    #[test]
    fn unsent_change_operations() {
        let locator = Locator::new_udpv4(7400, [127,0,0,1]);
        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;
        let expects_inline_qos = false;
        let mut reader_locator = ReaderLocator::new(locator, writer_entity_id, expects_inline_qos);

        let unsent_changes = reader_locator.unsent_changes(0);
        assert!(unsent_changes.is_empty());

        let unsent_changes = reader_locator.unsent_changes(2);
        assert_eq!(unsent_changes.len(), 2);
        assert!(unsent_changes.contains(&1));
        assert!(unsent_changes.contains(&2));

        let next_unsent_change = reader_locator.next_unsent_change(2).unwrap();
        assert_eq!(next_unsent_change, 1);
        let next_unsent_change = reader_locator.next_unsent_change(2).unwrap();
        assert_eq!(next_unsent_change, 2);
        let next_unsent_change = reader_locator.next_unsent_change(2);
        assert!(next_unsent_change.is_none());

        // Test also that the system is robust if the last_change_sequence_number input does not follow the precondition
        // of being a constantly increasing number
        let next_unsent_change = reader_locator.next_unsent_change(1);
        assert!(next_unsent_change.is_none());
    }

    #[test]
    fn unsent_changes_reset() {
        let locator = Locator::new_udpv4(7400, [127,0,0,1]);
        let writer_entity_id = ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER;
        let expects_inline_qos = false;
        let mut reader_locator = ReaderLocator::new(locator, writer_entity_id, expects_inline_qos);

        let next_unsent_change = reader_locator.next_unsent_change(2).unwrap();
        assert_eq!(next_unsent_change, 1);
        let next_unsent_change = reader_locator.next_unsent_change(2).unwrap();
        assert_eq!(next_unsent_change, 2);
        let next_unsent_change = reader_locator.next_unsent_change(2);
        assert!(next_unsent_change.is_none());

        reader_locator.unsent_changes_reset();

        let next_unsent_change = reader_locator.next_unsent_change(2).unwrap();
        assert_eq!(next_unsent_change, 1);
        let next_unsent_change = reader_locator.next_unsent_change(2).unwrap();
        assert_eq!(next_unsent_change, 2);
        let next_unsent_change = reader_locator.next_unsent_change(2);
        assert!(next_unsent_change.is_none());
    }
}