use core::ops::Deref;

use crate::{behavior::RTPSWriter, types::Locator};

use super::RTPSReaderLocator;

pub trait RTPSStatelessWriter<T: RTPSWriter>: Deref<Target = T> {
    type ReaderLocatorType: RTPSReaderLocator<Writer = T>;

    fn new(writer: T) -> Self;
    fn reader_locators(&self) -> &[Self::ReaderLocatorType];
    fn reader_locator_add(&mut self, a_locator: Locator);
    fn reader_locator_remove(&mut self, a_locator: &Locator);
    fn unsent_changes_reset(&mut self);
}
