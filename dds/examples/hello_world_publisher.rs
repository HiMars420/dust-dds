use cdr::CdrBe;
use dds::{
    domain::domain_participant::DomainParticipant,
    domain_participant_factory::DomainParticipantFactory,
    publication::{data_writer::DataWriter, publisher::Publisher},
    types::Time,
    DDSError,
};
use dds_implementation::dds_type::{DdsDeserialize, DdsSerialize, DdsType};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct HelloWorldType {
    id: u8,
    msg: String,
}

impl DdsType for HelloWorldType {
    fn type_name() -> &'static str {
        "HelloWorldType"
    }

    fn has_key() -> bool {
        false
    }
}

impl DdsSerialize for HelloWorldType {
    fn serialize<W: std::io::Write, E: dds_implementation::dds_type::Endianness>(
        &self,
        mut writer: W,
    ) -> dds::DDSResult<()> {
        writer
            .write(
                cdr::serialize::<_, _, CdrBe>(self, cdr::Infinite)
                    .map_err(|e| DDSError::PreconditionNotMet(format!("{}", e)))?
                    .as_slice(),
            )
            .map_err(|e| DDSError::PreconditionNotMet(format!("{}", e)))?;
        Ok(())
    }
}

impl<'de> DdsDeserialize<'de> for HelloWorldType {
    fn deserialize(buf: &mut &'de [u8]) -> dds::DDSResult<Self> {
        cdr::deserialize::<HelloWorldType>(buf)
            .map_err(|e| DDSError::PreconditionNotMet(format!("{}", e)))
    }
}

fn main() {
    let domain_id = 0;
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, None, None, 0)
        .unwrap();
    println!("{:?} [P] Created participant", std::time::SystemTime::now());

    while participant
        .get_builtin_subscriber()
        .unwrap()
        .as_ref()
        .upgrade()
        .unwrap()
        .data_reader_list
        .read_lock()
        .iter()
        .filter_map(|r| {
            r.rtps_reader
                .write_lock()
                .try_as_stateful_reader()
                .ok()
                .map(|sr| sr.matched_writers.len())
        })
        .next()
        .unwrap()
        < 2
    {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    println!("{:?} [P] Matched participant", std::time::SystemTime::now());

    let topic = participant
        .create_topic::<HelloWorldType>("HelloWorld", None, None, 0)
        .unwrap();

    let publisher = participant.create_publisher(None, None, 0).unwrap();
    let writer = publisher.create_datawriter(&topic, None, None, 0).unwrap();
    println!("{:?} [P] Created writer", std::time::SystemTime::now());
    
    publisher.create_datawriter(&topic, None, None, 0).unwrap();

    while writer
        .as_ref()
        .upgrade()
        .unwrap()
        .rtps_writer
        .write_lock()
        .try_as_stateful_writer()
        .unwrap()
        .matched_readers
        .len()
        == 0
    {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    println!("{:?} [P] Matched with reader", std::time::SystemTime::now());

    let hello_world = HelloWorldType {
        id: 8,
        msg: "Hello world!".to_string(),
    };
    writer
        .write_w_timestamp(&hello_world, None, Time { sec: 0, nanosec: 0 })
        .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(5));
}
