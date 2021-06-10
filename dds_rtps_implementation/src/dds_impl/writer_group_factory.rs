use rust_dds_api::{
    dcps_psm::StatusMask, infrastructure::qos::PublisherQos,
    publication::publisher_listener::PublisherListener, return_type::DDSResult,
};
use rust_rtps_pim::{
    behavior::types::DurationPIM,
    messages::types::ParameterIdPIM,
    structure::types::{
        DataPIM, EntityIdPIM, GuidPrefixPIM, InstanceHandlePIM, LocatorPIM, ParameterListPIM,
        SequenceNumberPIM, GUIDType, GUIDPIM,
    },
};

use crate::rtps_impl::rtps_writer_group_impl::RTPSWriterGroupImpl;

const ENTITYKIND_USER_DEFINED_WRITER_GROUP: u8 = 0x08;

pub trait WriterGroupFactoryTrait:
    GuidPrefixPIM
    + EntityIdPIM
    + SequenceNumberPIM
    + DurationPIM
    + InstanceHandlePIM
    + LocatorPIM
    + DataPIM
    + ParameterIdPIM
    + GUIDPIM<Self>
    + ParameterListPIM<Self>
    + Sized
{
}

impl<
        T: GuidPrefixPIM
            + EntityIdPIM
            + SequenceNumberPIM
            + DurationPIM
            + InstanceHandlePIM
            + LocatorPIM
            + DataPIM
            + ParameterIdPIM
            + GUIDPIM<Self>
            + ParameterListPIM<Self>
            + Sized,
    > WriterGroupFactoryTrait for T
{
}

pub struct WriterGroupFactory<PSM: WriterGroupFactoryTrait> {
    guid_prefix: PSM::GuidPrefixType,
    publisher_counter: u8,
    default_publisher_qos: PublisherQos,
}

impl<PSM: WriterGroupFactoryTrait> WriterGroupFactory<PSM>
where
    <PSM as GUIDPIM<PSM>>::GUIDType: Send,
    PSM::GuidPrefixType: Clone
{
    pub fn new(guid_prefix: PSM::GuidPrefixType) -> Self {
        Self {
            guid_prefix,
            publisher_counter: 0,
            default_publisher_qos: PublisherQos::default(),
        }
    }

    pub fn create_writer_group(
        &mut self,
        qos: Option<PublisherQos>,
        a_listener: Option<&'static dyn PublisherListener>,
        mask: StatusMask,
    ) -> DDSResult<RTPSWriterGroupImpl<PSM>> {
        let qos = qos.unwrap_or(self.default_publisher_qos.clone());
        let guid_prefix = self.guid_prefix.clone();

        self.publisher_counter += 1;
        let entity_id = [
            self.publisher_counter,
            0,
            0,
            ENTITYKIND_USER_DEFINED_WRITER_GROUP,
        ]
        .into();
        let guid = GUIDType::new(guid_prefix, entity_id);
        Ok(RTPSWriterGroupImpl::new(guid, qos, a_listener, mask))
    }

    pub fn set_default_qos(&mut self, qos: Option<PublisherQos>) {
        let qos = qos.unwrap_or_default();
        self.default_publisher_qos = qos;
    }

    pub fn get_default_qos(&self) -> PublisherQos {
        self.default_publisher_qos.clone()
    }
}

#[cfg(test)]
mod tests {
    // use rust_rtps_pim::structure::RTPSEntity;
    // use rust_rtps_udp_psm::RtpsUdpPsm;

    // use super::*;

    // #[test]
    // fn basic_create_writer_group() {
    //     let guid_prefix = [1; 12];
    //     let mut writer_group_factory: WriterGroupFactory<RtpsUdpPsm> =
    //         WriterGroupFactory::new(guid_prefix);

    //     writer_group_factory
    //         .create_writer_group(None, None, 0)
    //         .unwrap();
    //     assert_eq!(writer_group_factory.publisher_counter, 1);
    // }

    // #[test]
    // fn create_multiple_writer_groups() {
    //     let guid_prefix = [1; 12];
    //     let mut writer_group_factory: WriterGroupFactory<RtpsUdpPsm> =
    //         WriterGroupFactory::new(guid_prefix);

    //     let writer_group1 = writer_group_factory
    //         .create_writer_group(None, None, 0)
    //         .unwrap();
    //     let writer_group2 = writer_group_factory
    //         .create_writer_group(None, None, 0)
    //         .unwrap();

    //     assert!(writer_group1.guid() != writer_group2.guid());
    // }
}
