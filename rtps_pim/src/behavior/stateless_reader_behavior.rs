use crate::{
    messages::submessages::DataSubmessage,
    structure::{
        cache_change::RtpsCacheChange,
        history_cache::RtpsHistoryCacheAddChange,
        types::{ChangeKind, Guid, GuidPrefix, ENTITYID_UNKNOWN},
    },
};

pub struct BestEffortStatelessReaderBehavior<'a, C> {
    pub reader_guid: &'a Guid,
    pub reader_cache: &'a mut C,
}

impl<C> BestEffortStatelessReaderBehavior<'_, C> {
    pub fn receive_data<P, D>(
        &mut self,
        source_guid_prefix: GuidPrefix,
        data: &DataSubmessage<P, D>,
    ) where
        C: for<'a> RtpsHistoryCacheAddChange<&'a P, &'a D>,
    {
        let reader_id = data.reader_id.value;
        if &reader_id == self.reader_guid.entity_id() || reader_id == ENTITYID_UNKNOWN {
            let kind = match (data.data_flag, data.key_flag) {
                (true, false) => ChangeKind::Alive,
                (false, true) => ChangeKind::NotAliveDisposed,
                _ => todo!(),
            };
            let writer_guid = Guid::new(source_guid_prefix, data.writer_id.value);
            let instance_handle = 0;
            let sequence_number = data.writer_sn.value;
            let data_value = &data.serialized_payload.value;
            let inline_qos = &data.inline_qos.parameter;
            let a_change = RtpsCacheChange {
                kind,
                writer_guid,
                instance_handle,
                sequence_number,
                data_value,
                inline_qos,
            };
            self.reader_cache.add_change(a_change);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        discovery::{
            sedp::builtin_endpoints::ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
            spdp::builtin_endpoints::{
                ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER, ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
            },
        },
        messages::submessage_elements::{
            EntityIdSubmessageElement, ParameterListSubmessageElement,
            SequenceNumberSubmessageElement, SerializedDataSubmessageElement,
        },
    };

    use super::*;

    #[test]
    fn best_effort_stateless_reader_receive_data_reader_id_unknown() {
        struct MockHistoryCache(bool);

        impl<'a> RtpsHistoryCacheAddChange<&'a (), &'a ()> for MockHistoryCache {
            fn add_change(&mut self, _change: RtpsCacheChange<&(), &()>) {
                self.0 = true;
            }
        }
        let mut history_cache = MockHistoryCache(false);
        let mut stateless_reader_behavior = BestEffortStatelessReaderBehavior {
            reader_guid: &Guid::new(
                GuidPrefix([1; 12]),
                ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
            ),
            reader_cache: &mut history_cache,
        };
        let data_submessage = DataSubmessage {
            endianness_flag: true,
            inline_qos_flag: true,
            data_flag: true,
            key_flag: false,
            non_standard_payload_flag: false,
            reader_id: EntityIdSubmessageElement {
                value: ENTITYID_UNKNOWN,
            },
            writer_id: EntityIdSubmessageElement {
                value: ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
            },
            writer_sn: SequenceNumberSubmessageElement { value: 1 },
            inline_qos: ParameterListSubmessageElement { parameter: () },
            serialized_payload: SerializedDataSubmessageElement { value: () },
        };
        stateless_reader_behavior.receive_data(GuidPrefix([2; 12]), &data_submessage);

        assert_eq!(history_cache.0, true);
    }

    #[test]
    fn best_effort_stateless_reader_receive_data_reader_id_same_as_receiver() {
        struct MockHistoryCache(bool);

        impl<'a> RtpsHistoryCacheAddChange<&'a (), &'a ()> for MockHistoryCache {
            fn add_change(&mut self, _change: RtpsCacheChange<&(), &()>) {
                self.0 = true;
            }
        }
        let mut history_cache = MockHistoryCache(false);
        let mut stateless_reader_behavior = BestEffortStatelessReaderBehavior {
            reader_guid: &Guid::new(
                GuidPrefix([1; 12]),
                ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
            ),
            reader_cache: &mut history_cache,
        };
        let data_submessage = DataSubmessage {
            endianness_flag: true,
            inline_qos_flag: true,
            data_flag: true,
            key_flag: false,
            non_standard_payload_flag: false,
            reader_id: EntityIdSubmessageElement {
                value: ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
            },
            writer_id: EntityIdSubmessageElement {
                value: ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
            },
            writer_sn: SequenceNumberSubmessageElement { value: 1 },
            inline_qos: ParameterListSubmessageElement { parameter: () },
            serialized_payload: SerializedDataSubmessageElement { value: () },
        };
        stateless_reader_behavior.receive_data(GuidPrefix([2; 12]), &data_submessage);

        assert_eq!(history_cache.0, true);
    }

    #[test]
    fn best_effort_stateless_reader_receive_data_reader_id_other_than_receiver() {
        struct MockHistoryCache(bool);

        impl<'a> RtpsHistoryCacheAddChange<&'a (), &'a ()> for MockHistoryCache {
            fn add_change(&mut self, _change: RtpsCacheChange<&(), &()>) {
                self.0 = true;
            }
        }
        let mut history_cache = MockHistoryCache(false);
        let mut stateless_reader_behavior = BestEffortStatelessReaderBehavior {
            reader_guid: &Guid::new(
                GuidPrefix([1; 12]),
                ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
            ),
            reader_cache: &mut history_cache,
        };
        let data_submessage = DataSubmessage {
            endianness_flag: true,
            inline_qos_flag: true,
            data_flag: true,
            key_flag: false,
            non_standard_payload_flag: false,
            reader_id: EntityIdSubmessageElement {
                value: ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
            },
            writer_id: EntityIdSubmessageElement {
                value: ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
            },
            writer_sn: SequenceNumberSubmessageElement { value: 1 },
            inline_qos: ParameterListSubmessageElement { parameter: () },
            serialized_payload: SerializedDataSubmessageElement { value: () },
        };
        stateless_reader_behavior.receive_data(GuidPrefix([2; 12]), &data_submessage);

        assert_eq!(history_cache.0, false);
    }
}
