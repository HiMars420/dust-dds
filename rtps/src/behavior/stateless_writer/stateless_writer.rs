use std::collections::{HashMap,  VecDeque};
use std::sync::mpsc;
use crate::structure::HistoryCache;
use crate::structure::CacheChange;
use crate::serialized_payload::ParameterList;
use crate::types::{ChangeKind, InstanceHandle, Locator, ReliabilityKind, SequenceNumber, TopicKind, GUID, };
use crate::messages::RtpsSubmessage;
use super::reader_locator::ReaderLocator;

use rust_dds_interface::qos::DataWriterQos;

pub struct StatelessWriter {
    /// Entity base class (contains the GUID)
    guid: GUID,
    // entity: Entity,

    // Endpoint base class:
    /// Used to indicate whether the Endpoint supports instance lifecycle management operations. Indicates whether the Endpoint is associated with a DataType that has defined some fields as containing the DDS key.
    topic_kind: TopicKind,
    /// The level of reliability supported by the Endpoint.
    reliability_level: ReliabilityKind,

    // The stateless writer does not receive messages so these fields are left out
    /// List of unicast locators (transport, address, port combinations) that can be used to send messages to the Endpoint. The list may be empty
    // unicast_locator_list: Vec<Locator>,
    /// List of multicast locators (transport, address, port combinations) that can be used to send messages to the Endpoint. The list may be empty.
    // multicast_locator_list: Vec<Locator>,
    
    last_change_sequence_number: SequenceNumber,
    writer_cache: HistoryCache,
    data_max_sized_serialized: Option<i32>,

    reader_locators: HashMap<Locator, ReaderLocator>,

    sender: mpsc::Sender<(Vec<Locator>,RtpsSubmessage)>,
}

impl StatelessWriter {
    pub fn new(
        guid: GUID,
        topic_kind: TopicKind,
        writer_qos: &DataWriterQos,
        sender: mpsc::Sender<(Vec<Locator>,RtpsSubmessage)>,
    ) -> Self {
        StatelessWriter {
            guid,
            topic_kind,
            reliability_level: ReliabilityKind::BestEffort,
            last_change_sequence_number: 0,
            writer_cache: HistoryCache::new(&writer_qos.resource_limits),
            data_max_sized_serialized: None,
            reader_locators: HashMap::new(),
            sender
        }
    }

    pub fn new_change(
        &mut self,
        kind: ChangeKind,
        data: Option<Vec<u8>>,
        inline_qos: Option<ParameterList>,
        handle: InstanceHandle,
    ) -> CacheChange {
        self.last_change_sequence_number += 1;

        CacheChange::new(
            kind,
            self.guid,
            handle,
            self.last_change_sequence_number,
            data,
            inline_qos,
        )
    }

    pub fn writer_cache(&self) -> &HistoryCache {
        &self.writer_cache
    }

    pub fn reader_locator_add(&mut self, a_locator: Locator) {
        self.reader_locators.insert(a_locator, ReaderLocator::new(a_locator, self.guid.entity_id(), false /*expects_inline_qos*/, self.sender.clone()));
    }

    pub fn reader_locator_remove(&mut self, a_locator: &Locator) {
        self.reader_locators.remove(a_locator);
    }

    pub fn unsent_changes_reset(&mut self) {
        for (_, rl) in self.reader_locators.iter_mut() {
            rl.unsent_changes_reset();
        }
    }

    pub fn run(&mut self) {
        for (_, reader_locator) in self.reader_locators.iter_mut() {
            reader_locator.run(&self.writer_cache, self.last_change_sequence_number);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::constants::*;
    use crate::types::*;

    #[test]
    fn new_change() {
        let (sender, receiver) = mpsc::channel();

        let writer_qos = DataWriterQos::default();
        let mut writer = StatelessWriter::new(
            GUID::new([0; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER),
            TopicKind::WithKey,
            &writer_qos,
            sender
        );

        let cache_change_seq1 = writer.new_change(
            ChangeKind::Alive,
            Some(vec![1, 2, 3]), 
            None,                
            [1; 16],             
        );

        let cache_change_seq2 = writer.new_change(
            ChangeKind::NotAliveUnregistered,
            None,    
            None,    
            [1; 16], 
        );

        assert_eq!(cache_change_seq1.sequence_number(), 1);
        assert_eq!(cache_change_seq1.change_kind(), &ChangeKind::Alive);
        assert_eq!(cache_change_seq1.inline_qos().len(), 0);
        assert_eq!(cache_change_seq1.instance_handle(), &[1; 16]);

        assert_eq!(cache_change_seq2.sequence_number(), 2);
        assert_eq!(
            cache_change_seq2.change_kind(),
            &ChangeKind::NotAliveUnregistered
        );
        assert_eq!(cache_change_seq2.inline_qos().len(), 0);
        assert_eq!(cache_change_seq2.instance_handle(), &[1; 16]);
    }

    #[test]
    fn stateless_writer_run() {
        let (sender, _receiver) = mpsc::channel();

        // Create the stateless writer
        let writer_qos = DataWriterQos::default();

        let mut stateless_writer = StatelessWriter::new(
            GUID::new([0; 12], ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER),
            TopicKind::WithKey,
            &writer_qos,
            sender
        );

        // Add two locators
        let locator1 = Locator::new(0, 7400, [1; 16]);
        let locator2 = Locator::new(0, 7500, [2; 16]);
        stateless_writer.reader_locator_add(locator1);
        stateless_writer.reader_locator_add(locator2);

        let _cache_change_seq1 = stateless_writer.new_change(
            ChangeKind::Alive,
            Some(vec![1, 2, 3]), 
            None,                
            [1; 16],             
        );

        let cache_change_seq2 = stateless_writer.new_change(
            ChangeKind::Alive,
            Some(vec![4, 5, 6]), 
            None,                
            [1; 16],             
        );

        // stateless_writer.writer_cache().add_change(cache_change_seq1).unwrap();
        stateless_writer.writer_cache().add_change(cache_change_seq2).unwrap();

        stateless_writer.run();

        todo!()

        // let mut send_messages = stateless_writer.pop_send_messages();
        // assert_eq!(send_messages.len(), 2);

        // // Check that the two reader locators have messages sent to them. The order is not fixed so it can
        // // not be used for the test
        // send_messages.iter().find(|(dst_locator, _)| dst_locator == &vec![locator1]).unwrap();
        // send_messages.iter().find(|(dst_locator, _)| dst_locator == &vec![locator2]).unwrap();

        // let (_, send_messages_reader_locator_1) = send_messages.pop().unwrap();
        // let (_, send_messages_reader_locator_2) = send_messages.pop().unwrap();

        // // Check that the same messages are sent to both locators
        // assert_eq!(send_messages_reader_locator_1, send_messages_reader_locator_2);

        // if let RtpsSubmessage::Gap(_) = &send_messages_reader_locator_1[0] {
        //     // The contents of the message are tested in the reader locator so simply assert the type is correct
        //     assert!(true)
        // } else {
        //     panic!("Wrong message type");
        // };

        // if let RtpsSubmessage::Data(_) = &send_messages_reader_locator_1[1] {
        //         // The contents of the message are tested in the reader locator so simply assert the type is correct
        //         assert!(true)
        // } else {
        //     panic!("Wrong message type");
        // };

        // // Test that nothing more is sent after the first time
        // stateless_writer.run();
        // assert_eq!(stateless_writer.pop_send_messages().len(), 0);
    }
}
