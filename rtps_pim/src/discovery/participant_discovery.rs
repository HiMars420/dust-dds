use crate::{
    behavior::{
        reader::{
            stateful_reader::{RTPSStatefulReader, RTPSStatefulReaderOperations},
            writer_proxy::RTPSWriterProxyOperations,
        },
        writer::{
            reader_proxy::RTPSReaderProxyOperations,
            stateful_writer::{RTPSStatefulWriter, RTPSStatefulWriterOperations},
        },
    },
    discovery::sedp::builtin_endpoints::ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
    structure::types::{Locator, ENTITYID_UNKNOWN, GUID},
};

use super::{
    sedp::{
        builtin_endpoints::{
            ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
            ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
            ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
        },
        sedp_participant::SedpParticipant,
    },
    spdp::spdp_discovered_participant_data::SPDPdiscoveredParticipantData,
    types::BuiltinEndpointSet,
};

pub fn discover_new_remote_participant<Participant, ParticipantData>(
    participant_data: &ParticipantData,
    local_participant: &mut Participant,
) where
    ParticipantData: SPDPdiscoveredParticipantData,
    ParticipantData::LocatorListType: IntoIterator<Item = Locator>,
    Participant: SedpParticipant + SPDPdiscoveredParticipantData,
    Participant::BuiltinPublicationsWriter: RTPSStatefulWriterOperations + RTPSStatefulWriter,
    <Participant::BuiltinPublicationsWriter as RTPSStatefulWriter>::ReaderProxyType:
        RTPSReaderProxyOperations,
    Participant::BuiltinPublicationsReader: RTPSStatefulReaderOperations + RTPSStatefulReader,
    <Participant::BuiltinPublicationsReader as RTPSStatefulReader>::WriterProxyType:
        RTPSWriterProxyOperations,
    Participant::BuiltinSubscriptionsWriter: RTPSStatefulWriterOperations + RTPSStatefulWriter,
    <Participant::BuiltinSubscriptionsWriter as RTPSStatefulWriter>::ReaderProxyType:
        RTPSReaderProxyOperations,
    Participant::BuiltinSubscriptionsReader: RTPSStatefulReaderOperations + RTPSStatefulReader,
    <Participant::BuiltinSubscriptionsReader as RTPSStatefulReader>::WriterProxyType:
        RTPSWriterProxyOperations,
    Participant::BuiltinTopicsWriter: RTPSStatefulWriterOperations + RTPSStatefulWriter,
    <Participant::BuiltinTopicsWriter as RTPSStatefulWriter>::ReaderProxyType:
        RTPSReaderProxyOperations,
    Participant::BuiltinTopicsReader: RTPSStatefulReaderOperations + RTPSStatefulReader,
    <Participant::BuiltinTopicsReader as RTPSStatefulReader>::WriterProxyType:
        RTPSWriterProxyOperations,
{
    // Check that the domainId of the discovered participant equals the local one.
    // If it is not equal then there the local endpoints are not configured to
    // communicate with the discovered participant.
    if participant_data.domain_id() != local_participant.domain_id() {
        return;
    }

    // Check that the domainTag of the discovered participant equals the local one.
    // If it is not equal then there the local endpoints are not configured to
    // communicate with the discovered participant.
    if participant_data.domain_tag() != local_participant.domain_tag() {
        return;
    }

    if participant_data
        .available_builtin_endpoints()
        .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR)
    {
        if let Some(sedp_builtin_publications_writer) =
            local_participant.sedp_builtin_publications_writer()
        {
            let guid = GUID::new(
                participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let is_active = true;
            let proxy = RTPSReaderProxyOperations::new(
                guid,
                remote_group_entity_id,
                participant_data.metatraffic_unicast_locator_list(),
                participant_data.metatraffic_multicast_locator_list(),
                expects_inline_qos,
                is_active,
            );
            sedp_builtin_publications_writer.matched_reader_add(proxy);
        }
    }

    if participant_data
        .available_builtin_endpoints()
        .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER)
    {
        if let Some(sedp_builtin_publications_reader) =
            local_participant.sedp_builtin_publications_reader()
        {
            let guid = GUID::new(
                participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let data_max_size_serialized = None;

            let proxy = RTPSWriterProxyOperations::new(
                guid,
                remote_group_entity_id,
                participant_data.metatraffic_unicast_locator_list(),
                participant_data.metatraffic_multicast_locator_list(),
                data_max_size_serialized,
            );
            sedp_builtin_publications_reader.matched_writer_add(proxy);
        }
    }

    if participant_data
        .available_builtin_endpoints()
        .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR)
    {
        if let Some(sedp_builtin_subscriptions_writer) =
            local_participant.sedp_builtin_subscriptions_writer()
        {
            let guid = GUID::new(
                participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let is_active = true;
            let proxy = RTPSReaderProxyOperations::new(
                guid,
                remote_group_entity_id,
                participant_data.metatraffic_unicast_locator_list(),
                participant_data.metatraffic_multicast_locator_list(),
                expects_inline_qos,
                is_active,
            );
            sedp_builtin_subscriptions_writer.matched_reader_add(proxy);
        }
    }

    if participant_data
        .available_builtin_endpoints()
        .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER)
    {
        if let Some(sedp_builtin_subscriptions_reader) =
            local_participant.sedp_builtin_subscriptions_reader()
        {
            let guid = GUID::new(
                participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let data_max_size_serialized = None;

            let proxy = RTPSWriterProxyOperations::new(
                guid,
                remote_group_entity_id,
                participant_data.metatraffic_unicast_locator_list(),
                participant_data.metatraffic_multicast_locator_list(),
                data_max_size_serialized,
            );
            sedp_builtin_subscriptions_reader.matched_writer_add(proxy);
        }
    }

    if participant_data
        .available_builtin_endpoints()
        .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_DETECTOR)
    {
        if let Some(sedp_builtin_topics_writer) = local_participant.sedp_builtin_topics_writer() {
            let guid = GUID::new(
                participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let is_active = true;
            let proxy = RTPSReaderProxyOperations::new(
                guid,
                remote_group_entity_id,
                participant_data.metatraffic_unicast_locator_list(),
                participant_data.metatraffic_multicast_locator_list(),
                expects_inline_qos,
                is_active,
            );
            sedp_builtin_topics_writer.matched_reader_add(proxy);
        }
    }

    if participant_data
        .available_builtin_endpoints()
        .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_ANNOUNCER)
    {
        if let Some(sedp_builtin_topics_reader) = local_participant.sedp_builtin_topics_reader() {
            let guid = GUID::new(
                participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let data_max_size_serialized = None;

            let proxy = RTPSWriterProxyOperations::new(
                guid,
                remote_group_entity_id,
                participant_data.metatraffic_unicast_locator_list(),
                participant_data.metatraffic_multicast_locator_list(),
                data_max_size_serialized,
            );
            sedp_builtin_topics_reader.matched_writer_add(proxy);
        }
    }
}

pub fn remove_previously_discovered_participant() {}
