use std::collections::HashMap;

use crate::structure::{HistoryCache, RtpsEndpoint, RtpsEntity, RtpsRun};
use crate::types::{Locator, ReliabilityKind, TopicKind, GUID, GuidPrefix };
use crate::messages::RtpsSubmessage;
use crate::behavior::types::Duration;

use crate::behavior::WriterProxy;
use super::best_effort_writer_proxy::BestEffortWriterProxy;
use super::reliable_writer_proxy::ReliableWriterProxy;

use rust_dds_interface::protocol::{ProtocolEntity, ProtocolReader};
use rust_dds_interface::qos::DataReaderQos;
use rust_dds_interface::types::{InstanceHandle, ReturnCode};

pub trait WriterProxyOps {
    fn run(&mut self, history_cache: &HistoryCache);
    fn try_push_message(&mut self, src_locator: Locator, src_guid_prefix: GuidPrefix, submessage: &mut Option<RtpsSubmessage>);
}

pub struct StatefulReader {
    // From Entity base class
    guid: GUID,
    // entity: Entity,

    // From Endpoint base class:
    topic_kind: TopicKind,
    reliability_level: ReliabilityKind,

    // All communication to this reader is done by the writer proxies
    // so these fields are unnecessary
    // unicast_locator_list: Vec<Locator>,
    // multicast_locator_list: Vec<Locator>,

    // From Reader base class:
    expects_inline_qos: bool,
    heartbeat_response_delay: Duration,

    reader_cache: HistoryCache,

    // Fields
    matched_writers: HashMap<GUID, Box<dyn WriterProxyOps>>,
}

impl StatefulReader {
    pub fn new(
        guid: GUID,
        topic_kind: TopicKind,
        reader_qos: &DataReaderQos
        ) -> Self {
            
        let expects_inline_qos = false;
        let heartbeat_response_delay = Duration::from_millis(500);
        
        Self {
            guid,
            topic_kind,
            reliability_level: reader_qos.reliability.kind.into(),
            expects_inline_qos,
            heartbeat_response_delay,       
            reader_cache: HistoryCache::new(&reader_qos.resource_limits),
            matched_writers: HashMap::new()
        }
    }

    pub fn matched_writer_add(&mut self, a_writer_proxy: WriterProxy) {
        let remote_writer_guid = a_writer_proxy.remote_writer_guid().clone();
        let writer_proxy: Box<dyn WriterProxyOps> = match self.reliability_level {
            ReliabilityKind::Reliable => Box::new(ReliableWriterProxy::new(a_writer_proxy, self.guid.entity_id(), self.heartbeat_response_delay)),
            ReliabilityKind::BestEffort => Box::new(BestEffortWriterProxy::new(a_writer_proxy)),
        };
        
        self.matched_writers.insert(remote_writer_guid, writer_proxy);
    }

    pub fn matched_writer_remove(&mut self, writer_proxy_guid: &GUID) {
        self.matched_writers.remove(writer_proxy_guid);
    }

    pub fn reader_cache(&self) -> &HistoryCache {
        &self.reader_cache
    }

    pub fn heartbeat_response_delay(&self) -> Duration {
        self.heartbeat_response_delay
    }

    pub fn guid(&self) -> &GUID {
        &self.guid
    }
}

impl RtpsRun for StatefulReader {
    fn run(&mut self) {
        for (_writer_guid, writer_proxy) in self.matched_writers.iter_mut(){
            writer_proxy.run(&self.reader_cache)
        }
    }
}

impl ProtocolEntity for StatefulReader {
    fn enable(&self) -> ReturnCode<()> {
        todo!()
    }

    fn get_instance_handle(&self) -> InstanceHandle {
        self.guid.into()
    }
}

impl ProtocolReader for StatefulReader {}

impl RtpsEntity for StatefulReader {
    fn guid(&self) -> GUID {
        self.guid
    }
}

impl RtpsEndpoint for StatefulReader {
    fn try_push_message(&mut self, src_locator: Locator, src_guid_prefix: GuidPrefix, submessage: &mut Option<RtpsSubmessage>) {
        for (_, writer_proxy) in &mut self.matched_writers {
            writer_proxy.try_push_message(src_locator, src_guid_prefix, submessage)
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::types::constants::ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER;


}
