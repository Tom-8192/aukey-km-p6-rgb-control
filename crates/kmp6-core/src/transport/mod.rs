//! Feature report transport for the Aukey KM-P6.
//!
//! The pad shows up as a Holtek "USB Gaming Keyboard" with a keyboard interface (0) and a vendor interface (2,
//! usage page FF00). Lighting goes as an 8-byte feature report *without* report ID to the vendor interface; the
//! pad never answers (GET_FEATURE returns no data).
//!
//! * Windows: hidapi.
//! * Linux: direct `/dev/hidraw*` ioctls - no dependency, the kernel driver stays attached.
//!
//! Every command runs under a cross-process lock (`lock`/`unlock`) so that the GUI, the command line and the
//! shutdown hook do not interleave their packets.

use crate::{Error, Result, hex, tr};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

pub const VID: u16 = 0x04D9;
pub const PID: u16 = 0xA1FA;
/// USB interface and usage page of the vendor collection (G-aim `apConfig.bin`: `usagePage=ff00`, `usage=ff00`)
pub const INTERFACE: i32 = 2;
pub const USAGE_PAGE: u16 = 0xFF00;
/// data bytes; the report ID 0 is not included.
pub const REPORT_LEN: usize = 8;
/// Longest time a command waits for another program that is currently talking to the pad.
pub const LOCK_TIMEOUT_MS: u32 = 5000;

pub trait Transport {
    fn set_feature(&mut self, data: &[u8; REPORT_LEN]) -> Result<()>;
    /// Lock for a whole command (all packets of one setting).
    fn lock(&mut self) -> Result<()> {
        Ok(())
    }
    fn unlock(&mut self) {}
    /// Device path (hidapi path or /dev/hidrawN), for display only.
    fn path(&self) -> String {
        String::new()
    }
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn set_feature(&mut self, data: &[u8; REPORT_LEN]) -> Result<()> {
        (**self).set_feature(data)
    }
    fn lock(&mut self) -> Result<()> {
        (**self).lock()
    }
    fn unlock(&mut self) {
        (**self).unlock()
    }
    fn path(&self) -> String {
        (**self).path()
    }
}

/// Report as sent: report ID 0 in front of the data.
pub fn with_report_id(data: &[u8; REPORT_LEN]) -> [u8; REPORT_LEN + 1] {
    let mut buf = [0u8; REPORT_LEN + 1];
    buf[1..].copy_from_slice(data);
    buf
}

pub(crate) fn not_found() -> Error {
    Error::Transport(tr!(
        "transport.not_found",
        vid = format!("{VID:04X}"),
        pid = format!("{PID:04X}"),
        interface = INTERFACE
    ))
}

pub(crate) fn send_failed(data: &[u8; REPORT_LEN], e: impl std::fmt::Display) -> Error {
    Error::Transport(tr!("transport.send_failed", packet = hex(data), e = e))
}

/// Opens the pad (or the given path).
pub fn open_transport(path: Option<&str>) -> Result<Box<dyn Transport>> {
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::HidrawTransport::open(path)?))
    }
    #[cfg(windows)]
    {
        Ok(Box::new(windows::HidapiTransport::open(path)?))
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = path;
        Err(Error::Transport(tr!("transport.unsupported_os")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_id_in_front() {
        assert_eq!(with_report_id(&[8, 1, 2, 3, 4, 5, 6, 7]), [0, 8, 1, 2, 3, 4, 5, 6, 7]);
    }
}
