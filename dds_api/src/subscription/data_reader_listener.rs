

// use super::data_reader::DataReader;

pub trait DataReaderListener {
    type DataType;
    // fn on_data_available(&self, the_reader: &dyn DataReader<Self::DataPIM>);
    // fn on_sample_rejected(
    //     &self,
    //     the_reader: &dyn DataReader<Self::DataPIM>,
    //     status: SampleRejectedStatus,
    // );
    // fn on_liveliness_changed(
    //     &self,
    //     the_reader: &dyn DataReader<Self::DataPIM>,
    //     status: LivelinessChangedStatus,
    // );
    // fn on_requested_deadline_missed(
    //     &self,
    //     the_reader: &dyn DataReader<Self::DataPIM>,
    //     status: RequestedDeadlineMissedStatus,
    // );
    // fn on_requested_incompatible_qos(
    //     &self,
    //     the_reader: &dyn DataReader<Self::DataPIM>,
    //     status: RequestedIncompatibleQosStatus,
    // );
    // fn on_subscription_matched(
    //     &self,
    //     the_reader: &dyn DataReader<Self::DataPIM>,
    //     status: SubscriptionMatchedStatus,
    // );
    // fn on_sample_lost(&self, the_reader: &dyn DataReader<Self::DataPIM>, status: SampleLostStatus);
}
