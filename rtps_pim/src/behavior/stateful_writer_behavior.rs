/// This file implements the behaviors described in 8.4.9 RTPS StatefulWriter Behavior
use crate::{
    messages::{
        submessage_elements::{
            CountSubmessageElementConstructor, EntityIdSubmessageElementConstructor,
            SequenceNumberSetSubmessageElementAttributes,
            SequenceNumberSetSubmessageElementConstructor,
            SequenceNumberSubmessageElementConstructor,
        },
        submessages::{
            AckNackSubmessageAttributes, DataSubmessageConstructor, GapSubmessageConstructor,
            HeartbeatSubmessageConstructor,
        },
        types::Count,
    },
    structure::{
        cache_change::RtpsCacheChangeAttributes,
        history_cache::{RtpsHistoryAttributes, RtpsHistoryCacheOperations},
        types::{ChangeKind, EntityId, Guid, SequenceNumber, ENTITYID_UNKNOWN},
    },
};

use super::writer::reader_proxy::{RtpsReaderProxyAttributes, RtpsReaderProxyOperations};

pub enum StatefulWriterBehavior<'a, R, C> {
    BestEffort(BestEffortStatefulWriterBehavior<'a, R, C>),
    Reliable(ReliableStatefulWriterBehavior<'a, R, C>),
}

pub struct BestEffortStatefulWriterBehavior<'a, R, C> {
    pub reader_proxy: &'a mut R,
    pub writer_cache: &'a C,
    pub last_change_sequence_number: &'a SequenceNumber,
}

impl<'a, R, C> BestEffortStatefulWriterBehavior<'a, R, C> {
    /// Implement 8.4.9.1.4 Transition T4
    pub fn send_unsent_changes<
        Data,
        EntityIdElement,
        SequenceNumberElement,
        CacheChange,
        Gap,
        SequenceNumberSetElement,
    >(
        &mut self,
        mut send_data: impl FnMut(Data),
        mut send_gap: impl FnMut(Gap),
    ) where
        R: RtpsReaderProxyOperations + RtpsReaderProxyAttributes,
        Data: DataSubmessageConstructor<
            EntityIdSubmessageElementType = EntityIdElement,
            SequenceNumberSubmessageElementType = SequenceNumberElement,
            ParameterListSubmessageElementType = &'a CacheChange::ParameterListType,
            SerializedDataSubmessageElementType = &'a CacheChange::DataType,
        >,
        C: RtpsHistoryAttributes<CacheChangeType = CacheChange>,
        CacheChange: RtpsCacheChangeAttributes + 'a,
        EntityIdElement: EntityIdSubmessageElementConstructor<EntityIdType = EntityId>,
        SequenceNumberElement:
            SequenceNumberSubmessageElementConstructor<SequenceNumberType = SequenceNumber>,
        SequenceNumberSetElement: SequenceNumberSetSubmessageElementConstructor<
            SequenceNumberType = SequenceNumber,
            SequenceNumberSetType = [SequenceNumber],
        >,
        Gap: GapSubmessageConstructor<
            EntityIdSubmessageElementType = EntityIdElement,
            SequenceNumberSubmessageElementType = SequenceNumberElement,
            SequenceNumberSetSubmessageElementType = SequenceNumberSetElement,
        >,
    {
        while let Some(seq_num) = self
            .reader_proxy
            .next_unsent_change(self.last_change_sequence_number)
        {
            if let Some(change) = self.writer_cache.get_change(&seq_num) {
                let endianness_flag = true;
                let inline_qos_flag = true;
                let (data_flag, key_flag) = match change.kind() {
                    ChangeKind::Alive => (true, false),
                    ChangeKind::NotAliveDisposed | ChangeKind::NotAliveUnregistered => {
                        (false, true)
                    }
                    _ => todo!(),
                };
                let non_standard_payload_flag = false;
                let reader_id =
                    EntityIdElement::new(self.reader_proxy.remote_reader_guid().entity_id());
                let writer_id = EntityIdElement::new(change.writer_guid().entity_id());
                let writer_sn = SequenceNumberElement::new(change.sequence_number());
                let inline_qos = change.inline_qos();
                let serialized_payload = change.data_value();
                let data_submessage = Data::new(
                    endianness_flag,
                    inline_qos_flag,
                    data_flag,
                    key_flag,
                    non_standard_payload_flag,
                    reader_id,
                    writer_id,
                    writer_sn,
                    inline_qos,
                    serialized_payload,
                );
                send_data(data_submessage)
            } else {
                let endianness_flag = true;
                let reader_id = EntityIdElement::new(&ENTITYID_UNKNOWN);
                let writer_id = EntityIdElement::new(&ENTITYID_UNKNOWN);
                let gap_start = SequenceNumberElement::new(&seq_num);
                let gap_list = SequenceNumberSetElement::new(&seq_num, &[]);
                let gap_submessage =
                    Gap::new(endianness_flag, reader_id, writer_id, gap_start, gap_list);
                send_gap(gap_submessage)
            }
        }
    }
}

/// This struct is a wrapper for the implementation of the behaviors described in 8.4.9.2 Reliable StatefulWriter Behavior
pub struct ReliableStatefulWriterBehavior<'a, R, C> {
    pub reader_proxy: &'a mut R,
    pub writer_cache: &'a C,
    pub last_change_sequence_number: &'a SequenceNumber,
    pub writer_guid: &'a Guid,
    pub heartbeat_count: &'a Count,
    pub after_heartbeat_period: bool,
}

impl<'a, R, C> ReliableStatefulWriterBehavior<'a, R, C> {
    /// Implement 8.4.9.2.4 Transition T4
    pub fn send_unsent_changes<
        Data,
        EntityIdElement,
        SequenceNumberElement,
        CacheChange,
        Gap,
        SequenceNumberSetElement,
    >(
        &mut self,
        mut send_data: impl FnMut(Data),
        mut send_gap: impl FnMut(Gap),
    ) where
        R: RtpsReaderProxyOperations + RtpsReaderProxyAttributes,
        C: RtpsHistoryAttributes<CacheChangeType = CacheChange>,
        Data: DataSubmessageConstructor<
            EntityIdSubmessageElementType = EntityIdElement,
            SequenceNumberSubmessageElementType = SequenceNumberElement,
            ParameterListSubmessageElementType = &'a CacheChange::ParameterListType,
            SerializedDataSubmessageElementType = &'a CacheChange::DataType,
        >,
        EntityIdElement: EntityIdSubmessageElementConstructor<EntityIdType = EntityId>,
        CacheChange: RtpsCacheChangeAttributes + 'a,
        SequenceNumberElement:
            SequenceNumberSubmessageElementConstructor<SequenceNumberType = SequenceNumber>,
        SequenceNumberSetElement: SequenceNumberSetSubmessageElementConstructor<
            SequenceNumberType = SequenceNumber,
            SequenceNumberSetType = [SequenceNumber],
        >,
        Gap: GapSubmessageConstructor<
            EntityIdSubmessageElementType = EntityIdElement,
            SequenceNumberSubmessageElementType = SequenceNumberElement,
            SequenceNumberSetSubmessageElementType = SequenceNumberSetElement,
        >,
    {
        while let Some(seq_num) = self
            .reader_proxy
            .next_unsent_change(self.last_change_sequence_number)
        {
            if let Some(change) = self.writer_cache.get_change(&seq_num) {
                let endianness_flag = true;
                let inline_qos_flag = true;
                let (data_flag, key_flag) = match change.kind() {
                    ChangeKind::Alive => (true, false),
                    ChangeKind::NotAliveDisposed | ChangeKind::NotAliveUnregistered => {
                        (false, true)
                    }
                    _ => todo!(),
                };
                let non_standard_payload_flag = false;
                let reader_id =
                    EntityIdElement::new(self.reader_proxy.remote_reader_guid().entity_id());
                let writer_id = EntityIdElement::new(change.writer_guid().entity_id());
                let writer_sn = SequenceNumberElement::new(change.sequence_number());
                let inline_qos = change.inline_qos();
                let serialized_payload = change.data_value();
                let data_submessage = Data::new(
                    endianness_flag,
                    inline_qos_flag,
                    data_flag,
                    key_flag,
                    non_standard_payload_flag,
                    reader_id,
                    writer_id,
                    writer_sn,
                    inline_qos,
                    serialized_payload,
                );
                send_data(data_submessage)
            } else {
                let endianness_flag = true;
                let reader_id = EntityIdElement::new(&ENTITYID_UNKNOWN);
                let writer_id = EntityIdElement::new(&ENTITYID_UNKNOWN);
                let gap_start = SequenceNumberElement::new(&seq_num);
                let gap_list = SequenceNumberSetElement::new(&seq_num, &[]);
                let gap_submessage =
                    Gap::new(endianness_flag, reader_id, writer_id, gap_start, gap_list);
                send_gap(gap_submessage)
            }
        }
    }

    /// Implement 8.4.9.2.7 Transition T7
    pub fn send_heartbeat<Heartbeat, EntityIdElement, CountElement, SequenceNumberElement>(
        &mut self,
        mut send_heartbeat: impl FnMut(Heartbeat),
    ) where
        C: RtpsHistoryCacheOperations,
        Heartbeat: HeartbeatSubmessageConstructor<
            EntityIdSubmessageElementType = EntityIdElement,
            SequenceNumberSubmessageElementType = SequenceNumberElement,
            CountSubmessageElementType = CountElement,
        >,
        EntityIdElement: EntityIdSubmessageElementConstructor<EntityIdType = EntityId>,
        CountElement: CountSubmessageElementConstructor<CountType = Count>,
        SequenceNumberElement:
            SequenceNumberSubmessageElementConstructor<SequenceNumberType = SequenceNumber>,
    {
        if self.after_heartbeat_period {
            let endianness_flag = true;
            let final_flag = false;
            let liveliness_flag = false;
            let reader_id = EntityIdElement::new(&ENTITYID_UNKNOWN);
            let writer_id = EntityIdElement::new(&self.writer_guid.entity_id);
            let first_sn =
                SequenceNumberElement::new(&self.writer_cache.get_seq_num_min().unwrap_or(0));
            let last_sn =
                SequenceNumberElement::new(&self.writer_cache.get_seq_num_min().unwrap_or(0));
            let count = CountElement::new(self.heartbeat_count);
            let heartbeat_submessage = Heartbeat::new(
                endianness_flag,
                final_flag,
                liveliness_flag,
                reader_id,
                writer_id,
                first_sn,
                last_sn,
                count,
            );
            send_heartbeat(heartbeat_submessage)
        }
    }

    /// Implement 8.4.9.2.8 Transition T8
    pub fn process_acknack<S>(
        &mut self,
        acknack: &impl AckNackSubmessageAttributes<
            SequenceNumberSetSubmessageElementType = impl SequenceNumberSetSubmessageElementAttributes<
                SequenceNumberType = SequenceNumber,
                SequenceNumberSetType = [SequenceNumber],
            >,
        >,
    ) where
        R: RtpsReaderProxyOperations + RtpsReaderProxyAttributes,
        S: AsRef<[SequenceNumber]>,
    {
        self.reader_proxy
            .acked_changes_set(acknack.reader_sn_state().base() - 1);
        self.reader_proxy.requested_changes_set(
            acknack.reader_sn_state().set(),
            self.last_change_sequence_number,
        );
    }

    /// Implement 8.4.8.2.10 Transition T10
    pub fn send_requested_changes<
        Data,
        EntityIdElement,
        SequenceNumberElement,
        CacheChange,
        Gap,
        SequenceNumberSetElement,
    >(
        &mut self,
        mut send_data: impl FnMut(Data),
        mut send_gap: impl FnMut(Gap),
    ) where
        R: RtpsReaderProxyOperations + RtpsReaderProxyAttributes,
        C: RtpsHistoryAttributes<CacheChangeType = CacheChange>,
        Data: DataSubmessageConstructor<
            EntityIdSubmessageElementType = EntityIdElement,
            SequenceNumberSubmessageElementType = SequenceNumberElement,
            ParameterListSubmessageElementType = &'a CacheChange::ParameterListType,
            SerializedDataSubmessageElementType = &'a CacheChange::DataType,
        >,
        EntityIdElement: EntityIdSubmessageElementConstructor<EntityIdType = EntityId>,
        CacheChange: RtpsCacheChangeAttributes + 'a,
        SequenceNumberElement:
            SequenceNumberSubmessageElementConstructor<SequenceNumberType = SequenceNumber>,
        SequenceNumberSetElement: SequenceNumberSetSubmessageElementConstructor<
            SequenceNumberType = SequenceNumber,
            SequenceNumberSetType = [SequenceNumber],
        >,
        Gap: GapSubmessageConstructor<
            EntityIdSubmessageElementType = EntityIdElement,
            SequenceNumberSubmessageElementType = SequenceNumberElement,
            SequenceNumberSetSubmessageElementType = SequenceNumberSetElement,
        >,
    {
        while let Some(seq_num) = self.reader_proxy.next_requested_change() {
            if let Some(change) = self.writer_cache.get_change(&seq_num) {
                let endianness_flag = true;
                let inline_qos_flag = true;
                let (data_flag, key_flag) = match change.kind() {
                    ChangeKind::Alive => (true, false),
                    ChangeKind::NotAliveDisposed | ChangeKind::NotAliveUnregistered => {
                        (false, true)
                    }
                    _ => todo!(),
                };
                let non_standard_payload_flag = false;
                let reader_id = EntityIdElement::new(&ENTITYID_UNKNOWN);
                let writer_id = EntityIdElement::new(change.writer_guid().entity_id());
                let writer_sn = SequenceNumberElement::new(change.sequence_number());
                let inline_qos = change.inline_qos();
                let serialized_payload = change.data_value();
                let data_submessage = Data::new(
                    endianness_flag,
                    inline_qos_flag,
                    data_flag,
                    key_flag,
                    non_standard_payload_flag,
                    reader_id,
                    writer_id,
                    writer_sn,
                    inline_qos,
                    serialized_payload,
                );
                send_data(data_submessage)
            } else {
                let endianness_flag = true;
                let reader_id = EntityIdElement::new(&ENTITYID_UNKNOWN);
                let writer_id = EntityIdElement::new(&ENTITYID_UNKNOWN);
                let gap_start = SequenceNumberElement::new(&seq_num);
                let gap_list = SequenceNumberSetElement::new(&seq_num, &[]);
                let gap_submessage =
                    Gap::new(endianness_flag, reader_id, writer_id, gap_start, gap_list);
                send_gap(gap_submessage)
            }
        }
    }
}
