//! This backend uses the macOS HID manager to talk to HID devices

use std::ffi::CStr;

use crate::{DeviceInfo, HidDeviceBackendBase, HidDeviceBackendMacos, HidResult};

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
