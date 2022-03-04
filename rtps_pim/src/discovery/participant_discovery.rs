use crate::{
    behavior::{
        reader::{
            stateful_reader::RtpsStatefulReaderOperations, writer_proxy::RtpsWriterProxyConstructor,
        },
        writer::{
            reader_proxy::RtpsReaderProxyConstructor, stateful_writer::RtpsStatefulWriterOperations,
        },
    },
    structure::types::{Guid, ENTITYID_UNKNOWN},
};

use super::{
    sedp::builtin_endpoints::{
        ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
        ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
        ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
        ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
    },
    spdp::participant_proxy::RtpsSpdpDiscoveredParticipantDataAttributes,
    types::{BuiltinEndpointSet, DomainId},
};

pub struct ParticipantDiscovery<'a, P> {
    participant_data: &'a P,
}

impl<'a, P> ParticipantDiscovery<'a, P>
where
    P: RtpsSpdpDiscoveredParticipantDataAttributes,
{
    pub fn new(
        participant_data: &'a P,
        local_participant_domain_id: DomainId,
        local_participant_domain_tag: &'a str,
    ) -> core::result::Result<Self, ()> {
        // Check that the domainId of the discovered participant equals the local one.
        // If it is not equal then there the local endpoints are not configured to
        // communicate with the discovered participant.
        // AND
        // Check that the domainTag of the discovered participant equals the local one.
        // If it is not equal then there the local endpoints are not configured to
        // communicate with the discovered participant.
        if participant_data.domain_id() == local_participant_domain_id
            && participant_data.domain_tag() == local_participant_domain_tag
        {
            Ok(Self { participant_data })
        } else {
            Err(())
        }
    }

    pub fn discovered_participant_add_publications_writer<R>(
        &self,
        writer: &mut impl RtpsStatefulWriterOperations<ReaderProxyType = R>,
    ) where
        R: RtpsReaderProxyConstructor,
    {
        if self
            .participant_data
            .available_builtin_endpoints()
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR)
        {
            let remote_reader_guid = Guid::new(
                self.participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let proxy = R::new(
                remote_reader_guid,
                remote_group_entity_id,
                self.participant_data.metatraffic_unicast_locator_list(),
                self.participant_data.metatraffic_multicast_locator_list(),
                expects_inline_qos,
                true,
            );
            writer.matched_reader_add(proxy);
        }
    }

    pub fn discovered_participant_add_publications_reader<W>(
        &self,
        reader: &mut impl RtpsStatefulReaderOperations<WriterProxyType = W>,
    ) where
        W: RtpsWriterProxyConstructor,
    {
        if self
            .participant_data
            .available_builtin_endpoints()
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER)
        {
            let remote_writer_guid = Guid::new(
                self.participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let data_max_size_serialized = None;

            let proxy = W::new(
                remote_writer_guid,
                self.participant_data.metatraffic_unicast_locator_list(),
                self.participant_data.metatraffic_multicast_locator_list(),
                data_max_size_serialized,
                remote_group_entity_id,
            );

            reader.matched_writer_add(proxy);
        }
    }

    pub fn discovered_participant_add_subscriptions_writer<R>(
        &self,
        writer: &mut impl RtpsStatefulWriterOperations<ReaderProxyType = R>,
    ) where
        R: RtpsReaderProxyConstructor,
    {
        if self
            .participant_data
            .available_builtin_endpoints()
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR)
        {
            let remote_reader_guid = Guid::new(
                self.participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let proxy = R::new(
                remote_reader_guid,
                remote_group_entity_id,
                self.participant_data.metatraffic_unicast_locator_list(),
                self.participant_data.metatraffic_multicast_locator_list(),
                expects_inline_qos,
                true,
            );
            writer.matched_reader_add(proxy);
        }
    }

    pub fn discovered_participant_add_subscriptions_reader<W>(
        &self,
        reader: &mut impl RtpsStatefulReaderOperations<WriterProxyType = W>,
    ) where
        W: RtpsWriterProxyConstructor,
    {
        if self
            .participant_data
            .available_builtin_endpoints()
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER)
        {
            let remote_writer_guid = Guid::new(
                self.participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let data_max_size_serialized = None;

            let proxy = W::new(
                remote_writer_guid,
                self.participant_data.metatraffic_unicast_locator_list(),
                self.participant_data.metatraffic_multicast_locator_list(),
                data_max_size_serialized,
                remote_group_entity_id,
            );
            reader.matched_writer_add(proxy);
        }
    }

    pub fn discovered_participant_add_topics_writer<R>(
        &self,
        writer: &mut impl RtpsStatefulWriterOperations<ReaderProxyType = R>,
    ) where
        R: RtpsReaderProxyConstructor,
    {
        if self
            .participant_data
            .available_builtin_endpoints()
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_DETECTOR)
        {
            let remote_reader_guid = Guid::new(
                self.participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let proxy = R::new(
                remote_reader_guid,
                remote_group_entity_id,
                self.participant_data.metatraffic_unicast_locator_list(),
                self.participant_data.metatraffic_multicast_locator_list(),
                expects_inline_qos,
                true,
            );
            writer.matched_reader_add(proxy);
        }
    }

    pub fn discovered_participant_add_topics_reader<W>(
        &self,
        reader: &mut impl RtpsStatefulReaderOperations<WriterProxyType = W>,
    ) where
        W: RtpsWriterProxyConstructor,
    {
        if self
            .participant_data
            .available_builtin_endpoints()
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_ANNOUNCER)
        {
            let remote_writer_guid = Guid::new(
                self.participant_data.guid_prefix(),
                ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let data_max_size_serialized = None;

            let proxy = W::new(
                remote_writer_guid,
                self.participant_data.metatraffic_unicast_locator_list(),
                self.participant_data.metatraffic_multicast_locator_list(),
                data_max_size_serialized,
                remote_group_entity_id,
            );
            reader.matched_writer_add(proxy);
        }
    }
}

// fn discovered_participant_remove(&mut self, a_guid: &Guid) {
//     if let Some(sedp_builtin_publications_writer) = self.sedp_builtin_publications_writer() {
//         let guid = Guid::new(
//             *a_guid.prefix(),
//             ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
//         );
//         sedp_builtin_publications_writer.matched_reader_remove(&guid);
//     }

//     if let Some(sedp_builtin_publications_reader) = self.sedp_builtin_publications_reader() {
//         let guid = Guid::new(
//             *a_guid.prefix(),
//             ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
//         );
//         sedp_builtin_publications_reader.matched_writer_remove(&guid)
//     }

//     if let Some(sedp_builtin_subscriptions_writer) = self.sedp_builtin_subscriptions_writer() {
//         let guid = Guid::new(
//             *a_guid.prefix(),
//             ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
//         );
//         sedp_builtin_subscriptions_writer.matched_reader_remove(&guid);
//     }

//     if let Some(sedp_builtin_subscriptions_reader) = self.sedp_builtin_subscriptions_reader() {
//         let guid = Guid::new(
//             *a_guid.prefix(),
//             ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
//         );
//         sedp_builtin_subscriptions_reader.matched_writer_remove(&guid)
//     }

//     if let Some(sedp_builtin_topics_writer) = self.sedp_builtin_topics_writer() {
//         let guid = Guid::new(*a_guid.prefix(), ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR);
//         sedp_builtin_topics_writer.matched_reader_remove(&guid);
//     }

//     if let Some(sedp_builtin_topics_reader) = self.sedp_builtin_topics_reader() {
//         let guid = Guid::new(*a_guid.prefix(), ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER);
//         sedp_builtin_topics_reader.matched_writer_remove(&guid)
//     }
// }
// // }
