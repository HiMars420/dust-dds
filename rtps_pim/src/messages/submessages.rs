use super::{
    submessage_elements::{
        EntityIdSubmessageElementType, ParameterListSubmessageElementType,
        SequenceNumberSetSubmessageElementType, SequenceNumberSubmessageElementType,
        SerializedDataSubmessageElementType,
    },
    types::SubmessageFlag,
};

pub trait RtpsSubmessagePIM<'a> {
    type AckNackSubmessageType;
    type DataSubmessageType;
    type DataFragSubmessageType;
    type GapSubmessageType;
    type HeartbeatSubmessageType;
    type HeartbeatFragSubmessageType;
    type InfoDestinationSubmessageType;
    type InfoReplySubmessageType;
    type InfoSourceSubmessageType;
    type InfoTimestampSubmessageType;
    type NackFragSubmessageType;
    type PadSubmessageType;
}

#[derive(Debug, PartialEq)]
pub enum RtpsSubmessageType<'a, PSM>
where
    PSM: RtpsSubmessagePIM<'a>,
{
    AckNack(PSM::AckNackSubmessageType),
    Data(PSM::DataSubmessageType),
    DataFrag(PSM::DataFragSubmessageType),
    Gap(PSM::GapSubmessageType),
    Heartbeat(PSM::HeartbeatSubmessageType),
    HeartbeatFrag(PSM::HeartbeatFragSubmessageType),
    InfoDestination(PSM::InfoDestinationSubmessageType),
    InfoReply(PSM::InfoReplySubmessageType),
    InfoSource(PSM::InfoSourceSubmessageType),
    InfoTimestamp(PSM::InfoTimestampSubmessageType),
    NackFrag(PSM::NackFragSubmessageType),
    Pad(PSM::PadSubmessageType),
}

pub trait AckNackSubmessage {
    type EntityIdSubmessageElementType;
    type SequenceNumberSetSubmessageElementType;
    type CountSubmessageElementType;

    fn new(
        endianness_flag: SubmessageFlag,
        final_flag: SubmessageFlag,
        reader_id: Self::EntityIdSubmessageElementType,
        writer_id: Self::EntityIdSubmessageElementType,
        reader_sn_state: Self::SequenceNumberSetSubmessageElementType,
        count: Self::CountSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn final_flag(&self) -> SubmessageFlag;
    fn reader_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn reader_sn_state(&self) -> &Self::SequenceNumberSetSubmessageElementType;
    fn count(&self) -> &Self::CountSubmessageElementType;
}

pub trait DataSubmessage<'a> {
    type EntityIdSubmessageElementType: EntityIdSubmessageElementType;
    type SequenceNumberSubmessageElementType: SequenceNumberSubmessageElementType;
    type ParameterListSubmessageElementType: ParameterListSubmessageElementType<'a>;
    type SerializedDataSubmessageElementType: SerializedDataSubmessageElementType<
        'a,
        Value = &'a [u8],
    >;

    fn new(
        endianness_flag: SubmessageFlag,
        inline_qos_flag: SubmessageFlag,
        data_flag: SubmessageFlag,
        key_flag: SubmessageFlag,
        non_standard_payload_flag: SubmessageFlag,
        reader_id: Self::EntityIdSubmessageElementType,
        writer_id: Self::EntityIdSubmessageElementType,
        writer_sn: Self::SequenceNumberSubmessageElementType,
        inline_qos: Self::ParameterListSubmessageElementType,
        serialized_payload: Self::SerializedDataSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn inline_qos_flag(&self) -> SubmessageFlag;
    fn data_flag(&self) -> SubmessageFlag;
    fn key_flag(&self) -> SubmessageFlag;
    fn non_standard_payload_flag(&self) -> SubmessageFlag;
    fn reader_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_sn(&self) -> &Self::SequenceNumberSubmessageElementType;
    fn inline_qos(&self) -> &Self::ParameterListSubmessageElementType;
    fn serialized_payload(&self) -> &Self::SerializedDataSubmessageElementType;
}

pub trait DataFragSubmessage {
    type EntityIdSubmessageElementType;
    type SequenceNumberSubmessageElementType;
    type FragmentNumberSubmessageElementType;
    type UShortSubmessageElementType;
    type ULongSubmessageElementType;
    type ParameterListSubmessageElementType;
    type SerializedDataFragmentSubmessageElementType;

    fn new(
        endianness_flag: SubmessageFlag,
        inline_qos_flag: SubmessageFlag,
        non_standard_payload_flag: SubmessageFlag,
        key_flag: SubmessageFlag,
        reader_id: Self::EntityIdSubmessageElementType,
        writer_id: Self::EntityIdSubmessageElementType,
        writer_sn: Self::SequenceNumberSubmessageElementType,
        fragment_starting_num: Self::FragmentNumberSubmessageElementType,
        fragments_in_submessage: Self::UShortSubmessageElementType,
        data_size: Self::ULongSubmessageElementType,
        fragment_size: Self::UShortSubmessageElementType,
        inline_qos: Self::ParameterListSubmessageElementType,
        serialized_payload: Self::SerializedDataFragmentSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn inline_qos_flag(&self) -> SubmessageFlag;
    fn non_standard_payload_flag(&self) -> SubmessageFlag;
    fn key_flag(&self) -> SubmessageFlag;
    fn reader_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_sn(&self) -> &Self::SequenceNumberSubmessageElementType;
    fn fragment_starting_num(&self) -> &Self::FragmentNumberSubmessageElementType;
    fn fragments_in_submessage(&self) -> &Self::UShortSubmessageElementType;
    fn data_size(&self) -> &Self::ULongSubmessageElementType;
    fn fragment_size(&self) -> &Self::UShortSubmessageElementType;
    fn inline_qos(&self) -> &Self::ParameterListSubmessageElementType;
    fn serialized_payload(&self) -> &Self::SerializedDataFragmentSubmessageElementType;
}

pub trait GapSubmessage {
    type EntityIdSubmessageElementType: EntityIdSubmessageElementType;
    type SequenceNumberSubmessageElementType: SequenceNumberSubmessageElementType;
    type SequenceNumberSetSubmessageElementType: SequenceNumberSetSubmessageElementType;

    fn new(
        endianness_flag: SubmessageFlag,
        reader_id: Self::EntityIdSubmessageElementType,
        writer_id: Self::EntityIdSubmessageElementType,
        gap_start: Self::SequenceNumberSubmessageElementType,
        gap_list: Self::SequenceNumberSetSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn reader_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn gap_start(&self) -> &Self::SequenceNumberSubmessageElementType;
    fn gap_list(&self) -> &Self::SequenceNumberSetSubmessageElementType;
    // gap_start_gsn: submessage_elements::SequenceNumber,
    // gap_end_gsn: submessage_elements::SequenceNumber,
}

pub trait HeartbeatSubmessage {
    type EntityIdSubmessageElementType;
    type SequenceNumberSubmessageElementType;
    type CountSubmessageElementType;

    fn new(
        endianness_flag: SubmessageFlag,
        final_flag: SubmessageFlag,
        liveliness_flag: SubmessageFlag,
        reader_id: Self::EntityIdSubmessageElementType,
        writer_id: Self::EntityIdSubmessageElementType,
        first_sn: Self::SequenceNumberSubmessageElementType,
        last_sn: Self::SequenceNumberSubmessageElementType,
        count: Self::CountSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn final_flag(&self) -> SubmessageFlag;
    fn liveliness_flag(&self) -> SubmessageFlag;
    fn reader_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn first_sn(&self) -> &Self::SequenceNumberSubmessageElementType;
    fn last_sn(&self) -> &Self::SequenceNumberSubmessageElementType;
    fn count(&self) -> &Self::CountSubmessageElementType;
    // current_gsn: submessage_elements::SequenceNumber,
    // first_gsn: submessage_elements::SequenceNumber,
    // last_gsn: submessage_elements::SequenceNumber,
    // writer_set: submessage_elements::GroupDigest,
    // secure_writer_set: submessage_elements::GroupDigest,
}

pub trait HeartbeatFragSubmessage {
    type EntityIdSubmessageElementType;
    type SequenceNumberSubmessageElementType;
    type FragmentNumberSubmessageElementType;
    type CountSubmessageElementType;

    fn new(
        endianness_flag: SubmessageFlag,
        reader_id: Self::EntityIdSubmessageElementType,
        writer_id: Self::EntityIdSubmessageElementType,
        writer_sn: Self::SequenceNumberSubmessageElementType,
        last_fragment_num: Self::FragmentNumberSubmessageElementType,
        count: Self::CountSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn reader_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_sn(&self) -> &Self::SequenceNumberSubmessageElementType;
    fn last_fragment_num(&self) -> &Self::FragmentNumberSubmessageElementType;
    fn count(&self) -> &Self::CountSubmessageElementType;
}

pub trait InfoDestinationSubmessage {
    type GuidPrefixSubmessageElementType;
    fn new(
        endianness_flag: SubmessageFlag,
        guid_prefix: Self::GuidPrefixSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn guid_prefix(&self) -> &Self::GuidPrefixSubmessageElementType;
}

pub trait InfoReplySubmessage {
    type LocatorListSubmessageElementType;

    fn new(
        endianness_flag: SubmessageFlag,
        multicast_flag: SubmessageFlag,
        unicast_locator_list: Self::LocatorListSubmessageElementType,
        multicast_locator_list: Self::LocatorListSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn multicast_flag(&self) -> SubmessageFlag;
    fn unicast_locator_list(&self) -> &Self::LocatorListSubmessageElementType;
    fn multicast_locator_list(&self) -> &Self::LocatorListSubmessageElementType;
}

pub trait InfoSourceSubmessage {
    type ProtocolVersionSubmessageElementType;
    type VendorIdSubmessageElementType;
    type GuidPrefixSubmessageElementType;

    fn new(
        endianness_flag: SubmessageFlag,
        protocol_version: Self::ProtocolVersionSubmessageElementType,
        vendor_id: Self::VendorIdSubmessageElementType,
        guid_prefix: Self::GuidPrefixSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn protocol_version(&self) -> &Self::ProtocolVersionSubmessageElementType;
    fn vendor_id(&self) -> &Self::VendorIdSubmessageElementType;
    fn guid_prefix(&self) -> &Self::GuidPrefixSubmessageElementType;
}

pub trait InfoTimestampSubmessage {
    type TimestampSubmessageElementType;
    fn new(
        endianness_flag: SubmessageFlag,
        invalidate_flag: SubmessageFlag,
        timestamp: Self::TimestampSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn invalidate_flag(&self) -> SubmessageFlag;
    fn timestamp(&self) -> &Self::TimestampSubmessageElementType;
}

pub trait NackFragSubmessage {
    type EntityIdSubmessageElementType;
    type SequenceNumberSubmessageElementType;
    type FragmentNumberSetSubmessageElementType;
    type CountSubmessageElementType;

    fn new(
        endianness_flag: SubmessageFlag,
        reader_id: Self::EntityIdSubmessageElementType,
        writer_id: Self::EntityIdSubmessageElementType,
        writer_sn: Self::SequenceNumberSubmessageElementType,
        fragment_number_state: Self::FragmentNumberSetSubmessageElementType,
        count: Self::CountSubmessageElementType,
    ) -> Self;
    fn endianness_flag(&self) -> SubmessageFlag;
    fn reader_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_id(&self) -> &Self::EntityIdSubmessageElementType;
    fn writer_sn(&self) -> &Self::SequenceNumberSubmessageElementType;
    fn fragment_number_state(&self) -> &Self::FragmentNumberSetSubmessageElementType;
    fn count(&self) -> &Self::CountSubmessageElementType;
}

pub trait PadSubmessage {}
