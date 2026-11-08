//! Linux without hidraw: SET_REPORT as a control transfer through usbfs (`/dev/bus/usb/BBB/DDD`).
//!
//! The vendor interface 2 of the pad has no endpoint at all, so `usbhid` refuses it ("couldn't find an input
//! interrupt endpoint") and the kernel never creates a `/dev/hidraw*` node for it (docs/PROTOCOL.md "Linux"). The
//! report is sent with the plain HID class request instead, the same thing hidapi does on Windows. Interface 2 has
//! no kernel driver, so nothing has to be detached; the keyboard interface 0 stays with `usbhid`.
//!
//! Lock between processes: `flock` on the usbfs node. Interface 2 is claimed only while a command runs, so the
//! GUI, the command line and the OpenRGB plugin can keep the device open side by side.

use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::{Duration, Instant};

use super::{INTERFACE, LOCK_TIMEOUT_MS, PID, REPORT_LEN, Transport, VID, not_found, send_failed};
use crate::{Error, Result, tr};

/// `struct usbdevfs_ctrltransfer` from `<linux/usbdevice_fs.h>`
#[repr(C)]
struct CtrlTransfer {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
    timeout_ms: u32,
    data: *mut libc::c_void,
}

const fn ioc(direction: u64, kind: u8, nr: u64, size: u64) -> u64 {
    (direction << 30) | (size << 16) | ((kind as u64) << 8) | nr
}
const IOC_WRITE: u64 = 1;
const IOC_READ: u64 = 2;
pub const USBDEVFS_CONTROL: u64 = ioc(IOC_WRITE | IOC_READ, b'U', 0, size_of::<CtrlTransfer>() as u64);
pub const USBDEVFS_CLAIMINTERFACE: u64 = ioc(IOC_READ, b'U', 15, size_of::<libc::c_uint>() as u64);
pub const USBDEVFS_RELEASEINTERFACE: u64 = ioc(IOC_READ, b'U', 16, size_of::<libc::c_uint>() as u64);

/// host to device, class request, recipient interface
const REQUEST_TYPE_CLASS_INTERFACE_OUT: u8 = 0x21;
/// HID class request SET_REPORT
const HID_SET_REPORT: u8 = 0x09;
/// report type feature (3) in the high byte, report ID 0 in the low byte
const FEATURE_REPORT_ID_0: u16 = 0x0300;
const TRANSFER_TIMEOUT_MS: u32 = 1000;

fn read_attr(dir: &Path, name: &str) -> Option<String> {
    fs::read_to_string(dir.join(name)).ok().map(|s| s.trim().to_owned())
}

/// `/dev/bus/usb/BBB/DDD` of the pad, if it is plugged in and has the vendor interface.
pub fn find_usb_device() -> Option<String> {
    let mut devices: Vec<_> = fs::read_dir("/sys/bus/usb/devices").ok()?.flatten().map(|e| e.path()).collect();
    devices.sort();
    for dev in devices {
        let (Some(vid), Some(pid)) = (read_attr(&dev, "idVendor"), read_attr(&dev, "idProduct")) else {
            continue;
        };
        if u16::from_str_radix(&vid, 16) != Ok(VID) || u16::from_str_radix(&pid, 16) != Ok(PID) {
            continue;
        }
        // interface directories are named "<device>:<config>.<ifnum>", e.g. "2-2.2:1.2"
        let name = dev.file_name()?.to_string_lossy().into_owned();
        let has_interface = fs::read_dir(&dev).ok()?.flatten().any(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.strip_prefix(&format!("{name}:"))
                .and_then(|rest| rest.rsplit_once('.'))
                .is_some_and(|(_, i)| i == INTERFACE.to_string())
        });
        if !has_interface {
            continue;
        }
        let (Some(bus), Some(num)) = (read_attr(&dev, "busnum"), read_attr(&dev, "devnum")) else {
            continue;
        };
        let (Ok(bus), Ok(num)) = (bus.parse::<u32>(), num.parse::<u32>()) else {
            continue;
        };
        return Some(format!("/dev/bus/usb/{bus:03}/{num:03}"));
    }
    None
}

pub struct UsbfsTransport {
    file: File,
    path: String,
    locked: bool,
    claimed: bool,
}

impl UsbfsTransport {
    pub fn open(path: Option<&str>) -> Result<Self> {
        let path = match path {
            Some(p) => p.to_owned(),
            None => find_usb_device().ok_or_else(not_found)?,
        };
        let file = OpenOptions::new().read(true).write(true).open(&path).map_err(|e| {
            if e.kind() == ErrorKind::PermissionDenied {
                Error::Transport(tr!("transport.no_access", path = path))
            } else {
                Error::Transport(format!("{path}: {e}"))
            }
        })?;
        Ok(Self { file, path, locked: false, claimed: false })
    }

    fn interface_ioctl(&self, request: u64) -> std::io::Result<()> {
        let mut interface = INTERFACE as libc::c_uint;
        // SAFETY: valid file descriptor, the argument is the unsigned int the request code names.
        let n = unsafe { libc::ioctl(self.file.as_raw_fd(), request as _, &mut interface) };
        if n < 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
    }

    fn claim(&mut self) -> Result<()> {
        if self.claimed {
            return Ok(());
        }
        match self.interface_ioctl(USBDEVFS_CLAIMINTERFACE) {
            Ok(()) => {
                self.claimed = true;
                Ok(())
            }
            // another program holds the interface without our lock (e.g. a libusb script)
            Err(e) if e.raw_os_error() == Some(libc::EBUSY) => Err(Error::Device(tr!("transport.busy_elsewhere"))),
            Err(e) => Err(Error::Transport(format!("{}: {e}", self.path))),
        }
    }

    fn release(&mut self) {
        if self.claimed {
            let _ = self.interface_ioctl(USBDEVFS_RELEASEINTERFACE);
            self.claimed = false;
        }
    }

    fn transfer(&self, data: &[u8; REPORT_LEN]) -> std::io::Result<()> {
        let mut buf = *data;
        let mut ctrl = CtrlTransfer {
            request_type: REQUEST_TYPE_CLASS_INTERFACE_OUT,
            request: HID_SET_REPORT,
            value: FEATURE_REPORT_ID_0,
            index: INTERFACE as u16,
            length: REPORT_LEN as u16,
            timeout_ms: TRANSFER_TIMEOUT_MS,
            data: buf.as_mut_ptr().cast(),
        };
        // SAFETY: valid file descriptor; ctrl matches the size in the request code and points to `length` bytes.
        let n = unsafe { libc::ioctl(self.file.as_raw_fd(), USBDEVFS_CONTROL as _, &mut ctrl) };
        if n < 0 {
            return Err(std::io::Error::last_os_error());
        }
        if n as usize != REPORT_LEN {
            return Err(std::io::Error::other(format!("{n} of {REPORT_LEN} bytes sent")));
        }
        Ok(())
    }
}

impl Transport for UsbfsTransport {
    fn set_feature(&mut self, data: &[u8; REPORT_LEN]) -> Result<()> {
        // usbfs would claim the interface by itself and keep it until the file is closed; claim it only for now
        let was_claimed = self.claimed;
        self.claim()?;
        let result = self.transfer(data).map_err(|e| send_failed(data, e));
        if !was_claimed {
            self.release();
        }
        result
    }

    fn lock(&mut self) -> Result<()> {
        let end = Instant::now() + Duration::from_millis(LOCK_TIMEOUT_MS as u64);
        loop {
            // SAFETY: valid file descriptor.
            if unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                self.locked = true;
                break;
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
        if let Err(e) = self.claim() {
            self.unlock();
            return Err(e);
        }
        Ok(())
    }

    fn unlock(&mut self) {
        self.release();
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

impl Drop for UsbfsTransport {
    fn drop(&mut self) {
        self.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usbfs_ioctl_numbers() {
        assert_eq!(size_of::<CtrlTransfer>(), 24);
        assert_eq!(USBDEVFS_CONTROL, 0xC0185500);
        assert_eq!(USBDEVFS_CLAIMINTERFACE, 0x8004550F);
        assert_eq!(USBDEVFS_RELEASEINTERFACE, 0x80045510);
    }
}
