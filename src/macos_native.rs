//! This backend uses the Marcos HID manager to talk to HID devices

#![allow(non_upper_case_globals)]

use libc::{c_void, wchar_t};
use std::{
    cmp::min,
    ffi::{CStr, CString},
    mem, ptr,
    sync::Mutex,
};

use core_foundation_sys::{
    base::{kCFAllocatorDefault, Boolean, CFComparisonResult, CFGetTypeID, CFRange, CFRelease},
    number::{kCFNumberSInt32Type, CFNumberGetTypeID, CFNumberGetValue, CFNumberRef},
    runloop::{
        kCFRunLoopDefaultMode, kCFRunLoopRunFinished, kCFRunLoopRunTimedOut, CFRunLoopGetCurrent,
        CFRunLoopRunInMode,
    },
    set::{CFSetGetCount, CFSetGetValues},
    string::{
        kCFStringEncodingUTF8, CFStringCompareFlags, CFStringGetBytes, CFStringGetCStringPtr,
        CFStringGetLength, CFStringGetTypeID, CFStringRef,
    },
};
use io_kit_sys::{
    hid::{
        base::IOHIDDeviceRef,
        device::{IOHIDDeviceGetProperty, IOHIDDeviceGetService},
        keys::{
            kIOHIDManufacturerKey, kIOHIDPrimaryUsageKey, kIOHIDPrimaryUsagePageKey,
            kIOHIDProductIDKey, kIOHIDProductKey, kIOHIDSerialNumberKey,
            kIOHIDTransportBluetoothValue, kIOHIDTransportI2CValue, kIOHIDTransportKey,
            kIOHIDTransportSPIValue, kIOHIDTransportUSBValue, kIOHIDVendorIDKey,
            kIOHIDVersionNumberKey,
        },
        manager::{
            kIOHIDManagerOptionNone, IOHIDManagerClose, IOHIDManagerCopyDevices,
            IOHIDManagerCreate, IOHIDManagerRef, IOHIDManagerScheduleWithRunLoop,
            IOHIDManagerSetDeviceMatching,
        },
    },
    usb::usb_spec::{kUSBInterfaceClass, kUSBInterfaceNumber},
    IORegistryEntryGetRegistryEntryID, CFSTR,
};
use mach2::port::MACH_PORT_NULL;

use crate::{
    wchar_to_string, BusType, DeviceInfo, HidDeviceBackendBase, HidDeviceBackendMacos, HidError,
    HidResult, WcharString,
};

// From the Apple docs
const kCFStringEncodingUTF32LE: u32 = 0x1c000100;
const kUSBHIDClass: i32 = 3;
extern "C" {
    fn CFStringCompare(
        theString1: CFStringRef,
        theString2: CFStringRef,
        compareOptions: CFStringCompareFlags,
    ) -> CFComparisonResult;

    fn CFStringHasPrefix(theString: CFStringRef, prefix: CFStringRef) -> Boolean;
}

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

fn process_pending_events() {
    loop {
        unsafe {
            match CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.001, 0) {
                kCFRunLoopRunFinished | kCFRunLoopRunTimedOut => break,
                _ => {}
            }
        }
    }
}

#[repr(transparent)]
struct Device(IOHIDDeviceRef);

impl Device {
    pub fn int_property(&self, key: CFStringRef) -> i32 {
        unsafe {
            let p = IOHIDDeviceGetProperty(self.0, key);
            if !p.is_null() && CFGetTypeID(p) == CFNumberGetTypeID() {
                let mut value: i32 = 0;
                CFNumberGetValue(
                    p as CFNumberRef,
                    kCFNumberSInt32Type,
                    &mut value as *mut i32 as *mut _,
                );
                return value;
            }

            0
        }
    }

    pub fn string_property(&self, prop: CFStringRef) -> WcharString {
        // Should we be doing a type check like in int_property?
        let s = unsafe { IOHIDDeviceGetProperty(self.0, prop) } as CFStringRef;
        if s.is_null() {
            return WcharString::None;
        }

        s.into()
    }

    pub fn primary_usage_page(&self) -> i32 {
        self.int_property(CFSTR(kIOHIDPrimaryUsagePageKey))
    }

    pub fn primary_usage(&self) -> i32 {
        self.int_property(CFSTR(kIOHIDPrimaryUsageKey))
    }

    pub fn vendor_id(&self) -> u16 {
        self.int_property(CFSTR(kIOHIDVendorIDKey)) as u16
    }

    pub fn product_id(&self) -> u16 {
        self.int_property(CFSTR(kIOHIDProductIDKey)) as u16
    }

    pub fn serial_number(&self) -> WcharString {
        self.string_property(CFSTR(kIOHIDSerialNumberKey))
    }

    pub fn manufacturer_string(&self) -> WcharString {
        self.string_property(CFSTR(kIOHIDManufacturerKey))
    }

    pub fn product_string(&self) -> WcharString {
        self.string_property(CFSTR(kIOHIDProductKey))
    }

    pub fn release_number(&self) -> u16 {
        self.int_property(CFSTR(kIOHIDVersionNumberKey)) as u16
    }

    pub fn usb_interface_number(&self) -> i32 {
        self.int_property(CFSTR(kUSBInterfaceNumber))
    }

    pub fn usb_interface_class(&self) -> i32 {
        self.int_property(CFSTR(kUSBInterfaceClass))
    }
}

impl From<IOHIDDeviceRef> for Device {
    fn from(o: IOHIDDeviceRef) -> Self {
        Self(o)
    }
}

fn hid_enumerate() -> HidResult<Vec<DeviceInfo>> {
    hid_init()?;
    let guard = HID_MANAGER.lock().expect("hid lock");
    let manager = guard.as_ref().expect("hid manager");

    process_pending_events();

    let device_set = unsafe { IOHIDManagerCopyDevices(manager.0) };
    let devices = if !device_set.is_null() {
        unsafe {
            let ndevices = CFSetGetCount(device_set) as usize;
            let mut v = vec![ptr::null::<IOHIDDeviceRef>(); ndevices];
            CFSetGetValues(device_set, v.as_mut_ptr() as *mut _);
            v
        }
    } else {
        Vec::new()
    };

    let device_infos = devices
        .iter()
        .filter_map(|device| device_to_hid_device_info(unsafe { (**device).into() }))
        .flatten()
        .collect::<Vec<_>>();

    Ok(device_infos)
}

fn device_to_hid_device_info(device: Device) -> Option<Vec<DeviceInfo>> {
    todo!();
}

fn hid_device_info_with_usage(device: Device, usage_page: u16, usage: u16) -> Option<DeviceInfo> {
    let interface_number = if device.usb_interface_class() == kUSBHIDClass {
        device.usb_interface_number()
    } else {
        -1
    };

    let transport_prop = unsafe { IOHIDDeviceGetProperty(device.0, CFSTR(kIOHIDTransportKey)) };
    let bus_type =
        if !transport_prop.is_null() && unsafe { CFGetTypeID(transport_prop) == CFStringGetTypeID() } {
            let transport = transport_prop as CFStringRef;
            if unsafe { CFStringCompare(transport, CFSTR(kIOHIDTransportUSBValue), 0) }
                == CFComparisonResult::EqualTo
            {
                BusType::Usb
            } else if unsafe { CFStringHasPrefix(transport, CFSTR(kIOHIDTransportBluetoothValue)) } != 0 {
                // Matches "Bluetooth", "BluetoothLowEnergy" and "Bluetooth Low Energy" strings
                BusType::Bluetooth
            } else if unsafe { CFStringCompare(transport, CFSTR(kIOHIDTransportI2CValue), 0) }
                == CFComparisonResult::EqualTo
            {
                BusType::I2c
            } else if unsafe { CFStringCompare(transport, CFSTR(kIOHIDTransportSPIValue), 0) }
                == CFComparisonResult::EqualTo
            {
                BusType::Spi
            } else {
                BusType::Unknown
            }
        } else {
            BusType::Unknown
        };

    Some(DeviceInfo {
        path: lookup_path(&device),
        vendor_id: device.vendor_id(),
        product_id: device.product_id(),
        serial_number: device.serial_number(),
        release_number: device.release_number(),
        manufacturer_string: device.manufacturer_string(),
        product_string: device.product_string(),
        usage_page: usage_page,
        usage: usage,
        interface_number,
        bus_type,
    })
}

fn lookup_path(dev: &Device) -> CString {
    let iokitdev = unsafe { IOHIDDeviceGetService(dev.0) };
    if iokitdev == MACH_PORT_NULL {
        return CString::new("").unwrap();
    }

    let mut entry_id = mem::MaybeUninit::uninit();
    if unsafe { IORegistryEntryGetRegistryEntryID(iokitdev, entry_id.as_mut_ptr()) } != 0 {
        return CString::new("").unwrap();
    }

    CString::new(format!("DevSrvsID:{}", unsafe { entry_id.assume_init() })).unwrap()
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

impl From<CFStringRef> for WcharString {
    fn from(s: CFStringRef) -> Self {
        // Let's try to cheaply get a pointer to a UTF-8 string if the OS
        // already has that.
        let cptr = unsafe { CFStringGetCStringPtr(s, kCFStringEncodingUTF8) };
        if !cptr.is_null() {
            return Self::String(
                unsafe { CStr::from_ptr(cptr) }
                    .to_string_lossy()
                    .to_string(),
            );
        }

        // If that didn't work, we can instead go through the regular
        // conversion. This currently means two copies are created as
        // wchar_to_string assumes we're borrowing the pointer.
        let mut buf = vec![0 as wchar_t, 256];
        let len = buf.len() - 1;
        let slen = unsafe { CFStringGetLength(s) };
        let range = CFRange {
            location: 0,
            length: min(slen, len as isize),
        };
        let mut unused_len = mem::MaybeUninit::uninit();
        unsafe {
            CFStringGetBytes(
                s,
                range,
                kCFStringEncodingUTF32LE,
                b'?',
                0,
                buf.as_mut_ptr() as *mut _,
                (len * mem::size_of::<wchar_t>()) as isize,
                unused_len.as_mut_ptr(),
            );
            wchar_to_string(buf.as_ptr())
        }
    }
}
