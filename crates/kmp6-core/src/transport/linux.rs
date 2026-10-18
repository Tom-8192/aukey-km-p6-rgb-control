//! Linux: `/dev/hidraw*` directly via ioctl (without hidapi), `flock` on the device as a lock between processes.

use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::{Duration, Instant};

use super::{INTERFACE, LOCK_TIMEOUT_MS, PID, REPORT_LEN, Transport, VID, not_found, send_failed};
use crate::{Error, Result, tr};

const fn ioc(direction: u64, kind: u8, nr: u64, size: u64) -> u64 {
    (direction << 30) | (size << 16) | ((kind as u64) << 8) | nr
}
const IOC_WRITE: u64 = 1;
const IOC_READ: u64 = 2;
pub const HIDIOCSFEATURE: u64 = ioc(IOC_WRITE | IOC_READ, b'H', 0x06, (REPORT_LEN + 1) as u64);

/// /dev/hidrawN of the vendor interface (USB interface 2).
pub fn find_hidraw() -> Option<String> {
    let hid_id = format!("HID_ID=0003:{:08X}:{:08X}", VID as u32, PID as u32);
    let mut nodes: Vec<_> = fs::read_dir("/sys/class/hidraw").ok()?.flatten().map(|e| e.path()).collect();
    nodes.sort();
    for node in nodes {
        let Ok(uevent) = fs::read_to_string(node.join("device").join("uevent")) else {
            continue;
        };
        if !uevent.to_uppercase().contains(&hid_id) {
            continue;
        }
        // .../3-2:1.2/0003:04D9:A1FA.0007 -> the USB interface directory ends with ".<ifnum>"
        let Ok(real) = fs::canonicalize(node.join("device")) else {
            continue;
        };
        let usb_if = real.parent().and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned());
        if usb_if.is_some_and(|n| n.ends_with(&format!(".{INTERFACE}"))) {
            return Some(format!("/dev/{}", node.file_name()?.to_string_lossy()));
        }
    }
    None
}

pub struct HidrawTransport {
    file: File,
    path: String,
    locked: bool,
}

impl HidrawTransport {
    pub fn open(path: Option<&str>) -> Result<Self> {
        let path = match path {
            Some(p) => p.to_owned(),
            None => find_hidraw().ok_or_else(not_found)?,
        };
        let file = OpenOptions::new().read(true).write(true).open(&path).map_err(|e| {
            if e.kind() == ErrorKind::PermissionDenied {
                Error::Transport(tr!("transport.no_access", path = path))
            } else {
                Error::Transport(format!("{path}: {e}"))
            }
        })?;
        Ok(Self { file, path, locked: false })
    }
}

impl Transport for HidrawTransport {
    fn set_feature(&mut self, data: &[u8; REPORT_LEN]) -> Result<()> {
        let mut buf = super::with_report_id(data);
        // SAFETY: buf is as large as the size in the request code says.
        let n = unsafe { libc::ioctl(self.file.as_raw_fd(), HIDIOCSFEATURE as _, buf.as_mut_ptr()) };
        if n < 0 {
            return Err(send_failed(data, std::io::Error::last_os_error()));
        }
        Ok(())
    }

    fn lock(&mut self) -> Result<()> {
        let end = Instant::now() + Duration::from_millis(LOCK_TIMEOUT_MS as u64);
        loop {
            // SAFETY: valid file descriptor.
            if unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                self.locked = true;
                return Ok(());
            }
            let err = std::io::Error::last_os_error();
            if err.kind() != ErrorKind::WouldBlock {
                return Err(Error::Transport(tr!("transport.lock_failed", error = err)));
            }
            if Instant::now() >= end {
                return Err(Error::Device(tr!("transport.busy_elsewhere")));
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn unlock(&mut self) {
        if self.locked {
            // SAFETY: valid file descriptor.
            unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
            self.locked = false;
        }
    }

    fn path(&self) -> String {
        self.path.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidraw_ioctl_number() {
        assert_eq!(HIDIOCSFEATURE, 0xC0094806);
    }
}
