use std::sync::Arc;
use crate::behavior::stateful_reader::StatefulReader;

use rust_dds_interface::protocol::{ProtocolEntity, ProtocolReader};
use rust_dds_interface::types::InstanceHandle;

pub struct Reader {
    reader: Arc<StatefulReader>,
}

impl Reader {
    pub fn new(reader: Arc<StatefulReader>) -> Self {
        Self {
            reader
        }
    }
}

impl ProtocolEntity for Reader {

    fn get_instance_handle(&self) -> InstanceHandle {
        todo!()
    }
}

impl ProtocolReader for Reader {

}