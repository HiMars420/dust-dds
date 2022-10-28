use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{qos::Qos, status::NO_STATUS},
};

#[test]
fn get_subscriber_parent_participant() {
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(0, Qos::Default, None, NO_STATUS)
        .unwrap();

    let subscriber = participant
        .create_subscriber(Qos::Default, None, NO_STATUS)
        .unwrap();

    let subscriber_parent_participant = subscriber.get_participant().unwrap();

    assert_eq!(participant, subscriber_parent_participant);
}
