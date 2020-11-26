use crate::structure::RtpsGroup;
use crate::types::{EntityId, EntityKey, EntityKind, GuidPrefix, GUID};

use crate::writer::Writer;

use rust_dds_interface::qos::DataWriterQos;
use rust_dds_interface::types::{InstanceHandle, TopicKind, ReturnCode};

pub struct Publisher {
    group: RtpsGroup,
    writer_counter: usize,
}

impl Publisher {
    pub fn new(guid_prefix: GuidPrefix, entity_key: EntityKey) -> Self {
        let entity_id = EntityId::new(entity_key, EntityKind::UserDefinedWriterGroup);
        let publisher_guid = GUID::new(guid_prefix, entity_id);
        let group = RtpsGroup::new(publisher_guid);

        Self {
            group,
            writer_counter: 0,
        }
    }

    pub fn create_writer(
        &mut self,
        topic_kind: TopicKind,
        data_writer_qos: &DataWriterQos,
    ) -> ReturnCode<Writer> {
        let guid_prefix = self.group.entity.guid.prefix();
        let entity_key = [
            self.group.entity.guid.entity_id().entity_key()[0],
            self.writer_counter as u8,
            0,
        ];

        self.writer_counter += 1;

        Ok(Writer::new(
            guid_prefix,
            entity_key,
            topic_kind,
            data_writer_qos,
        ))
    }

    pub fn get_instance_handle(&self) -> InstanceHandle {
        self.group.entity.guid.into()
    }
}