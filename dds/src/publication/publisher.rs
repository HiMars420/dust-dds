use std::any::Any;
use std::sync::{Arc, Weak, Mutex};

use crate::types::{StatusKind, ReturnCode, Duration, InstanceHandle, StatusMask, ReturnCodes, DDSType};
use crate::domain::domain_participant::{DomainParticipant, DomainParticipantImpl};
use crate::topic::topic::Topic;
use crate::topic::qos::TopicQos;
use crate::publication::data_writer_listener::DataWriterListener;
use crate::publication::data_writer::{DataWriter, DataWriterImpl, AnyDataWriter};
use crate::publication::data_writer::qos::DataWriterQos;
use crate::infrastructure::entity::{Entity, StatusCondition};
use crate::infrastructure::entity::DomainEntity;
use crate::publication::publisher_listener::PublisherListener;
use qos::PublisherQos;

pub mod qos {
    use crate::infrastructure::qos_policy::{
        PresentationQosPolicy,
        PartitionQosPolicy,
        GroupDataQosPolicy,
        EntityFactoryQosPolicy,
    };

    #[derive(Debug, Default, PartialEq, Clone)]
    pub struct PublisherQos {
        pub presentation: PresentationQosPolicy,
        pub partition: PartitionQosPolicy,
        pub group_data: GroupDataQosPolicy,
        pub entity_factory: EntityFactoryQosPolicy,
    }
}

/// The Publisher acts on the behalf of one or several DataWriter objects that belong to it. When it is informed of a change to the
/// data associated with one of its DataWriter objects, it decides when it is appropriate to actually send the data-update message.
/// In making this decision, it considers any extra information that goes with the data (timestamp, writer, etc.) as well as the QoS
/// of the Publisher and the DataWriter.
/// All operations except for the base-class operations set_qos, get_qos, set_listener, get_listener, enable, get_statuscondition,
/// create_datawriter, and delete_datawriter may return the value NOT_ENABLED.
pub struct Publisher(pub(crate) Weak<PublisherImpl>);

impl Publisher {
    /// This operation creates a DataWriter. The returned DataWriter will be attached and belongs to the Publisher.
    /// The DataWriter returned by the create_datawriter operation will in fact be a derived class, specific to the data-type associated
    /// with the Topic. As described in 2.2.2.3.7, for each application-defined type “Foo” there is an implied, auto-generated class
    /// FooDataWriter that extends DataWriter and contains the operations to write data of type “Foo.”
    /// In case of failure, the operation will return a ‘nil’ value (as specified by the platform).
    /// Note that a common application pattern to construct the QoS for the DataWriter is to:
    /// • Retrieve the QoS policies on the associated Topic by means of the get_qos operation on the Topic.
    /// • Retrieve the default DataWriter qos by means of the get_default_datawriter_qos operation on the Publisher.
    /// • Combine those two QoS policies and selectively modify policies as desired.
    /// • Use the resulting QoS policies to construct the DataWriter.
    /// The special value DATAWRITER_QOS_DEFAULT can be used to indicate that the DataWriter should be created with the
    /// default DataWriter QoS set in the factory. The use of this value is equivalent to the application obtaining the default
    /// DataWriter QoS by means of the operation get_default_datawriter_qos (2.2.2.4.1.15) and using the resulting QoS to create
    /// the DataWriter.
    /// The special value DATAWRITER_QOS_USE_TOPIC_QOS can be used to indicate that the DataWriter should be created
    /// with a combination of the default DataWriter QoS and the Topic QoS. The use of this value is equivalent to the application
    /// obtaining the default DataWriter QoS and the Topic QoS (by means of the operation Topic::get_qos) and then combining these
    /// two QoS using the operation copy_from_topic_qos whereby any policy that is set on the Topic QoS “overrides” the
    /// corresponding policy on the default QoS. The resulting QoS is then applied to the creation of the DataWriter.
    /// The Topic passed to this operation must have been created from the same DomainParticipant that was used to create this
    /// Publisher. If the Topic was created from a different DomainParticipant, the operation will fail and return a nil result.
    pub fn create_datawriter<T: DDSType+Any+Sync+Send>(
        &self,
        a_topic: Topic,
        qos: DataWriterQos,
        a_listener: Box<dyn DataWriterListener<T>>,
        mask: StatusMask
    ) -> Option<DataWriter<T>> {
        PublisherImpl::create_datawriter(&self.0, a_topic, qos, a_listener, mask)
    }

    /// This operation deletes a DataWriter that belongs to the Publisher.
    /// The delete_datawriter operation must be called on the same Publisher object used to create the DataWriter. If
    /// delete_datawriter is called on a different Publisher, the operation will have no effect and it will return
    /// PRECONDITION_NOT_MET.
    /// The deletion of the DataWriter will automatically unregister all instances. Depending on the settings of the
    /// WRITER_DATA_LIFECYCLE QosPolicy, the deletion of the DataWriter may also dispose all instances. Refer to 2.2.3.21 for
    /// details.
    /// Possible error codes returned in addition to the standard ones: PRECONDITION_NOT_MET.
    pub fn delete_datawriter<T: DDSType+Any+Send+Sync>(
        &self,
        a_datawriter: &DataWriter<T>
    ) -> ReturnCode<()> {
        PublisherImpl::delete_datawriter(&self.0, &a_datawriter)
    }

    /// This operation retrieves a previously created DataWriter belonging to the Publisher that is attached to a Topic with a matching
    /// topic_name. If no such DataWriter exists, the operation will return ’nil.’
    /// If multiple DataWriter attached to the Publisher satisfy this condition, then the operation will return one of them. It is not
    /// specified which one.
    pub fn lookup_datawriter<T: DDSType+Any+Send+Sync>(
        &self,
        topic_name: String,
    ) -> Option<DataWriter<T>> {
        PublisherImpl::lookup_datawriter(&self.0, topic_name)
    }

    /// This operation indicates to the Service that the application is about to make multiple modifications using DataWriter objects
    /// belonging to the Publisher.
    /// It is a hint to the Service so it can optimize its performance by e.g., holding the dissemination of the modifications and then
    /// batching them.
    /// It is not required that the Service use this hint in any way.
    /// The use of this operation must be matched by a corresponding call to resume_publications indicating that the set of
    /// modifications has completed. If the Publisher is deleted before resume_publications is called, any suspended updates yet to
    /// be published will be discarded.
    pub fn suspend_publications(&self) -> ReturnCode<()> {
        PublisherImpl::suspend_publications(&self.0)
    }

    /// This operation indicates to the Service that the application has completed the multiple changes initiated by the previous
    /// suspend_publications. This is a hint to the Service that can be used by a Service implementation to e.g., batch all the
    /// modifications made since the suspend_publications.
    /// The call to resume_publications must match a previous call to suspend_publications. Otherwise the operation will return the
    /// error PRECONDITION_NOT_MET.
    /// Possible error codes returned in addition to the standard ones: PRECONDITION_NOT_MET.
    pub fn resume_publications(&self) -> ReturnCode<()> {
        PublisherImpl::resume_publications(&self.0)
    }

    /// This operation requests that the application will begin a ‘coherent set’ of modifications using DataWriter objects attached to
    /// the Publisher. The ‘coherent set’ will be completed by a matching call to end_coherent_changes.
    /// A ‘coherent set’ is a set of modifications that must be propagated in such a way that they are interpreted at the receivers’ side
    /// as a consistent set of modifications; that is, the receiver will only be able to access the data after all the modifications in the set
    /// are available at the receiver end. This does not imply that the middleware has to encapsulate all the modifications in a single message; it only implies that the
    /// receiving applications will behave as if this was the case.
    /// A connectivity change may occur in the middle of a set of coherent changes; for example, the set of partitions used by the
    /// Publisher or one of its Subscribers may change, a late-joining DataReader may appear on the network, or a communication
    /// failure may occur. In the event that such a change prevents an entity from receiving the entire set of coherent changes, that
    /// entity must behave as if it had received none of the set.
    /// These calls can be nested. In that case, the coherent set terminates only with the last call to end_coherent_ changes.
    /// The support for ‘coherent changes’ enables a publishing application to change the value of several data-instances that could
    /// belong to the same or different topics and have those changes be seen ‘atomically’ by the readers. This is useful in cases where
    /// the values are inter-related (for example, if there are two data-instances representing the ‘altitude’ and ‘velocity vector’ of the
    /// same aircraft and both are changed, it may be useful to communicate those values in a way the reader can see both together;
    /// otherwise, it may e.g., erroneously interpret that the aircraft is on a collision course).
    pub fn begin_coherent_changes(&self) -> ReturnCode<()> {
        PublisherImpl::begin_coherent_changes(&self.0)
    }

    /// This operation terminates the ‘coherent set’ initiated by the matching call to begin_coherent_ changes. If there is no matching
    /// call to begin_coherent_ changes, the operation will return the error PRECONDITION_NOT_MET.
    /// Possible error codes returned in addition to the standard ones: PRECONDITION_NOT_MET
    pub fn end_coherent_changes(&self) -> ReturnCode<()> {
        PublisherImpl::end_coherent_changes(&self.0)
    }

    /// This operation blocks the calling thread until either all data written by the reliable DataWriter entities is acknowledged by all
    /// matched reliable DataReader entities, or else the duration specified by the max_wait parameter elapses, whichever happens
    /// first. A return value of OK indicates that all the samples written have been acknowledged by all reliable matched data readers;
    /// a return value of TIMEOUT indicates that max_wait elapsed before all the data was acknowledged.
    pub fn wait_for_acknowledgments(
        &self,
        max_wait: Duration
    ) -> ReturnCode<()> {
        PublisherImpl::wait_for_acknowledgments(&self.0, max_wait)
    }

    /// This operation returns the DomainParticipant to which the Publisher belongs.
    pub fn get_participant(&self,) -> DomainParticipant {
        PublisherImpl::get_participant(&self.0, )
    }

    /// This operation deletes all the entities that were created by means of the “create” operations on the Publisher. That is, it deletes
    /// all contained DataWriter objects.
    /// The operation will return PRECONDITION_NOT_MET if the any of the contained entities is in a state where it cannot be
    /// deleted.
    /// Once delete_contained_entities returns successfully, the application may delete the Publisher knowing that it has no
    /// contained DataWriter objects
    pub fn delete_contained_entities(&self) -> ReturnCode<()> {
        PublisherImpl::delete_contained_entities(&self.0)
    }

    /// This operation sets a default value of the DataWriter QoS policies which will be used for newly created DataWriter entities in
    /// the case where the QoS policies are defaulted in the create_datawriter operation.
    /// This operation will check that the resulting policies are self consistent; if they are not, the operation will have no effect and
    /// return INCONSISTENT_POLICY.
    /// The special value DATAWRITER_QOS_DEFAULT may be passed to this operation to indicate that the default QoS should be
    /// reset back to the initial values the factory would use, that is the values that would be used if the set_default_datawriter_qos
    /// operation had never been called.
    pub fn set_default_datawriter_qos(
        &self,
        qos_list: DataWriterQos,
    ) -> ReturnCode<()> {
        PublisherImpl::set_default_datawriter_qos(&self.0, qos_list)
    }

    /// This operation retrieves the default value of the DataWriter QoS, that is, the QoS policies which will be used for newly created
    /// DataWriter entities in the case where the QoS policies are defaulted in the create_datawriter operation.
    /// The values retrieved by get_default_datawriter_qos will match the set of values specified on the last successful call to
    /// set_default_datawriter_qos, or else, if the call was never made, the default values listed in the QoS table in 2.2.3, Supported
    /// QoS.
    pub fn get_default_datawriter_qos (
        &self,
        qos_list: &mut DataWriterQos,
    ) -> ReturnCode<()> {
        PublisherImpl::get_default_datawriter_qos(&self.0, qos_list)
    }

    /// This operation copies the policies in the a_topic_qos to the corresponding policies in the a_datawriter_qos (replacing values
    /// in the a_datawriter_qos, if present).
    /// This is a “convenience” operation most useful in combination with the operations get_default_datawriter_qos and
    /// Topic::get_qos. The operation copy_from_topic_qos can be used to merge the DataWriter default QoS policies with the
    /// corresponding ones on the Topic. The resulting QoS can then be used to create a new DataWriter, or set its QoS.
    /// This operation does not check the resulting a_datawriter_qos for consistency. This is because the ‘merged’ a_datawriter_qos
    /// may not be the final one, as the application can still modify some policies prior to applying the policies to the DataWriter.
    pub fn copy_from_topic_qos(
        &self,
        a_datawriter_qos: &mut DataWriterQos,
        a_topic_qos: &TopicQos,
    ) -> ReturnCode<()> {
        PublisherImpl::copy_from_topic_qos(&self.0, a_datawriter_qos, a_topic_qos)
    }
}

impl Entity for Publisher{
    type Qos = PublisherQos;
    type Listener = Box<dyn PublisherListener>;

    fn set_qos(&self, qos_list: Self::Qos) -> ReturnCode<()> {
        PublisherImpl::set_qos(&self.0, qos_list)
    }

    fn get_qos(&self, qos_list: &mut Self::Qos) -> ReturnCode<()> {
        PublisherImpl::get_qos(&self.0, qos_list)
    }

    fn set_listener(&self, a_listener: Self::Listener, mask: &[StatusKind]) -> ReturnCode<()> {
        PublisherImpl::set_listener(&self.0, a_listener, mask)
    }

    fn get_listener(&self, ) -> Self::Listener {
        PublisherImpl::get_listener(&self.0)
    }

    fn get_statuscondition(&self, ) -> crate::infrastructure::entity::StatusCondition {
        PublisherImpl::get_statuscondition(&self.0)
    }

    fn get_status_changes(&self, ) -> StatusKind {
        PublisherImpl::get_status_changes(&self.0)
    }

    fn enable(&self, ) -> ReturnCode<()> {
        PublisherImpl::enable(&self.0)
    }

    fn get_instance_handle(&self, ) -> crate::types::InstanceHandle {
        PublisherImpl::get_instance_handle(&self.0)
    }
}

impl DomainEntity for Publisher{}

// impl Drop for Publisher {
//     fn drop(&mut self) {
//         let parent_participant = self.get_participant();
//         parent_participant.delete_publisher(self);
//     }
// }

pub struct PublisherImpl{
    parent_participant: Weak<DomainParticipantImpl>,
    datawriter_list: Mutex<Vec<AnyDataWriter>>,
    default_datawriter_qos: Mutex<DataWriterQos>,
}

impl PublisherImpl {
    pub(crate) fn create_datawriter<T: DDSType+Any+Send+Sync>(
        this: &Weak<PublisherImpl>,
        _a_topic: Topic,
        _qos: DataWriterQos,
        _a_listener: Box<dyn DataWriterListener<T>>,
        _mask: StatusMask,
    ) -> Option<DataWriter<T>> {
        let datawriter_impl = Arc::new(DataWriterImpl::new(this.clone()));
        let datawriter = DataWriter(Arc::downgrade(&datawriter_impl));        

        this.upgrade()?.datawriter_list.lock().ok()?.push(AnyDataWriter(datawriter_impl));

        Some(datawriter)
    }

    pub(crate) fn delete_datawriter<T: DDSType+Any+Send+Sync>(
        this: &Weak<PublisherImpl>,
        a_datawriter: &DataWriter<T>
    ) -> ReturnCode<()> {
        let publisher = this.upgrade().unwrap();
        let mut datawriter_list = publisher.datawriter_list.lock().unwrap();
        let index = datawriter_list.iter().position(|x| 
            match x.get::<T>() {
                Some(dw) => dw.0.ptr_eq(&a_datawriter.0),
                None => false,
        });
        
        if let Some(index) = index{
            datawriter_list.swap_remove(index);
            Ok(())
        } else {
            Err(ReturnCodes::PreconditionNotMet)
        }
    }

    pub(crate) fn lookup_datawriter<T: DDSType+Any+Send+Sync>(
        _this: &Weak<PublisherImpl>,
        _topic_name: String,
    ) -> Option<DataWriter<T>> {
        todo!()
    }

    pub(crate) fn suspend_publications(_this: &Weak<PublisherImpl>) -> ReturnCode<()> {
        todo!()
    }

    pub(crate) fn resume_publications(_this: &Weak<PublisherImpl>) -> ReturnCode<()> {
        todo!()
    }

    pub(crate) fn begin_coherent_changes(_this: &Weak<PublisherImpl>) -> ReturnCode<()> {
        todo!()
    }

    pub(crate) fn end_coherent_changes(_this: &Weak<PublisherImpl>) -> ReturnCode<()> {
        todo!()
    }

    pub(crate) fn wait_for_acknowledgments(
        _this: &Weak<PublisherImpl>,
        _max_wait: Duration
    ) -> ReturnCode<()> {
        todo!()
    }

    pub(crate) fn get_participant(this: &Weak<PublisherImpl>) -> DomainParticipant {
        DomainParticipant(this.upgrade().unwrap().parent_participant.upgrade().unwrap())
    }

    pub(crate) fn delete_contained_entities(_this: &Weak<PublisherImpl>) -> ReturnCode<()> {
        todo!()
    }

    pub(crate) fn set_default_datawriter_qos(
        this: &Weak<PublisherImpl>,
        qos: DataWriterQos,
    ) -> ReturnCode<()> {
        let publisher = PublisherImpl::upgrade_publisher(this)?;

        if qos.is_consistent() {
            *publisher.default_datawriter_qos.lock().unwrap() = qos;
        } else {
            return Err(ReturnCodes::InconsistentPolicy);
        }
        
        Ok(())
    }

    pub(crate) fn get_default_datawriter_qos (
        this: &Weak<PublisherImpl>,
        qos: &mut DataWriterQos,
    ) -> ReturnCode<()> {
        let publisher = PublisherImpl::upgrade_publisher(this)?;

        qos.clone_from(&publisher.default_datawriter_qos.lock().unwrap());
        Ok(())
    }

    pub(crate) fn copy_from_topic_qos(
        _this: &Weak<PublisherImpl>,
        _a_datawriter_qos: &mut DataWriterQos,
        _a_topic_qos: &TopicQos,
    ) -> ReturnCode<()> {
        todo!()
    }

    ///////////////// Entity trait methods
    pub(crate) fn set_qos(_this: &Weak<PublisherImpl>, _qos_list: PublisherQos) -> ReturnCode<()> {
        todo!()
    }

    pub(crate) fn get_qos(_this: &Weak<PublisherImpl>, _qos_list: &mut PublisherQos) -> ReturnCode<()> {
        todo!()
    }

    pub(crate) fn set_listener(_this: &Weak<PublisherImpl>, _a_listener: Box<dyn PublisherListener>, _mask: &[StatusKind]) -> ReturnCode<()> {
        todo!()
    }

    pub(crate) fn get_listener(_this: &Weak<PublisherImpl>, ) -> Box<dyn PublisherListener> {
        todo!()
    }

    pub(crate) fn get_statuscondition(_this: &Weak<PublisherImpl>, ) -> StatusCondition {
        todo!()
    }

    pub(crate) fn get_status_changes(_this: &Weak<PublisherImpl>, ) -> StatusKind {
        todo!()
    }

    pub(crate) fn enable(_this: &Weak<PublisherImpl>, ) -> ReturnCode<()> {
        todo!()
    }

    pub(crate) fn get_instance_handle(_this: &Weak<PublisherImpl>, ) -> InstanceHandle {
        todo!()
    }

    //////////////// From here on are the functions that do not belong to the standard API
    pub(crate) fn new(parent_participant: Weak<DomainParticipantImpl>
    ) -> Self {
        Self{
            parent_participant,
            datawriter_list: Mutex::new(Vec::new()),
            default_datawriter_qos: Mutex::new(DataWriterQos::default()),
        }
    }

    fn upgrade_publisher(this: &Weak<PublisherImpl>) -> ReturnCode<Arc<PublisherImpl>> {
        this.upgrade().ok_or(ReturnCodes::AlreadyDeleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::listener::NoListener;
    use crate::infrastructure::qos_policy::ReliabilityQosPolicyKind;
    #[derive(Debug)]
    struct  Foo {
        value: bool
    }

    impl DDSType for Foo {
        fn key(&self) -> InstanceHandle {
        todo!()
    }

        fn data(&self) -> Vec<u8> {
        todo!()
    }
    }

    #[test]
    fn create_delete_datawriter() {
        let publisher_impl = Arc::new(PublisherImpl::new(Weak::new()));
        let topic = Topic(Weak::new());
        
        assert_eq!(publisher_impl.datawriter_list.lock().unwrap().len(), 0);
        let datawriter = PublisherImpl::create_datawriter::<Foo>(&Arc::downgrade(&publisher_impl),topic, DataWriterQos::default(), Box::new(NoListener), 0).unwrap();
        assert_eq!(publisher_impl.datawriter_list.lock().unwrap().len(), 1);
        
        PublisherImpl::delete_datawriter(&Arc::downgrade(&publisher_impl), &datawriter).unwrap();
        assert_eq!(publisher_impl.datawriter_list.lock().unwrap().len(), 0);
    }

    #[test]
    fn set_and_get_default_datawriter_qos() {
        let publisher_impl = Arc::new(PublisherImpl::new(Weak::new()));
        let publisher = Arc::downgrade(&publisher_impl);

        let mut datawriter_qos = DataWriterQos::default();
        datawriter_qos.user_data.value = vec![1,2,3,4];
        datawriter_qos.reliability.kind = ReliabilityQosPolicyKind::ReliableReliabilityQos;

        PublisherImpl::set_default_datawriter_qos(&publisher, datawriter_qos.clone()).unwrap();
        assert_eq!(*publisher_impl.default_datawriter_qos.lock().unwrap(), datawriter_qos);

        let mut read_datawriter_qos = DataWriterQos::default();
        PublisherImpl::get_default_datawriter_qos(&publisher, &mut read_datawriter_qos).unwrap();

        assert_eq!(read_datawriter_qos, datawriter_qos);
    }

    #[test]
    fn inconsistent_datawriter_qos() {
        let publisher_impl = Arc::new(PublisherImpl::new(Weak::new()));
        let publisher = Arc::downgrade(&publisher_impl);

        let mut datawriter_qos = DataWriterQos::default();
        datawriter_qos.resource_limits.max_samples = 5;
        datawriter_qos.resource_limits.max_samples_per_instance = 15;

        let error = PublisherImpl::set_default_datawriter_qos(&publisher, datawriter_qos.clone());
        assert_eq!(error, Err(ReturnCodes::InconsistentPolicy));

        assert_eq!(*publisher_impl.default_datawriter_qos.lock().unwrap(), DataWriterQos::default());
    }
}