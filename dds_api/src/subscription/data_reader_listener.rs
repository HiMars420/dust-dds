use crate::{dcps_psm::{LivelinessChangedStatus, RequestedDeadlineMissedStatus, RequestedIncompatibleQosStatus, SampleLostStatus, SampleRejectedStatus, SubscriptionMatchedStatus}, dds_type::DDSType, infrastructure::listener::Listener};

use super::data_reader::DataReader;

pub trait DataReaderListener<T: DDSType>: Listener {
    fn on_data_available(&self, the_reader: &dyn DataReader<T>);
    fn on_sample_rejected(&self, the_reader: &dyn DataReader<T>, status: SampleRejectedStatus);
    fn on_liveliness_changed(
        &self,
        the_reader: &dyn DataReader<T>,
        status: LivelinessChangedStatus,
    );
    fn on_requested_deadline_missed(
        &self,
        the_reader: &dyn DataReader<T>,
        status: RequestedDeadlineMissedStatus,
    );
    fn on_requested_incompatible_qos(
        &self,
        the_reader: &dyn DataReader<T>,
        status: RequestedIncompatibleQosStatus,
    );
    fn on_subscription_matched(
        &self,
        the_reader: &dyn DataReader<T>,
        status: SubscriptionMatchedStatus,
    );
    fn on_sample_lost(&self, the_reader: &dyn DataReader<T>, status: SampleLostStatus);
}
