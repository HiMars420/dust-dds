use rust_dds_api::{
    builtin_topics::ParticipantBuiltinTopicData, dcps_psm::BuiltInTopicKey,
    infrastructure::qos_policy::UserDataQosPolicy,
};
use rust_rtps_pim::{
    behavior::types::Duration,
    discovery::spdp::participant_proxy::ParticipantProxy,
    structure::types::{Guid, Locator, ENTITYID_PARTICIPANT},
};

use crate::{
    data_representation_builtin_endpoints::parameter_id_values::{
        DEFAULT_DOMAIN_TAG, DEFAULT_EXPECTS_INLINE_QOS, PID_DEFAULT_UNICAST_LOCATOR,
        PID_DOMAIN_TAG, PID_EXPECTS_INLINE_QOS, PID_METATRAFFIC_UNICAST_LOCATOR,
        PID_PARTICIPANT_LEASE_DURATION,
    },
    data_serialize_deserialize::{MappingRead, ParameterList, ParameterSerializer},
    dds_type::{DdsDeserialize, DdsSerialize},
};

use super::{
    dds_serialize_deserialize_impl::{
        BuiltinEndpointQosSerdeDeserialize, BuiltinEndpointQosSerdeSerialize,
        BuiltinEndpointSetSerdeDeserialize, BuiltinEndpointSetSerdeSerialize,
        CountSerdeDeserialize, CountSerdeSerialize, DurationSerdeDeserialize,
        DurationSerdeSerialize, GuidSerdeDeserialize, GuidSerdeSerialize, LocatorSerdeDeserialize,
        LocatorSerdeSerialize, ProtocolVersionSerdeDeserialize, ProtocolVersionSerdeSerialize,
        UserDataQosPolicySerdeSerialize,
    },
    parameter_id_values::{
        DEFAULT_BUILTIN_ENDPOINT_QOS, DEFAULT_PARTICIPANT_LEASE_DURATION, PID_BUILTIN_ENDPOINT_QOS,
        PID_BUILTIN_ENDPOINT_SET, PID_DEFAULT_MULTICAST_LOCATOR, PID_DOMAIN_ID,
        PID_METATRAFFIC_MULTICAST_LOCATOR, PID_PARTICIPANT_GUID,
        PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT, PID_PROTOCOL_VERSION, PID_USER_DATA, PID_VENDORID,
    },
};

#[derive(Debug, PartialEq)]
pub struct SpdpDiscoveredParticipantData<'a, S, L> {
    pub dds_participant_data: ParticipantBuiltinTopicData<'a>,
    pub participant_proxy: ParticipantProxy<S, L>,
    pub lease_duration: Duration,
}

impl<S> DdsSerialize for SpdpDiscoveredParticipantData<'_, S, Vec<Locator>>
where
    S: AsRef<str> + PartialEq<&'static str>,
{
    fn serialize<W: std::io::Write, E: crate::dds_type::Endianness>(
        &self,
        writer: W,
    ) -> rust_dds_api::return_type::DDSResult<()> {
        let mut parameter_list_serializer = ParameterSerializer::<_, E>::new(writer);

        parameter_list_serializer
            .serialize_parameter(PID_DOMAIN_ID, &self.participant_proxy.domain_id)
            .unwrap();

        if self.participant_proxy.domain_tag != DEFAULT_DOMAIN_TAG {
            parameter_list_serializer
                .serialize_parameter(PID_DOMAIN_TAG, &self.participant_proxy.domain_tag.as_ref())
                .unwrap();
        }

        parameter_list_serializer
            .serialize_parameter(
                PID_PROTOCOL_VERSION,
                &ProtocolVersionSerdeSerialize(&self.participant_proxy.protocol_version),
            )
            .unwrap();

        parameter_list_serializer
            .serialize_parameter(
                PID_PARTICIPANT_GUID,
                &GuidSerdeSerialize(&Guid {
                    prefix: self.participant_proxy.guid_prefix,
                    entity_id: ENTITYID_PARTICIPANT,
                }),
            )
            .unwrap();

        parameter_list_serializer
            .serialize_parameter(PID_VENDORID, &self.participant_proxy.vendor_id)
            .unwrap();

        if self.participant_proxy.expects_inline_qos != DEFAULT_EXPECTS_INLINE_QOS {
            parameter_list_serializer
                .serialize_parameter(
                    PID_EXPECTS_INLINE_QOS,
                    &self.participant_proxy.expects_inline_qos,
                )
                .unwrap();
        }

        for metatraffic_unicast_locator in &self.participant_proxy.metatraffic_unicast_locator_list
        {
            parameter_list_serializer
                .serialize_parameter(
                    PID_METATRAFFIC_UNICAST_LOCATOR,
                    &LocatorSerdeSerialize(metatraffic_unicast_locator),
                )
                .unwrap();
        }

        for metatraffic_multicast_locator in
            &self.participant_proxy.metatraffic_multicast_locator_list
        {
            parameter_list_serializer
                .serialize_parameter(
                    PID_METATRAFFIC_MULTICAST_LOCATOR,
                    &LocatorSerdeSerialize(metatraffic_multicast_locator),
                )
                .unwrap();
        }

        for default_unicast_locator in &self.participant_proxy.default_unicast_locator_list {
            parameter_list_serializer
                .serialize_parameter(
                    PID_DEFAULT_UNICAST_LOCATOR,
                    &LocatorSerdeSerialize(default_unicast_locator),
                )
                .unwrap();
        }

        for default_multicast_locator in &self.participant_proxy.default_multicast_locator_list {
            parameter_list_serializer
                .serialize_parameter(
                    PID_DEFAULT_MULTICAST_LOCATOR,
                    &LocatorSerdeSerialize(default_multicast_locator),
                )
                .unwrap();
        }

        parameter_list_serializer
            .serialize_parameter(
                PID_BUILTIN_ENDPOINT_SET,
                &BuiltinEndpointSetSerdeSerialize(
                    &self.participant_proxy.available_builtin_endpoints,
                ),
            )
            .unwrap();

        parameter_list_serializer
            .serialize_parameter(
                PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT,
                &CountSerdeSerialize(&self.participant_proxy.manual_liveliness_count),
            )
            .unwrap();

        if self.participant_proxy.builtin_endpoint_qos != DEFAULT_BUILTIN_ENDPOINT_QOS {
            parameter_list_serializer
                .serialize_parameter(
                    PID_BUILTIN_ENDPOINT_QOS,
                    &BuiltinEndpointQosSerdeSerialize(&self.participant_proxy.builtin_endpoint_qos),
                )
                .unwrap();
        }

        parameter_list_serializer
            .serialize_parameter(
                PID_PARTICIPANT_LEASE_DURATION,
                &DurationSerdeSerialize(&self.lease_duration),
            )
            .unwrap();

        if self.dds_participant_data.user_data != UserDataQosPolicy::default() {
            parameter_list_serializer
                .serialize_parameter(
                    PID_USER_DATA,
                    &UserDataQosPolicySerdeSerialize(&self.dds_participant_data.user_data),
                )
                .unwrap();
        }

        Ok(())
    }
}

impl<'de> DdsDeserialize<'de> for SpdpDiscoveredParticipantData<'_, String, Vec<Locator>> {
    fn deserialize(buf: &mut &'de [u8]) -> rust_dds_api::return_type::DDSResult<Self> {
        let param_list: ParameterList = MappingRead::read(buf).unwrap();

        let guid = param_list
            .get::<GuidSerdeDeserialize>(PID_PARTICIPANT_GUID)
            .unwrap()
            .0;
        let guid_prefix = guid.prefix;

        let dds_participant_data = ParticipantBuiltinTopicData {
            key: BuiltInTopicKey { value: [8, 8, 8] },
            user_data: UserDataQosPolicy { value: &[] },
        };

        let domain_id = param_list.get(PID_DOMAIN_ID).unwrap();
        let domain_tag = param_list
            .get(PID_DOMAIN_TAG)
            .unwrap_or(DEFAULT_DOMAIN_TAG.to_string());
        let protocol_version = param_list
            .get::<ProtocolVersionSerdeDeserialize>(PID_PROTOCOL_VERSION)
            .unwrap()
            .0;
        let vendor_id = param_list.get(PID_VENDORID).unwrap();
        let expects_inline_qos = param_list
            .get(PID_EXPECTS_INLINE_QOS)
            .unwrap_or(DEFAULT_EXPECTS_INLINE_QOS);
        let metatraffic_unicast_locator_list = param_list
            .get_list::<LocatorSerdeDeserialize>(PID_METATRAFFIC_UNICAST_LOCATOR)
            .unwrap();
        let metatraffic_multicast_locator_list = param_list
            .get_list::<LocatorSerdeDeserialize>(PID_METATRAFFIC_MULTICAST_LOCATOR)
            .unwrap();
        let default_unicast_locator_list = param_list
            .get_list::<LocatorSerdeDeserialize>(PID_DEFAULT_UNICAST_LOCATOR)
            .unwrap();
        let default_multicast_locator_list = param_list
            .get_list::<LocatorSerdeDeserialize>(PID_DEFAULT_MULTICAST_LOCATOR)
            .unwrap();
        let available_builtin_endpoints = param_list
            .get::<BuiltinEndpointSetSerdeDeserialize>(PID_BUILTIN_ENDPOINT_SET)
            .unwrap()
            .0;
        let manual_liveliness_count = param_list
            .get::<CountSerdeDeserialize>(PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT)
            .unwrap()
            .0;
        let builtin_endpoint_qos = param_list
            .get::<BuiltinEndpointQosSerdeDeserialize>(PID_BUILTIN_ENDPOINT_QOS)
            .unwrap_or(BuiltinEndpointQosSerdeDeserialize(
                DEFAULT_BUILTIN_ENDPOINT_QOS,
            ))
            .0;

        let participant_proxy = ParticipantProxy {
            domain_id,
            domain_tag,
            protocol_version,
            guid_prefix,
            vendor_id,
            expects_inline_qos,
            metatraffic_unicast_locator_list: metatraffic_unicast_locator_list
                .into_iter()
                .map(|l| l.0)
                .collect(),
            metatraffic_multicast_locator_list: metatraffic_multicast_locator_list
                .into_iter()
                .map(|l| l.0)
                .collect(),
            default_unicast_locator_list: default_unicast_locator_list
                .into_iter()
                .map(|l| l.0)
                .collect(),
            default_multicast_locator_list: default_multicast_locator_list
                .into_iter()
                .map(|l| l.0)
                .collect(),
            available_builtin_endpoints,
            manual_liveliness_count,
            builtin_endpoint_qos,
        };
        let lease_duration = param_list
            .get::<DurationSerdeDeserialize>(PID_PARTICIPANT_LEASE_DURATION)
            .unwrap_or(DurationSerdeDeserialize(DEFAULT_PARTICIPANT_LEASE_DURATION))
            .0;
        Ok(Self {
            dds_participant_data,
            participant_proxy,
            lease_duration,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::dds_type::LittleEndian;

    use super::*;
    use rust_dds_api::{dcps_psm::BuiltInTopicKey, infrastructure::qos_policy::UserDataQosPolicy};
    use rust_rtps_pim::{
        discovery::types::{BuiltinEndpointQos, BuiltinEndpointSet},
        messages::types::Count,
        structure::types::ProtocolVersion,
    };

    pub fn to_bytes_le<S: DdsSerialize>(value: &S) -> Vec<u8> {
        let mut writer = Vec::<u8>::new();
        value.serialize::<_, LittleEndian>(&mut writer).unwrap();
        writer
    }

    #[test]
    fn deserialize_spdp_discovered_participant_data() {
        let locator1 = Locator::new(11, 12, [1; 16]);
        let locator2 = Locator::new(21, 22, [2; 16]);

        let domain_id = 1;
        let domain_tag = "ab".to_string();
        let protocol_version = ProtocolVersion { major: 2, minor: 4 };
        let guid_prefix = [8; 12];
        let vendor_id = [73, 74];
        let expects_inline_qos = true;
        let metatraffic_unicast_locator_list = vec![locator1, locator2];
        let metatraffic_multicast_locator_list = vec![locator1];
        let default_unicast_locator_list = vec![locator1];
        let default_multicast_locator_list = vec![locator1];
        let available_builtin_endpoints =
            BuiltinEndpointSet::new(BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR);
        let manual_liveliness_count = Count(2);
        let builtin_endpoint_qos = BuiltinEndpointQos::new(
            BuiltinEndpointQos::BEST_EFFORT_PARTICIPANT_MESSAGE_DATA_READER,
        );

        let dds_participant_data = ParticipantBuiltinTopicData {
            key: BuiltInTopicKey { value: [8, 8, 8] },
            user_data: UserDataQosPolicy { value: &[] },
        };
        let participant_proxy = ParticipantProxy {
            domain_id,
            domain_tag,
            protocol_version,
            guid_prefix,
            vendor_id,
            expects_inline_qos,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            default_unicast_locator_list,
            default_multicast_locator_list,
            available_builtin_endpoints,
            manual_liveliness_count,
            builtin_endpoint_qos,
        };
        let lease_duration = Duration {
            seconds: 10,
            fraction: 11,
        };

        let expected = SpdpDiscoveredParticipantData {
            dds_participant_data,
            participant_proxy,
            lease_duration,
        };

        let mut data = &[
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x0f, 0x00, 0x04, 0x00, // PID_DOMAIN_ID, Length: 4
            0x01, 0x00, 0x00, 0x00, // DomainId
            0x14, 0x40, 0x08, 0x00, // PID_DOMAIN_TAG, Length: 8
            3, 0x00, 0x00, 0x00, // DomainTag: string length (incl. terminator)
            b'a', b'b', 0, 0x00, // DomainTag: string + padding (1 byte)
            0x15, 0x00, 4, 0x00, // PID_PROTOCOL_VERSION, Length
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion
            0x50, 0x00, 16, 0x00, // PID_PARTICIPANT_GUID, Length
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            0, 0, 1, 0xc1, // EntityId,
            0x16, 0x00, 4, 0x00, // PID_VENDORID
            73, 74, 0x00, 0x00, // VendorId
            0x43, 0x00, 0x04, 0x00, // PID_EXPECTS_INLINE_QOS, Length: 4,
            0x01, 0x00, 0x00, 0x00, // True
            0x32, 0x00, 24, 0x00, // PID_METATRAFFIC_UNICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x32, 0x00, 24, 0x00, // PID_METATRAFFIC_UNICAST_LOCATOR
            21, 0x00, 0x00, 0x00, // Locator{kind
            22, 0x00, 0x00, 0x00, // port,
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // address
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // }
            0x33, 0x00, 24, 0x00, // PID_METATRAFFIC_MULTICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x31, 0x00, 24, 0x00, // PID_DEFAULT_UNICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x48, 0x00, 24, 0x00, // PID_DEFAULT_MULTICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x58, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_SET
            0x02, 0x00, 0x00, 0x00, //
            0x34, 0x00, 4, 0x00, // PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT
            0x02, 0x00, 0x00, 0x00, // Count
            0x77, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_QOS
            0x00, 0x00, 0x00, 0x20, //
            0x02, 0x00, 8, 0x00, // PID_PARTICIPANT_LEASE_DURATION
            10, 0x00, 0x00, 0x00, // Duration: seconds
            11, 0x00, 0x00, 0x00, // Duration: fraction
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL
        ][..];
        let result: SpdpDiscoveredParticipantData<String, Vec<Locator>> =
            DdsDeserialize::deserialize(&mut data).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn serialize_spdp_discovered_participant_data() {
        let locator1 = Locator::new(11, 12, [1; 16]);
        let locator2 = Locator::new(21, 22, [2; 16]);

        let domain_id = 1;
        let domain_tag = "ab";
        let protocol_version = ProtocolVersion { major: 2, minor: 4 };
        let guid_prefix = [8; 12];
        let vendor_id = [73, 74];
        let expects_inline_qos = true;
        let metatraffic_unicast_locator_list = vec![locator1, locator2];
        let metatraffic_multicast_locator_list = vec![locator1];
        let default_unicast_locator_list = vec![locator1];
        let default_multicast_locator_list = vec![locator1];
        let available_builtin_endpoints =
            BuiltinEndpointSet::new(BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR);
        let manual_liveliness_count = Count(2);
        let builtin_endpoint_qos = BuiltinEndpointQos::new(
            BuiltinEndpointQos::BEST_EFFORT_PARTICIPANT_MESSAGE_DATA_READER,
        );

        let dds_participant_data = ParticipantBuiltinTopicData {
            key: BuiltInTopicKey { value: [8, 8, 8] },
            user_data: UserDataQosPolicy { value: &[] },
        };
        let participant_proxy = ParticipantProxy {
            domain_id,
            domain_tag,
            protocol_version,
            guid_prefix,
            vendor_id,
            expects_inline_qos,
            metatraffic_unicast_locator_list,
            metatraffic_multicast_locator_list,
            default_unicast_locator_list,
            default_multicast_locator_list,
            available_builtin_endpoints,
            manual_liveliness_count,
            builtin_endpoint_qos,
        };
        let lease_duration = Duration {
            seconds: 10,
            fraction: 11,
        };

        let data = SpdpDiscoveredParticipantData {
            dds_participant_data,
            participant_proxy,
            lease_duration,
        };

        let expected = vec![
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x0f, 0x00, 0x04, 0x00, // PID_DOMAIN_ID, Length: 4
            0x01, 0x00, 0x00, 0x00, // DomainId
            0x14, 0x40, 0x08, 0x00, // PID_DOMAIN_TAG, Length: 8
            3, 0x00, 0x00, 0x00, // DomainTag: string length (incl. terminator)
            b'a', b'b', 0, 0x00, // DomainTag: string + padding (1 byte)
            0x15, 0x00, 4, 0x00, // PID_PROTOCOL_VERSION, Length
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion
            0x50, 0x00, 16, 0x00, // PID_PARTICIPANT_GUID, Length
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            0, 0, 1, 0xc1, // EntityId
            0x16, 0x00, 4, 0x00, // PID_VENDORID
            73, 74, 0x00, 0x00, // VendorId
            0x43, 0x00, 0x04, 0x00, // PID_EXPECTS_INLINE_QOS, Length: 4,
            0x01, 0x00, 0x00, 0x00, // True
            0x32, 0x00, 24, 0x00, // PID_METATRAFFIC_UNICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x32, 0x00, 24, 0x00, // PID_METATRAFFIC_UNICAST_LOCATOR
            21, 0x00, 0x00, 0x00, // Locator{kind
            22, 0x00, 0x00, 0x00, // port,
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // address
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // }
            0x33, 0x00, 24, 0x00, // PID_METATRAFFIC_MULTICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x31, 0x00, 24, 0x00, // PID_DEFAULT_UNICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x48, 0x00, 24, 0x00, // PID_DEFAULT_MULTICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            0x58, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_SET
            0x02, 0x00, 0x00, 0x00, //
            0x34, 0x00, 4, 0x00, // PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT
            0x02, 0x00, 0x00, 0x00, // Count
            0x77, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_QOS
            0x00, 0x00, 0x00, 0x20, //
            0x02, 0x00, 8, 0x00, // PID_PARTICIPANT_LEASE_DURATION
            10, 0x00, 0x00, 0x00, // Duration: seconds
            11, 0x00, 0x00, 0x00, // Duration: fraction
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL
        ];
        assert_eq!(to_bytes_le(&data), expected);
    }
}
