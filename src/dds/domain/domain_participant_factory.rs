use std::any::Any;
use std::sync::{Arc, Weak, Mutex};

use crate::dds::types::{DomainId, StatusMask, ReturnCode};
use crate::dds::domain::domain_participant::DomainParticipant;
use crate::dds::domain::domain_participant_impl::DomainParticipantImpl;
use crate::dds::domain::domain_participant_listener::DomainParticipantListener;
use crate::dds::infrastructure::qos_policy::QosPolicy;
use crate::dds::domain::domain_participant_listener::NoListener;

pub mod qos {
    use crate::dds::infrastructure::qos_policy::EntityFactoryQosPolicy;

    #[derive(Default)]
    pub struct DomainParticipantFactoryQos {
        entity_factory: EntityFactoryQosPolicy,
    }
}

pub struct DomainParticipantFactory{
    participant_list: Mutex<Vec<Weak<DomainParticipantImpl>>>,
    // default_qos_list: 
}

lazy_static! {
    pub static ref DOMAIN_PARTICIPANT_FACTORY: DomainParticipantFactory = DomainParticipantFactory{
        participant_list: Mutex::new(Vec::new())
    };
}

pub const PARTICIPANT_QOS_DEFAULT : &[&dyn QosPolicy] = &[];

impl DomainParticipantFactory {
    /// This operation creates a new DomainParticipant object. The DomainParticipant signifies that the calling application intends
    /// to join the Domain identified by the domain_id argument.
    /// If the specified QoS policies are not consistent, the operation will fail and no DomainParticipant will be created.
    /// The special value PARTICIPANT_QOS_DEFAULT can be used to indicate that the DomainParticipant should be created
    /// with the default DomainParticipant QoS set in the factory. The use of this value is equivalent to the application obtaining the
    /// default DomainParticipant QoS by means of the operation get_default_participant_qos (2.2.2.2.2.6) and using the resulting
    /// QoS to create the DomainParticipant.
    /// In case of failure, the operation will return a ‘nil’ value (as specified by the platform).
    pub fn create_participant (
        &self,
        domain_id: DomainId,
        qos_list: &[&dyn QosPolicy],
        a_listener: impl DomainParticipantListener,
        mask: StatusMask,
    ) ->  Option<DomainParticipant> {

        if qos_list.is_empty() {
            println!("TODO: Use default QoS")
        }
        
        if !Any::is::<NoListener>(&a_listener) {
            println!("TODO: Use the real listener")
        }

        let new_participant = Arc::new(DomainParticipantImpl::new(domain_id, qos_list, a_listener, mask));
        self.participant_list.lock().unwrap().push(Arc::downgrade(&new_participant));

        Some(DomainParticipant(new_participant))
    }

    /// This operation deletes an existing DomainParticipant. This operation can only be invoked if all domain entities belonging to
    /// the participant have already been deleted. Otherwise the error PRECONDITION_NOT_MET is returned.
    /// Possible error codes returned in addition to the standard ones: PRECONDITION_NOT_MET.
    pub fn delete_participant(
        _a_participant: DomainParticipant,
    ) -> ReturnCode {
        todo!()
    }

    /// This operation returns the DomainParticipantFactory singleton. The operation is idempotent, that is, it can be called multiple
    /// times without side-effects and it will return the same DomainParticipantFactory instance.
    /// The get_instance operation is a static operation implemented using the syntax of the native language and can therefore not be
    /// expressed in the IDL PSM.
    /// The pre-defined value TheParticipantFactory can also be used as an alias for the singleton factory returned by the operation
    /// get_instance.
    pub fn get_instance() -> &'static DOMAIN_PARTICIPANT_FACTORY {
        &DOMAIN_PARTICIPANT_FACTORY
    }

    /// This operation retrieves a previously created DomainParticipant belonging to specified domain_id. If no such
    /// DomainParticipant exists, the operation will return a ‘nil’ value.
    /// If multiple DomainParticipant entities belonging to that domain_id exist, then the operation will return one of them. It is not
    /// specified which one.
    pub fn lookup_participant(
        _domain_id: DomainId,
    ) -> DomainParticipant {
        todo!()
    }

    /// This operation sets a default value of the DomainParticipant QoS policies which will be used for newly created
    /// DomainParticipant entities in the case where the QoS policies are defaulted in the create_participant operation.
    /// This operation will check that the resulting policies are self consistent; if they are not, the operation will have no effect and
    /// return INCONSISTENT_POLICY.
    pub fn set_default_participant_qos(
        _qos_list: &[&dyn QosPolicy],
    ) -> ReturnCode {
        todo!()
    }

    /// This operation retrieves the default value of the DomainParticipant QoS, that is, the QoS policies which will be used for
    /// newly created DomainParticipant entities in the case where the QoS policies are defaulted in the create_participant
    /// operation.
    /// The values retrieved get_default_participant_qos will match the set of values specified on the last successful call to
    /// set_default_participant_qos, or else, if the call was never made, the default values listed in the QoS table in 2.2.3,
    /// Supported QoS.
    pub fn get_default_participant_qos(
        _qos_list: &mut [&dyn QosPolicy],
    ) -> ReturnCode {
        todo!()
    }

    /// This operation sets the value of the DomainParticipantFactory QoS policies. These policies control the behavior of the object
    /// a factory for entities.
    /// Note that despite having QoS, the DomainParticipantFactory is not an Entity.
    /// This operation will check that the resulting policies are self consistent; if they are not, the operation will have no effect and
    /// return INCONSISTENT_POLICY.
    pub fn set_qos(
        _qos_list: &[&dyn QosPolicy],
    ) -> ReturnCode {
        todo!()
    }

    /// This operation returns the value of the DomainParticipantFactory QoS policies.
    pub fn get_qos(
        _qos_list: &mut [&dyn QosPolicy],
    ) -> ReturnCode {
        todo!()
    }   
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_participants() {
        let domain_participant_factory = DomainParticipantFactory::get_instance();

        assert_eq!(domain_participant_factory.participant_list.lock().unwrap().len(), 0);
        let participant = domain_participant_factory.create_participant(1, PARTICIPANT_QOS_DEFAULT,NoListener, 0).unwrap();

        assert_eq!(domain_participant_factory.participant_list.lock().unwrap().len(), 1);
        assert!(std::ptr::eq(
            participant.0.as_ref(),
            domain_participant_factory.participant_list.lock().unwrap()[0].upgrade().unwrap().as_ref())
        );

        let participant2 = domain_participant_factory.create_participant(2, PARTICIPANT_QOS_DEFAULT,NoListener, 0).unwrap();
        assert_eq!(domain_participant_factory.participant_list.lock().unwrap().len(), 2);
        assert!(std::ptr::eq(
            participant.0.as_ref(),
            domain_participant_factory.participant_list.lock().unwrap()[0].upgrade().unwrap().as_ref())
        );

        assert!(std::ptr::eq(
            participant2.0.as_ref(),
            domain_participant_factory.participant_list.lock().unwrap()[1].upgrade().unwrap().as_ref())
        );

    }
}