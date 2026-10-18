//! KM-P6 RGB Tool - core library for the Aukey KM-P6 RGB mouse pad (Holtek, USB `04D9:A1FA`).
//!
//! Replaces the Python prototype (`prototype/python/Mauspad_GUI.py`); the packets follow the vendor software
//! G-aim Control Center 1.1.5. The evidence for every command is in `docs/PROTOCOL.md`.

pub mod config;
pub mod device;
pub mod i18n;
pub mod names;
pub mod protocol;
pub mod transport;

#[cfg(any(test, feature = "fake"))]
pub mod fake;

pub use device::KmP6;
pub use protocol::{Direction, Mode, Rgb, Settings};
pub use transport::{Transport, open_transport};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Device(String),
    #[error("{0}")]
    Transport(String),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Invalid(msg.into()))
}

/// Bytes as "08 00 20 05".
pub fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}
