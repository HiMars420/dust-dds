use crate::structure::{RtpsGroup, RtpsEntity};
use crate::types::{GUID, EntityId, EntityKind, ReliabilityKind,};
use crate::behavior::StatefulWriter;
use crate::behavior::types::Duration;

use crate::writer::Writer;

use rust_dds_interface::protocol::{ProtocolPublisher, ProtocolEntity, ProtocolWriter};
use rust_dds_interface::types::{InstanceHandle, TopicKind};
use rust_dds_interface::qos::DataWriterQos;
use rust_dds_interface::qos_policy::ReliabilityQosPolicyKind;
use rust_dds_interface::history_cache::HistoryCache;

pub struct Publisher {
    group: RtpsGroup,
}

impl Publisher {
    pub fn new(group: RtpsGroup) -> Self {        
        Self {
            group
        }
    }
}

impl ProtocolEntity for Publisher {
    fn get_instance_handle(&self) -> InstanceHandle {
        todo!()
        // self.group.guid().into()
    }
}

impl ProtocolPublisher for Publisher {
    fn create_writer(&mut self, topic_kind: TopicKind, data_writer_qos: &DataWriterQos) -> Box<dyn ProtocolWriter> {
        todo!()
        // let dummy_counter = 1;
        // let group_guid = self.group.guid();
        // let entity_kind = match topic_kind {
        //     TopicKind::NoKey => EntityKind::UserDefinedReaderNoKey,
        //     TopicKind::WithKey => EntityKind::UserDefinedReaderWithKey,
        // };
        // let entity_key = [group_guid.entity_id().entity_key()[0], dummy_counter as u8, 0];
        // let entity_id = EntityId::new(entity_key, entity_kind);
        // // self.endpoint_counter += 1;
        // let writer_guid = GUID::new(group_guid.prefix(), entity_id);

        // let reliability_level = match data_writer_qos.reliability.kind {
        //     ReliabilityQosPolicyKind::ReliableReliabilityQos => ReliabilityKind::Reliable,
        //     ReliabilityQosPolicyKind::BestEffortReliabilityQos => ReliabilityKind::BestEffort,
        // };

        // let writer_cache = HistoryCache::new(data_writer_qos.resource_limits.clone());

        // let push_mode = true;
        // let heartbeat_period = Duration::from_millis(500);
        // let nack_response_delay = Duration::from_millis(0);
        // let nack_supression_duration = Duration::from_millis(0);

        // let new_writer = Arc::new(
        //     StatefulWriter::new(
        //         writer_guid,
        //         topic_kind,
        //         reliability_level,
        //         writer_cache,
        //         push_mode,
        //         heartbeat_period,
        //         nack_response_delay,
        //         nack_supression_duration,
        //     ));

        // Box::new(Writer::new(new_writer))
    }
}