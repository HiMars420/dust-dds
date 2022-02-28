use rust_dds::{
    communication::Communication,
    domain::domain_participant::DomainParticipant,
    domain_participant_factory::{port_user_unicast, DomainParticipantFactory, get_unicast_socket},
    infrastructure::{
        qos::{DataReaderQos, DomainParticipantQos},
        qos_policy::ReliabilityQosPolicyKind,
    },
    publication::{data_writer::DataWriter, publisher::Publisher},
    subscription::{data_reader::DataReader, subscriber::Subscriber},
    types::Time,
    udp_transport::UdpTransport,
};
use rust_dds_rtps_implementation::{
    dds_type::{DdsDeserialize, DdsSerialize, DdsType},
    rtps_impl::{
        rtps_reader_proxy_impl::RtpsReaderProxyAttributesImpl,
        rtps_writer_proxy_impl::RtpsWriterProxyImpl,
    },
};
use rust_rtps_pim::behavior::writer::reader_proxy::RtpsReaderProxyConstructor;
use rust_rtps_pim::{
    behavior::{
        reader::{
            reader::RtpsReaderAttributes, stateful_reader::RtpsStatefulReaderOperations,
            writer_proxy::RtpsWriterProxyConstructor,
        },
        writer::{stateful_writer::RtpsStatefulWriterOperations, writer::RtpsWriterAttributes},
    },
    structure::{
        entity::RtpsEntityAttributes,
        types::{GuidPrefix, LOCATOR_KIND_UDPv4, Locator, PROTOCOLVERSION, VENDOR_ID_S2E},
    },
};

struct MyType {}

impl DdsType for MyType {
    fn type_name() -> &'static str {
        "MyType"
    }

    fn has_key() -> bool {
        false
    }
}

impl DdsSerialize for MyType {
    fn serialize<W: std::io::Write, E: rust_dds_rtps_implementation::dds_type::Endianness>(
        &self,
        _writer: W,
    ) -> rust_dds::DDSResult<()> {
        Ok(())
    }
}

impl<'de> DdsDeserialize<'de> for MyType {
    fn deserialize(_buf: &mut &'de [u8]) -> rust_dds::DDSResult<Self> {
        Ok(MyType {})
    }
}

#[test]
fn user_defined_write_read() {
    let unicast_address = [127, 0, 0, 1];
    let participant_factory = DomainParticipantFactory::get_instance();

    let mut qos = DomainParticipantQos::default();
    qos.entity_factory.autoenable_created_entities = false;

    let participant1 = participant_factory
        .create_participant(0, Some(qos.clone()), None, 0)
        .unwrap();

    let participant2 = participant_factory
        .create_participant(0, Some(qos.clone()), None, 0)
        .unwrap();

    let topic = participant1
        .create_topic::<MyType>("MyTopic", None, None, 0)
        .unwrap();

    let publisher = participant1.create_publisher(None, None, 0).unwrap();
    let mut writer = publisher.create_datawriter(&topic, None, None, 0).unwrap();

    let mut reader_qos = DataReaderQos::default();
    reader_qos.reliability.kind = ReliabilityQosPolicyKind::ReliableReliabilityQos;
    let subscriber = participant2.create_subscriber(None, None, 0).unwrap();
    let mut reader = subscriber
        .create_datareader(&topic, Some(reader_qos), None, 0)
        .unwrap();

    {
        let rtps_writer_shared = writer.as_ref().upgrade().unwrap();
        let rtps_writer = &mut rtps_writer_shared.write_lock().rtps_writer;
        let stateful_writer = rtps_writer.try_as_stateful_writer().unwrap();
        let writer_proxy = RtpsWriterProxyImpl::new(
            stateful_writer.guid().clone(),
            &[Locator::new(
                LOCATOR_KIND_UDPv4,
                port_user_unicast(0, 0) as u32,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 127, 0, 0, 1],
            )],
            &[],
            stateful_writer.data_max_size_serialized().clone(),
            stateful_writer.guid().entity_id,
        );

        let rtps_reader_shared = reader.as_ref().upgrade().unwrap();
        let rtps_reader = &mut rtps_reader_shared.write_lock().rtps_reader;
        let stateful_reader = rtps_reader.try_as_stateful_reader().unwrap();
        let reader_proxy = RtpsReaderProxyAttributesImpl::new(
            stateful_reader.guid().clone(),
            stateful_reader.guid().entity_id,
            &[Locator::new(
                LOCATOR_KIND_UDPv4,
                port_user_unicast(0, 1) as u32,
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 127, 0, 0, 1],
            )],
            &[],
            *stateful_reader.expects_inline_qos(),
            true,
        );

        stateful_writer.matched_reader_add(reader_proxy);
        stateful_reader.matched_writer_add(writer_proxy);
    }

    writer
        .write_w_timestamp(&MyType {}, None, Time { sec: 0, nanosec: 0 })
        .unwrap();

    let mut communication1 = Communication {
        version: PROTOCOLVERSION,
        vendor_id: VENDOR_ID_S2E,
        guid_prefix: GuidPrefix([3; 12]),
        transport: UdpTransport::new(
            get_unicast_socket(unicast_address.into(), port_user_unicast(0, 0)).unwrap(),
        ),
    };

    let mut communication2 = Communication {
        version: PROTOCOLVERSION,
        vendor_id: VENDOR_ID_S2E,
        guid_prefix: GuidPrefix([3; 12]),
        transport: UdpTransport::new(
            get_unicast_socket(unicast_address.into(), port_user_unicast(0, 1)).unwrap(),
        ),
    };

    communication1.send(&[publisher.as_ref().upgrade().unwrap()]);
    communication2.receive(&[subscriber.as_ref().upgrade().unwrap()]);

    let samples = reader.read(1, &[], &[], &[]).unwrap();
    assert!(samples.len() == 1);
}
