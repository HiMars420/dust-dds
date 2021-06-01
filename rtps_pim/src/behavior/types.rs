///
/// This files shall only contain the types as listed in the DDSI-RTPS Version 2.3
/// Table 8.46 - Types definitions for the Behavior Module
///

pub trait DurationPIM {
    type DurationType: Copy + Send + Sync;
}

pub trait ParticipantMessageDataPIM {
    type ParticipantMessageDataType: Copy + Send + Sync;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeForReaderStatusKind {
    Unsent,
    Unacknowledged,
    Requested,
    Acknowledged,
    Underway,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeFromWriterStatusKind {
    Lost,
    Missing,
    Received,
    Unknown,
}
