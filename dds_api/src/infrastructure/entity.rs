use crate::{
    dcps_psm::{InstanceHandle, StatusMask},
    return_type::DDSResult,
};

pub struct StatusCondition;

pub trait DomainEntity {}

pub trait Entity {
    type Qos;
    type Listener;

    fn set_qos(&self, qos: Option<Self::Qos>) -> DDSResult<()>;

    fn get_qos(&self) -> DDSResult<Self::Qos>;

    fn set_listener(&self, a_listener: Option<Self::Listener>, mask: StatusMask) -> DDSResult<()>;

    fn get_listener(&self) -> DDSResult<Option<Self::Listener>>;

    fn get_statuscondition(&self) -> StatusCondition;

    fn get_status_changes(&self) -> StatusMask;

    fn enable(&self) -> DDSResult<()>;

    fn get_instance_handle(&self) -> DDSResult<InstanceHandle>;
}
