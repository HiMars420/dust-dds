use dds::dcps_psm::{DURATION_ZERO, LENGTH_UNLIMITED};
use dds::dds_type::{DdsDeserialize, DdsSerialize, DdsType};
use dds::domain::domain_participant_factory::DdsDomainParticipantFactory;
use dds::infrastructure::qos::{DataReaderQos, DataWriterQos};
use dds::infrastructure::qos_policy::{
    HistoryQosPolicy, HistoryQosPolicyKind, ReliabilityQosPolicy, ReliabilityQosPolicyKind,
    ResourceLimitsQosPolicy,
};

use dds::return_type::{DdsError, DdsResult};

use dds::{
    dcps_psm::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
    domain::domain_participant_factory::DomainParticipantFactory,
};

#[derive(Debug, PartialEq)]
struct UserData(u8);

impl DdsType for UserData {
    fn type_name() -> &'static str {
        "UserData"
    }

    fn has_key() -> bool {
        false
    }
}

impl DdsSerialize for UserData {
    fn serialize<W: std::io::Write, E: dds::dds_type::Endianness>(
        &self,
        mut writer: W,
    ) -> DdsResult<()> {
        writer
            .write(&[self.0])
            .map(|_| ())
            .map_err(|e| DdsError::PreconditionNotMet(format!("{}", e)))
    }
}

impl<'de> DdsDeserialize<'de> for UserData {
    fn deserialize(buf: &mut &'de [u8]) -> DdsResult<Self> {
        Ok(UserData(buf[0]))
    }
}

#[test]
fn user_defined_write_read_auto_enable() {
    let domain_id = 8;
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant1 = participant_factory
        .create_participant(domain_id, None, None, 0)
        .unwrap();

    let participant2 = participant_factory
        .create_participant(domain_id, None, None, 0)
        .unwrap();

    let topic = participant1
        .create_topic::<UserData>("MyTopic", None, None, 0)
        .unwrap();

    let publisher = participant1.create_publisher(None, None, 0).unwrap();
    let writer = publisher.create_datawriter(&topic, None, None, 0).unwrap();

    let subscriber = participant2.create_subscriber(None, None, 0).unwrap();
    let reader = subscriber.create_datareader(&topic, None, None, 0).unwrap();

    //Wait for reader to be aware of the user writer
    while reader
        .get_subscription_matched_status()
        .unwrap()
        .total_count
        < 1
    {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    writer.write(&UserData(8), None).unwrap();

    let mut samples = reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE);
    while let Err(DdsError::NoData) = samples {
        std::thread::sleep(std::time::Duration::from_millis(50));
        samples = reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
    }

    assert_eq!(samples.unwrap()[0].data.as_ref().unwrap(), &UserData(8));
}

#[test]
fn data_reader_resource_limits() {
    let domain_id = 10;
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant1 = participant_factory
        .create_participant(domain_id, None, None, 0)
        .unwrap();
    let topic1 = participant1
        .create_topic::<UserData>("MyTopic", None, None, 0)
        .unwrap();

    let participant2 = participant_factory
        .create_participant(domain_id, None, None, 0)
        .unwrap();
    let topic2 = participant2
        .create_topic::<UserData>("MyTopic", None, None, 0)
        .unwrap();

    let publisher = participant1.create_publisher(None, None, 0).unwrap();
    let data_writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::BestEffortReliabilityQos,
            max_blocking_time: DURATION_ZERO,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAllHistoryQos,
            depth: LENGTH_UNLIMITED,
        },
        ..Default::default()
    };
    let data_writer = publisher
        .create_datawriter(&topic1, Some(data_writer_qos), None, 0)
        .unwrap();

    let subscriber = participant2.create_subscriber(None, None, 0).unwrap();
    let data_reader_qos = DataReaderQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAllHistoryQos,
            depth: LENGTH_UNLIMITED,
        },
        resource_limits: ResourceLimitsQosPolicy {
            max_samples: 2,
            max_instances: LENGTH_UNLIMITED,
            max_samples_per_instance: LENGTH_UNLIMITED,
        },
        ..Default::default()
    };
    let data_reader = subscriber
        .create_datareader(&topic2, Some(data_reader_qos), None, 0)
        .unwrap();

    //Wait for reader to be aware of the user writer
    while data_reader
        .get_subscription_matched_status()
        .unwrap()
        .total_count
        < 1
    {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    data_writer.write(&UserData(1), None).unwrap();
    data_writer.write(&UserData(2), None).unwrap();
    data_writer.write(&UserData(3), None).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));

    let mut samples = data_reader.read(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE);
    while let Err(DdsError::NoData) = samples {
        std::thread::sleep(std::time::Duration::from_millis(50));
        samples = data_reader.read(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE);
    }
    let samples = samples.unwrap();
    assert_eq!(samples.len(), 2);
}
