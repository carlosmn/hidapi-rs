//! This backend uses the Marcos HID manager to talk to HID devices

use libc::c_void;
use std::{ffi::CStr, ptr, sync::Mutex};

use core_foundation_sys::{
    base::{kCFAllocatorDefault, CFRelease},
    runloop::{kCFRunLoopDefaultMode, CFRunLoopGetCurrent},
};
use io_kit_sys::hid::manager::{
    kIOHIDManagerOptionNone, IOHIDManagerClose, IOHIDManagerCreate, IOHIDManagerRef,
    IOHIDManagerScheduleWithRunLoop, IOHIDManagerSetDeviceMatching,
};

use crate::{DeviceInfo, HidDeviceBackendBase, HidDeviceBackendMacos, HidError, HidResult};

static HID_MANAGER: Mutex<Option<HidManager>> = Mutex::new(None);

/// Struct wrapping the manager so we can do rusty stuff
#[repr(transparent)]
struct HidManager(IOHIDManagerRef);

unsafe impl Send for HidManager {}

impl HidManager {
    pub fn new() -> Option<Self> {
        let mgr = unsafe { IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDManagerOptionNone) };

        if mgr.is_null() {
            None
        } else {
            Some(Self(mgr))
        }
    }

    pub fn close(&mut self) {
        unsafe { IOHIDManagerClose(self.0, kIOHIDManagerOptionNone) };
    }
}

impl Drop for HidManager {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.0 as *const c_void);
        }
    }
}

pub fn hid_init() -> HidResult<()> {
    let manager = HID_MANAGER.lock().unwrap();
    if manager.is_none() {
        // TODO: set open exclusive and store if we're on macOS >= 10.10
        if let Some(mgr) = HidManager::new() {
            unsafe {
                IOHIDManagerSetDeviceMatching(mgr.0, ptr::null());
                IOHIDManagerScheduleWithRunLoop(
                    mgr.0,
                    CFRunLoopGetCurrent(),
                    kCFRunLoopDefaultMode,
                );
            }
            return Ok(());
        } else {
            return Err(HidError::InitializationError);
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub fn hid_exit() -> HidResult<()> {
    // Take the manager and let the Drop clean it
    let mgr = HID_MANAGER.lock().unwrap().take();
    if let Some(mut mgr) = mgr {
        mgr.close();
    }
    Ok(())
}

pub struct HidApiBackend;

impl HidApiBackend {
    pub fn get_hid_device_info_vector() -> HidResult<Vec<DeviceInfo>> {
        todo!()
    }

    pub fn open(vid: u16, pid: u16) -> HidResult<HidDevice> {
        todo!()
    }

    pub fn open_serial(vid: u16, pid: u16, sn: &str) -> HidResult<HidDevice> {
        todo!()
    }

    pub fn open_path(device_path: &CStr) -> HidResult<HidDevice> {
        todo!()
    }
}

pub struct HidDevice;

impl HidDeviceBackendBase for HidDevice {
    fn write(&self, data: &[u8]) -> HidResult<usize> {
        todo!()
    }

    fn read(&self, buf: &mut [u8]) -> HidResult<usize> {
        todo!()
    }

    fn read_timeout(&self, buf: &mut [u8], timeout: i32) -> HidResult<usize> {
        todo!()
    }

    fn send_feature_report(&self, data: &[u8]) -> HidResult<()> {
        todo!()
    }

    fn get_feature_report(&self, buf: &mut [u8]) -> HidResult<usize> {
        todo!()
    }

    fn set_blocking_mode(&self, blocking: bool) -> HidResult<()> {
        todo!()
    }

    fn get_device_info(&self) -> HidResult<DeviceInfo> {
        todo!()
    }

    fn get_manufacturer_string(&self) -> HidResult<Option<String>> {
        todo!()
    }

    fn get_product_string(&self) -> HidResult<Option<String>> {
        todo!()
    }

    fn get_serial_number_string(&self) -> HidResult<Option<String>> {
        todo!()
    }
}

impl HidDeviceBackendMacos for HidDevice {
    fn get_location_id(&self) -> HidResult<u32> {
        todo!()
    }

    fn is_open_exclusive(&self) -> HidResult<bool> {
        todo!()
    }
}
