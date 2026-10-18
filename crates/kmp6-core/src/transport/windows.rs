//! Windows: hidapi for the feature reports, a named mutex as the lock between processes.

use std::ffi::CString;

use hidapi::{HidApi, HidDevice};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

use super::{INTERFACE, LOCK_TIMEOUT_MS, PID, REPORT_LEN, Transport, USAGE_PAGE, VID, not_found, send_failed};
use crate::{Error, Result, tr};

const MUTEX_NAME: &str = "Local\\kmp6_rgbtool";

pub struct HidapiTransport {
    dev: HidDevice,
    _api: HidApi,
    path: String,
    mutex: HANDLE,
    locked: bool,
}

fn hid_err(e: hidapi::HidError) -> Error {
    Error::Transport(e.to_string())
}

impl HidapiTransport {
    pub fn open(path: Option<&str>) -> Result<Self> {
        let api = HidApi::new().map_err(hid_err)?;
        let path = match path {
            Some(p) => CString::new(p).map_err(|_| Error::Transport(tr!("transport.invalid_device_path")))?,
            // Windows lists one entry per top-level collection; the vendor one has usage page FF00.
            None => api
                .device_list()
                .find(|d| {
                    d.vendor_id() == VID
                        && d.product_id() == PID
                        && d.interface_number() == INTERFACE
                        && d.usage_page() == USAGE_PAGE
                })
                .map(|d| d.path().to_owned())
                .ok_or_else(not_found)?,
        };
        let dev = api.open_path(&path).map_err(hid_err)?;
        let name: Vec<u16> = MUTEX_NAME.encode_utf16().chain(Some(0)).collect();
        // SAFETY: valid, null-terminated name; the handle is closed in Drop.
        let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if mutex.is_null() {
            return Err(Error::Transport(tr!("transport.lock_create_failed", error = std::io::Error::last_os_error())));
        }
        Ok(Self { dev, _api: api, path: path.to_string_lossy().into_owned(), mutex, locked: false })
    }
}

impl Transport for HidapiTransport {
    fn set_feature(&mut self, data: &[u8; REPORT_LEN]) -> Result<()> {
        self.dev.send_feature_report(&super::with_report_id(data)).map_err(|e| send_failed(data, e))
    }

    fn lock(&mut self) -> Result<()> {
        // SAFETY: handle from CreateMutexW, lives as long as self.
        match unsafe { WaitForSingleObject(self.mutex, LOCK_TIMEOUT_MS) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => {
                self.locked = true;
                Ok(())
            }
            WAIT_TIMEOUT => Err(Error::Device(tr!("transport.busy_elsewhere"))),
            _ => Err(Error::Transport(tr!("transport.lock_failed", error = std::io::Error::last_os_error()))),
        }
    }

    fn unlock(&mut self) {
        if self.locked {
            // SAFETY: this thread owns the mutex (lock succeeded).
            unsafe { ReleaseMutex(self.mutex) };
            self.locked = false;
        }
    }

    fn path(&self) -> String {
        self.path.clone()
    }
}

impl Drop for HidapiTransport {
    fn drop(&mut self) {
        self.unlock();
        // SAFETY: the handle is ours.
        unsafe { CloseHandle(self.mutex) };
    }
}
