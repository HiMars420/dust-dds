use std::{
    any::Any,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    dds::{
        infrastructure::{
            qos::DataWriterQos, qos_policy::ReliabilityQosPolicyKind, status::StatusMask,
        },
        publication::data_writer_listener::DataWriterListener,
    },
    rtps::{
        behavior::{
            self, endpoint_traits::CacheChangeSender, StatefulWriter, StatelessWriter, Writer,
        },
        types::{
            constants::{
                ENTITY_KIND_BUILT_IN_WRITER_NO_KEY, ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY,
                ENTITY_KIND_USER_DEFINED_WRITER_NO_KEY, ENTITY_KIND_USER_DEFINED_WRITER_WITH_KEY,
            },
            EntityId, EntityKey, GuidPrefix, ReliabilityKind, GUID,
        },
    },
    types::{DDSType, InstanceHandle, ReturnCode, ReturnCodes, Time, TopicKind},
    utils::{
        as_any::AsAny,
        maybe_valid::{MaybeValid, MaybeValidRef},
    },
};

use super::rtps_topic::{AnyRtpsTopic, RtpsAnyTopicRef};

pub enum WriterFlavor {
    Stateful(StatefulWriter),
    Stateless(StatelessWriter),
}
impl WriterFlavor {
    pub fn try_get_stateless(&mut self) -> Option<&mut StatelessWriter> {
        match self {
            WriterFlavor::Stateless(writer) => Some(writer),
            WriterFlavor::Stateful(_) => None,
        }
    }

    pub fn try_get_stateful(&mut self) -> Option<&mut StatefulWriter> {
        match self {
            WriterFlavor::Stateless(_) => None,
            WriterFlavor::Stateful(writer) => Some(writer),
        }
    }
}
impl Deref for WriterFlavor {
    type Target = Writer;

    fn deref(&self) -> &Self::Target {
        match self {
            WriterFlavor::Stateful(writer) => writer,
            WriterFlavor::Stateless(writer) => writer,
        }
    }
}
impl DerefMut for WriterFlavor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            WriterFlavor::Stateful(writer) => writer,
            WriterFlavor::Stateless(writer) => writer,
        }
    }
}

enum EntityType {
    BuiltIn,
    UserDefined,
}

pub struct RtpsDataWriter<T: DDSType> {
    pub writer: Mutex<WriterFlavor>,
    pub qos: Mutex<DataWriterQos>,
    pub topic: Mutex<Option<Arc<dyn AnyRtpsTopic>>>,
    pub listener: Option<Box<dyn DataWriterListener<T>>>,
    pub status_mask: StatusMask,
}

impl<T: DDSType> RtpsDataWriter<T> {
    pub fn new_builtin_stateless(
        guid_prefix: GuidPrefix,
        entity_key: EntityKey,
        topic: &RtpsAnyTopicRef,
        qos: DataWriterQos,
        listener: Option<Box<dyn DataWriterListener<T>>>,
        status_mask: StatusMask,
    ) -> Self {
        let entity_kind = match T::topic_kind() {
            TopicKind::NoKey => ENTITY_KIND_BUILT_IN_WRITER_NO_KEY,
            TopicKind::WithKey => ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY,
        };
        let entity_id = EntityId::new(entity_key, entity_kind);
        let guid = GUID::new(guid_prefix, entity_id);
        Self::new_stateless(guid, topic, qos, listener, status_mask)
    }

    pub fn new_user_defined_stateless(
        guid_prefix: GuidPrefix,
        entity_key: EntityKey,
        topic: &RtpsAnyTopicRef,
        qos: DataWriterQos,
        listener: Option<Box<dyn DataWriterListener<T>>>,
        status_mask: StatusMask,
    ) -> Self {
        let entity_kind = match T::topic_kind() {
            TopicKind::NoKey => ENTITY_KIND_USER_DEFINED_WRITER_NO_KEY,
            TopicKind::WithKey => ENTITY_KIND_USER_DEFINED_WRITER_WITH_KEY,
        };
        let entity_id = EntityId::new(entity_key, entity_kind);
        let guid = GUID::new(guid_prefix, entity_id);
        Self::new_stateless(guid, topic, qos, listener, status_mask)
    }

    pub fn new_builtin_stateful(
        guid_prefix: GuidPrefix,
        entity_key: EntityKey,
        topic: &RtpsAnyTopicRef,
        qos: DataWriterQos,
        listener: Option<Box<dyn DataWriterListener<T>>>,
        status_mask: StatusMask,
    ) -> Self {
        let entity_kind = match T::topic_kind() {
            TopicKind::NoKey => ENTITY_KIND_BUILT_IN_WRITER_NO_KEY,
            TopicKind::WithKey => ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY,
        };
        let entity_id = EntityId::new(entity_key, entity_kind);
        let guid = GUID::new(guid_prefix, entity_id);
        Self::new_stateful(guid, topic, qos, listener, status_mask)
    }

    pub fn new_user_defined_stateful(
        guid_prefix: GuidPrefix,
        entity_key: EntityKey,
        topic: &RtpsAnyTopicRef,
        qos: DataWriterQos,
        listener: Option<Box<dyn DataWriterListener<T>>>,
        status_mask: StatusMask,
    ) -> Self {
        let entity_kind = match T::topic_kind() {
            TopicKind::NoKey => ENTITY_KIND_BUILT_IN_WRITER_NO_KEY,
            TopicKind::WithKey => ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY,
        };
        let entity_id = EntityId::new(entity_key, entity_kind);
        let guid = GUID::new(guid_prefix, entity_id);
        Self::new_stateful(guid, topic, qos, listener, status_mask)
    }

    fn new_stateful(
        guid: GUID,
        topic: &RtpsAnyTopicRef,
        qos: DataWriterQos,
        listener: Option<Box<dyn DataWriterListener<T>>>,
        status_mask: StatusMask,
    ) -> Self {
        assert!(
            qos.is_consistent().is_ok(),
            "RtpsDataWriter can only be created with consistent QoS"
        );
        let topic = topic.get().unwrap().clone();
        let topic_kind = topic.topic_kind();
        let reliability_level = match qos.reliability.kind {
            ReliabilityQosPolicyKind::BestEffortReliabilityQos => ReliabilityKind::BestEffort,
            ReliabilityQosPolicyKind::ReliableReliabilityQos => ReliabilityKind::Reliable,
        };
        let push_mode = true;
        let data_max_sized_serialized = None;
        let heartbeat_period = behavior::types::Duration::from_millis(500);
        let nack_response_delay = behavior::types::constants::DURATION_ZERO;
        let nack_supression_duration = behavior::types::constants::DURATION_ZERO;
        let writer = StatefulWriter::new(
            guid,
            topic_kind,
            reliability_level,
            push_mode,
            data_max_sized_serialized,
            heartbeat_period,
            nack_response_delay,
            nack_supression_duration,
        );

        Self {
            writer: Mutex::new(WriterFlavor::Stateful(writer)),
            qos: Mutex::new(qos),
            topic: Mutex::new(Some(topic)),
            listener,
            status_mask,
        }
    }

    fn new_stateless(
        guid: GUID,
        topic: &RtpsAnyTopicRef,
        qos: DataWriterQos,
        listener: Option<Box<dyn DataWriterListener<T>>>,
        status_mask: StatusMask,
    ) -> Self {
        assert!(
            qos.is_consistent().is_ok(),
            "RtpsDataWriter can only be created with consistent QoS"
        );
        let topic = topic.get().unwrap().clone();
        let topic_kind = topic.topic_kind();
        let reliability_level = match qos.reliability.kind {
            ReliabilityQosPolicyKind::BestEffortReliabilityQos => ReliabilityKind::BestEffort,
            ReliabilityQosPolicyKind::ReliableReliabilityQos => ReliabilityKind::Reliable,
        };
        let push_mode = true;
        let data_max_sized_serialized = None;
        let writer = StatelessWriter::new(
            guid,
            topic_kind,
            reliability_level,
            push_mode,
            data_max_sized_serialized,
        );

        Self {
            writer: Mutex::new(WriterFlavor::Stateless(writer)),
            qos: Mutex::new(qos),
            topic: Mutex::new(Some(topic)),
            listener,
            status_mask,
        }
    }
}

pub trait AnyRtpsWriter: AsAny + Send + Sync {
    fn writer(&self) -> MutexGuard<WriterFlavor>;

    fn topic(&self) -> MutexGuard<Option<Arc<dyn AnyRtpsTopic>>>;

    fn qos(&self) -> MutexGuard<DataWriterQos>;
}

impl<T: DDSType + Sized> AnyRtpsWriter for RtpsDataWriter<T> {
    fn writer(&self) -> MutexGuard<WriterFlavor> {
        self.writer.lock().unwrap()
    }

    fn topic(&self) -> MutexGuard<Option<Arc<dyn AnyRtpsTopic>>> {
        self.topic.lock().unwrap()
    }

    fn qos(&self) -> MutexGuard<DataWriterQos> {
        self.qos.lock().unwrap()
    }
}

impl<T: DDSType + Sized> AsAny for RtpsDataWriter<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub type RtpsAnyDataWriterRef<'a> = MaybeValidRef<'a, Box<dyn AnyRtpsWriter>>;

impl<'a> RtpsAnyDataWriterRef<'a> {
    fn get(&self) -> ReturnCode<&Box<dyn AnyRtpsWriter>> {
        MaybeValid::get(self).ok_or(ReturnCodes::AlreadyDeleted)
    }

    pub fn get_as<U: DDSType>(&self) -> ReturnCode<&RtpsDataWriter<U>> {
        self.get()?
            .as_ref()
            .as_any()
            .downcast_ref()
            .ok_or(ReturnCodes::Error)
    }

    pub fn delete(&self) -> ReturnCode<()> {
        self.get()?.topic().take(); // Drop the topic
        MaybeValid::delete(self);
        Ok(())
    }

    pub fn write_w_timestamp<T: DDSType>(
        &self,
        data: T,
        _handle: Option<InstanceHandle>,
        _timestamp: Time,
    ) -> ReturnCode<()> {
        let writer = &mut self.get()?.writer();
        let kind = crate::types::ChangeKind::Alive;
        let inline_qos = None;
        let change = writer.new_change(
            kind,
            Some(data.serialize()),
            inline_qos,
            data.instance_handle(),
        );
        writer.writer_cache.add_change(change);

        Ok(())
    }

    pub fn get_qos(&self) -> ReturnCode<DataWriterQos> {
        Ok(self.get()?.qos().clone())
    }

    pub fn set_qos(&self, qos: Option<DataWriterQos>) -> ReturnCode<()> {
        let qos = qos.unwrap_or_default();
        qos.is_consistent()?;
        *self.get()?.qos() = qos;
        Ok(())
    }

    pub fn produce_messages(&self) -> Vec<behavior::endpoint_traits::DestinedMessages> {
        if let Some(rtps_writer) = self.get().ok() {
            match &mut *rtps_writer.writer() {
                WriterFlavor::Stateful(writer) => writer.produce_messages(),
                WriterFlavor::Stateless(writer) => writer.produce_messages(),
            }
        } else {
            vec![]
        }
    }
}
