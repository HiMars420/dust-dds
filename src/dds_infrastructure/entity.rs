use crate::types::{ReturnCode, InstanceHandle};
use crate::dds_infrastructure::status::StatusMask;

pub struct StatusCondition;

pub trait DomainEntity : Entity{}

pub trait Entity {
    type Qos;
    type Listener;

    fn set_qos(&self, qos_list: Self::Qos) -> ReturnCode<()>;

    fn get_qos(&self, qos_list: &mut Self::Qos) -> ReturnCode<()>;

    fn set_listener(&self, a_listener: Self::Listener, mask: StatusMask) -> ReturnCode<()>;

    fn get_listener(&self, ) -> Self::Listener;

    fn get_statuscondition(&self, ) -> StatusCondition;

    fn get_status_changes(&self, ) -> StatusMask;

    fn enable(&self, ) -> ReturnCode<()>;

    fn get_instance_handle(&self, ) -> ReturnCode<InstanceHandle>;
}