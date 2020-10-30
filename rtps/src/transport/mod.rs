use crate::types::Locator;
use crate::messages::RtpsMessage;

pub mod udp;
pub mod memory;

#[derive(Debug)]
pub enum TransportError {
    IoError(std::io::Error),
    InterfaceNotFound(String),
    Other(String),
}

impl From<std::io::Error> for TransportError {
    fn from(error: std::io::Error) -> Self {
        TransportError::IoError(error)
    }
}

pub type TransportResult<T> = std::result::Result<T, TransportError>;
pub trait Transport : 'static{
    fn write(&self, message: RtpsMessage, destination_locator: &Locator);

    fn read(&self) -> TransportResult<Option<(RtpsMessage, Locator)>>;

    fn unicast_locator_list(&self) -> &Vec<Locator>;

    fn multicast_locator_list(&self) -> &Vec<Locator>;
}