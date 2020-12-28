use crate::dds::infrastructure::status::InconsistentTopicStatus;
use crate::dds::infrastructure::status::{
    LivelinessChangedStatus, LivelinessLostStatus, OfferedDeadlineMissedStatus,
    OfferedIncompatibleQosStatus, PublicationMatchedStatus, RequestedDeadlineMissedStatus,
    RequestedIncompatibleQosStatus, SampleLostStatus, SampleRejectedStatus,
    SubscriptionMatchedStatus,
};
use crate::dds::publication::data_writer::AnyDataWriter;
use crate::dds::subscription::data_reader::AnyDataReader;
use crate::dds::subscription::subscriber::Subscriber;
use crate::dds::topic::topic_description::TopicDescription;

/// The purpose of the DomainParticipantListener is to be the listener of last resort that is notified of all status changes not
/// captured by more specific listeners attached to the DomainEntity objects. When a relevant status change occurs, the DCPS
/// Service will first attempt to notify the listener attached to the concerned DomainEntity if one is installed. Otherwise, the
/// DCPS Service will notify the Listener attached to the DomainParticipant.
pub trait DomainParticipantListener {
    fn on_inconsistent_topic(
        &self,
        the_topic: dyn TopicDescription,
        status: InconsistentTopicStatus,
    );
    fn on_data_on_readers(&self, the_subscriber: Subscriber);
    fn on_data_available(&self, the_reader: dyn AnyDataReader);
    fn on_sample_rejected(&self, the_reader: dyn AnyDataReader, status: SampleRejectedStatus);
    fn on_liveliness_changed(&self, the_reader: dyn AnyDataReader, status: LivelinessChangedStatus);
    fn on_requested_deadline_missed(
        &self,
        the_reader: dyn AnyDataReader,
        status: RequestedDeadlineMissedStatus,
    );
    fn on_requested_incompatible_qos(
        &self,
        the_reader: dyn AnyDataReader,
        status: RequestedIncompatibleQosStatus,
    );
    fn on_subscription_matched(
        &self,
        the_reader: dyn AnyDataReader,
        status: SubscriptionMatchedStatus,
    );
    fn on_sample_lost(&self, the_reader: dyn AnyDataReader, status: SampleLostStatus);
    fn on_liveliness_lost(&self, the_writer: dyn AnyDataWriter, status: LivelinessLostStatus);
    fn on_offered_deadline_missed(
        &self,
        the_writer: dyn AnyDataWriter,
        status: OfferedDeadlineMissedStatus,
    );
    fn on_offered_incompatible_qos(
        &self,
        the_writer: dyn AnyDataWriter,
        status: OfferedIncompatibleQosStatus,
    );
    fn on_publication_matched(
        &self,
        the_writer: dyn AnyDataWriter,
        status: PublicationMatchedStatus,
    );
}
