use std::sync::Weak;
use crate::types::{InstanceHandle, Data, Time, ReturnCode};

pub trait ProtocolEntity : Send + Sync {
    fn enable(&self) -> ReturnCode<()>;
    fn get_instance_handle(&self) -> InstanceHandle;
}

pub trait ProtocolEndpoint : ProtocolEntity {}

pub trait ProtocolParticipant : ProtocolEntity {
    fn create_publisher(&self) -> Weak<dyn ProtocolPublisher>;
    fn create_subscriber(&self) -> Weak<dyn ProtocolSubscriber>;
}

pub trait ProtocolSubscriber : ProtocolEntity {
    fn create_reader(&self) -> Weak<dyn ProtocolReader>;
}
pub trait ProtocolPublisher : ProtocolEntity {
    fn create_writer(&self) -> Weak<dyn ProtocolWriter>;
}

pub trait ProtocolWriter : ProtocolEndpoint {    
    fn write(&self, instance_handle: InstanceHandle, data: Data, timestamp: Time) -> ReturnCode<()>;

    fn dispose(&self, instance_handle: InstanceHandle, timestamp: Time) -> ReturnCode<()>;

    fn unregister(&self, instance_handle: InstanceHandle, timestamp: Time) -> ReturnCode<()>;

    fn register(&self, instance_handle: InstanceHandle, timestamp: Time) -> ReturnCode<Option<InstanceHandle>>;

    fn lookup_instance(&self, instance_handle: InstanceHandle) -> Option<InstanceHandle>;
}

pub trait ProtocolReader: ProtocolEndpoint {

}