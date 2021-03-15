use crate::{
    behavior::RTPSWriter,
    types::{Locator, SequenceNumber},
};

pub trait RTPSReaderLocator<'a> {
    type CacheChangeRepresentation;
    type CacheChangeRepresentationList: IntoIterator<Item = Self::CacheChangeRepresentation>;
    type Writer : RTPSWriter<'a>;
    type WriterReferenceType : core::ops::Deref<Target=Self::Writer>;

    fn requested_changes(&self) -> Self::CacheChangeRepresentationList;
    fn unsent_changes(&self) -> Self::CacheChangeRepresentationList;

    fn new(locator: Locator, expects_inline_qos: bool, writer: Self::WriterReferenceType) -> Self;
    fn locator(&self) -> Locator;
    fn expects_inline_qos(&self) -> bool;

    fn next_requested_change(&mut self) -> Option<Self::CacheChangeRepresentation>;
    fn next_unsent_change(&mut self) -> Option<Self::CacheChangeRepresentation>;
    fn requested_changes_set(&mut self, req_seq_num_set: &[SequenceNumber]);
}
