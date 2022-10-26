use dust_dds::{
    dds_type::{DdsSerde, DdsType},
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        qos::DataWriterQos,
        qos_policy::{ReliabilityQosPolicy, ReliabilityQosPolicyKind},
        status::StatusKind,
        time::Duration,
        wait_set::{Condition, WaitSet},
    },
};
use dust_dds_derive::{DdsSerde, DdsType};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, DdsType, DdsSerde)]
struct HelloWorldType {
    id: u8,
    msg: String,
}

fn main() {
    let domain_id = 0;
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, None, None, &[])
        .unwrap();

    let topic = participant
        .create_topic::<HelloWorldType>("HelloWorld", None, None, &[])
        .unwrap();

    let publisher = participant.create_publisher(None, None, &[]).unwrap();

    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::ReliableReliabilityQos,
            max_blocking_time: Duration::new(1, 0),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(&topic, Some(writer_qos), None, &[])
        .unwrap();
    let writer_cond = writer.get_statuscondition().unwrap();
    writer_cond
        .set_enabled_statuses(&[StatusKind::PublicationMatchedStatus])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(writer_cond))
        .unwrap();

    wait_set.wait(Duration::new(60, 0)).unwrap();

    let hello_world = HelloWorldType {
        id: 8,
        msg: "Hello world!".to_string(),
    };
    writer.write(&hello_world, None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(30, 0))
        .unwrap();
}
