use std::collections::HashMap;

use crate::behavior::RtpsWriter;
use crate::behavior::endpoint_traits::{DestinedMessages, CacheChangeSender};
use crate::types::{Locator, ReliabilityKind, GUID};
use super::best_effort_reader_locator::BestEffortReaderLocator;
use rust_dds_interface::types::TopicKind;
use rust_dds_interface::history_cache::HistoryCache;

pub struct StatelessWriter {
    pub writer: RtpsWriter,
    reader_locators: HashMap<Locator, BestEffortReaderLocator>,
}

impl StatelessWriter {
    pub fn new(
        guid: GUID,
        topic_kind: TopicKind,
        reliability_level: ReliabilityKind,
        push_mode: bool,
        writer_cache: HistoryCache,
        data_max_sized_serialized: Option<i32>,
    ) -> Self {
        assert!(reliability_level == ReliabilityKind::BestEffort, "Only BestEffort is supported on stateless writer");

        let writer = RtpsWriter::new(guid, topic_kind, reliability_level, push_mode, writer_cache, data_max_sized_serialized);

        Self {
            writer,
            reader_locators: HashMap::new(),
        }
    }

    pub fn reader_locator_add(&mut self, a_locator: Locator) {
        self.reader_locators.insert(a_locator, BestEffortReaderLocator::new(a_locator));
    }

    pub fn reader_locator_remove(&mut self, a_locator: &Locator) {
        self.reader_locators.remove(a_locator);
    }

    pub fn unsent_changes_reset(&mut self) {
        for (_, rl) in self.reader_locators.iter_mut() {
            rl.unsent_changes_reset();
        }
    }
}

impl CacheChangeSender for StatelessWriter {
    fn produce_messages(&mut self) -> Vec<DestinedMessages> {
        let mut output = Vec::new();
        for (&locator, reader_locator) in self.reader_locators.iter_mut() {
            let messages = reader_locator.produce_messages(&self.writer.writer_cache, self.writer.endpoint.entity.guid.entity_id(), self.writer.last_change_sequence_number);
            if !messages.is_empty() {
                output.push(DestinedMessages::SingleDestination{locator, messages});
            }
        }
        output
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::types::constants::*;
//     use crate::types::*;

    // #[test]
    // fn stateless_writer_run() {
    //     // Create the stateless writer
    //     let mut stateless_writer = StatelessWriter::new(
    //         GUID::new([0; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER),
    //         TopicKind::WithKey,
    //         ReliabilityKind::BestEffort,
    //         HistoryCacheResourceLimits::default(),
    //     );

    //     // Add two locators
    //     let locator1 = Locator::new(0, 7400, [1; 16]);
    //     let locator2 = Locator::new(0, 7500, [2; 16]);
    //     stateless_writer.reader_locator_add(locator1);
    //     stateless_writer.reader_locator_add(locator2);

    //     let _cache_change_seq1 = stateless_writer.new_change(
    //         ChangeKind::Alive,
    //         Some(vec![1, 2, 3]), 
    //         None,                
    //         [1; 16],             
    //     );

    //     let cache_change_seq2 = stateless_writer.new_change(
    //         ChangeKind::Alive,
    //         Some(vec![4, 5, 6]), 
    //         None,                
    //         [1; 16],             
    //     );

    //     // stateless_writer.writer_cache().add_change(cache_change_seq1).unwrap();
    //     stateless_writer.writer_cache().add_change(cache_change_seq2).unwrap();

    //     stateless_writer.run();

    //     todo!()

    //     // let mut send_messages = stateless_writer.pop_send_messages();
    //     // assert_eq!(send_messages.len(), 2);

    //     // // Check that the two reader locators have messages sent to them. The order is not fixed so it can
    //     // // not be used for the test
    //     // send_messages.iter().find(|(dst_locator, _)| dst_locator == &vec![locator1]).unwrap();
    //     // send_messages.iter().find(|(dst_locator, _)| dst_locator == &vec![locator2]).unwrap();

    //     // let (_, send_messages_reader_locator_1) = send_messages.pop().unwrap();
    //     // let (_, send_messages_reader_locator_2) = send_messages.pop().unwrap();

    //     // // Check that the same messages are sent to both locators
    //     // assert_eq!(send_messages_reader_locator_1, send_messages_reader_locator_2);

    //     // if let RtpsSubmessage::Gap(_) = &send_messages_reader_locator_1[0] {
    //     //     // The contents of the message are tested in the reader locator so simply assert the type is correct
    //     //     assert!(true)
    //     // } else {
    //     //     panic!("Wrong message type");
    //     // };

    //     // if let RtpsSubmessage::Data(_) = &send_messages_reader_locator_1[1] {
    //     //         // The contents of the message are tested in the reader locator so simply assert the type is correct
    //     //         assert!(true)
    //     // } else {
    //     //     panic!("Wrong message type");
    //     // };

    //     // // Test that nothing more is sent after the first time
    //     // stateless_writer.run();
    //     // assert_eq!(stateless_writer.pop_send_messages().len(), 0);
    // }
// }
