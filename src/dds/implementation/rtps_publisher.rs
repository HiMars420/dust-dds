use std::sync::{atomic, Arc, Mutex};

use crate::{dds::{
        infrastructure::{
            qos::{DataWriterQos, PublisherQos},
            status::StatusMask,
        },
        publication::publisher_listener::PublisherListener,
    }, rtps::{structure::Group, types::{EntityId, GUID, constants::{ENTITY_KIND_BUILT_IN_WRITER_NO_KEY, ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY, ENTITY_KIND_USER_DEFINED_WRITER_NO_KEY, ENTITY_KIND_USER_DEFINED_WRITER_WITH_KEY}}}, types::{DDSType, ReturnCode, TopicKind}, utils::maybe_valid::{MaybeValidList, MaybeValidRef}};

use super::{
    rtps_datawriter::{AnyRtpsWriter, RtpsDataWriter},
    rtps_topic::AnyRtpsTopic,
};

enum Statefulness {
    Stateless,
    Stateful
}
enum EntityType {
    BuiltIn(Statefulness),
    UserDefined,
}

pub struct RtpsPublisher {
    pub group: Group,
    pub writer_list: MaybeValidList<Box<dyn AnyRtpsWriter>>,
    pub writer_count: atomic::AtomicU8,
    pub default_datawriter_qos: Mutex<DataWriterQos>,
    pub qos: PublisherQos,
    pub listener: Option<Box<dyn PublisherListener>>,
    pub status_mask: StatusMask,
}

impl RtpsPublisher {
    pub fn new(
        guid: GUID,
        qos: PublisherQos,
        listener: Option<Box<dyn PublisherListener>>,
        status_mask: StatusMask,
    ) -> Self {
        Self {
            group: Group::new(guid),
            writer_list: Default::default(),
            writer_count: atomic::AtomicU8::new(0),
            default_datawriter_qos: Mutex::new(DataWriterQos::default()),
            qos,
            listener,
            status_mask,
        }
    }

    pub fn create_stateful_builtin_datawriter<T: DDSType>(
        &self,
        a_topic: Arc<dyn AnyRtpsTopic>,
        qos: Option<DataWriterQos>,
        // _a_listener: impl DataWriterListener<T>,
        // _mask: StatusMask
    ) -> Option<MaybeValidRef<Box<dyn AnyRtpsWriter>>> {
        self.create_datawriter::<T>(a_topic, qos, &EntityType::BuiltIn(Statefulness::Stateful))
    }

    pub fn create_stateless_builtin_datawriter<T: DDSType>(
        &self,
        a_topic: Arc<dyn AnyRtpsTopic>,
        qos: Option<DataWriterQos>,
        // _a_listener: impl DataWriterListener<T>,
        // _mask: StatusMask
    ) -> Option<MaybeValidRef<Box<dyn AnyRtpsWriter>>> {
        self.create_datawriter::<T>(a_topic, qos, &EntityType::BuiltIn(Statefulness::Stateless))
    }

    pub fn create_user_defined_datawriter<T: DDSType>(
        &self,
        a_topic: Arc<dyn AnyRtpsTopic>,
        qos: Option<DataWriterQos>,
        // _a_listener: impl DataWriterListener<T>,
        // _mask: StatusMask
    ) -> Option<MaybeValidRef<Box<dyn AnyRtpsWriter>>> {
        self.create_datawriter::<T>(a_topic, qos, &EntityType::UserDefined)
    }

    fn create_datawriter<T: DDSType>(
        &self,
        a_topic: Arc<dyn AnyRtpsTopic>,
        qos: Option<DataWriterQos>,
        entity_type: &EntityType,
        // _a_listener: impl DataWriterListener<T>,
        // _mask: StatusMask
    ) -> Option<MaybeValidRef<Box<dyn AnyRtpsWriter>>> {
        let guid_prefix = self.group.entity.guid.prefix();
        let entity_key = [
            0,
            self.writer_count.fetch_add(1, atomic::Ordering::Relaxed),
            0,
        ];
        let entity_kind = match (a_topic.topic_kind(), entity_type){
            (TopicKind::WithKey, EntityType::UserDefined) => ENTITY_KIND_USER_DEFINED_WRITER_WITH_KEY,
            (TopicKind::NoKey, EntityType::UserDefined) => ENTITY_KIND_USER_DEFINED_WRITER_NO_KEY,
            (TopicKind::WithKey, EntityType::BuiltIn(_)) => ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY,
            (TopicKind::NoKey, EntityType::BuiltIn(_)) => ENTITY_KIND_BUILT_IN_WRITER_NO_KEY,
        };
        let entity_id = EntityId::new(entity_key, entity_kind);
        let guid = GUID::new(guid_prefix, entity_id);
        let qos = qos.unwrap_or(self.get_default_datawriter_qos());
        let writer: RtpsDataWriter<T> = match entity_type {            
            EntityType::UserDefined => RtpsDataWriter::new_stateful(guid, a_topic, qos, None, 0),
            EntityType::BuiltIn(Statefulness::Stateful) => RtpsDataWriter::new_stateful(guid, a_topic, qos, None, 0),
            EntityType::BuiltIn(Statefulness::Stateless) => RtpsDataWriter::new_stateless(guid, a_topic, qos, None, 0)
        };
        self.writer_list.add(Box::new(writer))
    }

    pub fn get_default_datawriter_qos(&self) -> DataWriterQos {
        self.default_datawriter_qos.lock().unwrap().clone()
    }

    pub fn set_default_datawriter_qos(&self, qos: Option<DataWriterQos>) -> ReturnCode<()> {
        let datawriter_qos = qos.unwrap_or_default();
        datawriter_qos.is_consistent()?;
        *self.default_datawriter_qos.lock().unwrap() = datawriter_qos;
        Ok(())
    }
}
