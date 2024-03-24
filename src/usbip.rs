use std::{
    error::Error,
    os::fd::{AsRawFd, BorrowedFd},
    path::Path,
};

use libudev::{Context, Device};
use packed_struct::prelude::*;

const SYSFS_PATH_MAX: usize = 256;
const SYSFS_BUS_ID_SIZE: usize = 32;
const MAX_STATUS_NAME: usize = 18;
const USBIP_VHCI_BUS_TYPE: &str = "platform";
const USBIP_VHCI_DEVICE_NAME: &str = "vhci_hcd.0";

/// Available USB Hub speeds
pub enum HubSpeed {
    High = 0,
    Super = 1,
}

/// Available USB Speeds
pub enum USBDeviceSpeed {
    USBSpeedUnknown = 0,   /* enumerating */
    USBSpeedLow = 1,       /* usb 1.1 */
    USBSpeedFull = 2,      /* usb 1.1 */
    USBSpeedHigh = 3,      /* usb 2.0 */
    USBSpeedWireless = 4,  /* wireless (usb 2.5) */
    USBSpeedSuper = 5,     /* usb 3.0 */
    USBSpeedSuperPlus = 6, /* usb 3.1 */
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0")]
pub struct USBDevice {
    #[packed_field(bytes = "0..=255", element_size_bytes = "1", endian = "lsb")]
    pub path: [u8; 256],
    #[packed_field(bytes = "256..=287", element_size_bytes = "1", endian = "lsb")]
    pub busid: [u8; 32],

    #[packed_field(bytes = "288..=291", endian = "lsb")]
    pub busnum: Integer<u32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "292..=295", endian = "lsb")]
    pub devnum: Integer<u32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "296..=299", endian = "lsb")]
    pub speed: Integer<u32, packed_bits::Bits<32>>,

    #[packed_field(bytes = "300..=301", endian = "lsb")]
    pub id_vendor: Integer<u16, packed_bits::Bits<16>>,
    #[packed_field(bytes = "302..=303", endian = "lsb")]
    pub id_product: Integer<u16, packed_bits::Bits<16>>,
    #[packed_field(bytes = "304..=305", endian = "lsb")]
    pub bcd_device: Integer<u16, packed_bits::Bits<16>>,

    #[packed_field(bytes = "306")]
    pub b_device_class: u8,
    #[packed_field(bytes = "307")]
    pub b_device_subclass: u8,
    #[packed_field(bytes = "308")]
    pub b_device_protocol: u8,
    #[packed_field(bytes = "309")]
    pub b_configuration_value: u8,
    #[packed_field(bytes = "310")]
    pub b_num_configurations: u8,
    #[packed_field(bytes = "311")]
    pub b_num_interfaces: u8,
}

/// Device imported from USBIP
pub struct ImportedDevice {
    hub: HubSpeed,
    port: u8,
    status: u32,
    devid: u32,
    busnum: u8,
    devnum: u8,
    udev: USBDevice,
}

/// Driver for interfacing with the sysfs API for vhci-hcd.
#[derive(Default)]
pub struct Driver {
    /* /sys/devices/platform/vhci_hcd */
    hc_device: Option<Device>,
    n_controllers: i32,
    n_ports: i32,
}

impl Driver {
    pub fn new() -> Self {
        Driver::default()
    }

    /// Open the vhci driver api
    /// TODO: Create a driver per vhci-hcd controller
    pub fn open(&mut self) -> Result<(), Box<dyn Error>> {
        let context = Context::new()?;

        // Open the hc device
        let syspath = format!("/sys/devices/{USBIP_VHCI_BUS_TYPE}/{USBIP_VHCI_DEVICE_NAME}");
        let syspath = Path::new(syspath.as_str());
        let hc_device = Device::from_syspath(&context, syspath)?;
        self.hc_device = Some(hc_device);

        // Find the number of available ports
        let nports = self.get_nports()?;
        if nports <= 0 {
            return Err("No available ports".into());
        }
        log::debug!("available ports: {nports}");
        self.n_ports = nports;

        Ok(())
    }

    /// Attach a given device to the given port
    pub fn attach_device2(
        &mut self,
        port: u8,
        sockfd: BorrowedFd,
        devid: u32,
        speed: u32,
    ) -> Result<(), Box<dyn Error>> {
        let Some(device) = self.hc_device.as_mut() else {
            return Err("Device driver has not been opened".into());
        };
        let Some(path) = device.syspath() else {
            return Err("Failed to get device path".into());
        };
        let fd = sockfd.as_raw_fd();

        // Create the content to send
        let data = format!("{port} {fd} {devid}, {speed}");
        log::debug!("attach data: {data}");

        // Construct the path to the sysfs attach attribute
        let path = path.to_string_lossy().to_string();
        let attach_attr_path = format!("{path}/attach");
        log::debug!("attach attribute path: {attach_attr_path}");

        // Attach the device
        device.set_attribute_value(attach_attr_path, data)?;
        log::debug!("attached port: {port}");

        Ok(())
    }

    /// Get the number of ports from the vhci device
    fn get_nports(&self) -> Result<i32, Box<dyn Error>> {
        let Some(ref device) = self.hc_device else {
            return Err("Device driver has not been opened".into());
        };
        let result = device.attribute_value("nports");
        if result.is_none() {
            return Err("Unable to find nports attribute".into());
        }
        let nports = result.unwrap().to_string_lossy().to_string().parse()?;

        Ok(nports)
    }
}
