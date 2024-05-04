use std::{
    error::Error,
    os::fd::{AsRawFd, BorrowedFd},
    path::Path,
};

use libudev::{Context, Device};
use packed_struct::prelude::*;

use crate::usb::SetupRequest;

pub const SYSFS_PATH_MAX: usize = 256;
pub const SYSFS_BUS_ID_SIZE: usize = 32;
pub const MAX_STATUS_NAME: usize = 18;
pub const USBIP_CMD_SIZE: usize = 48;
pub const USBIP_CMD_SUBMIT: u32 = 1;
pub const USBIP_CMD_UNLINK: u32 = 2;
pub const USBIP_RET_SUBMIT: u32 = 3;
pub const USBIP_RET_UNLINK: u32 = 4;
pub const USBIP_DIR_OUT: u32 = 0;
pub const USBIP_DIR_IN: u32 = 1;
pub const USBIP_VHCI_BUS_TYPE: &str = "platform";
pub const USBIP_VHCI_DEVICE_NAME: &str = "vhci_hcd.0";

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

/// Possible USBIP headers
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum USBIPCommandHeader {
    CmdSubmit(USBIPHeaderCmdSubmit),
    CmdUnlink(USBIPHeaderCmdUnlink),
}

/// Possible USBIP reply headers
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum USBIPReplyHeader {
    RetSubmit(USBIPHeaderRetSubmit),
    RetUnlink(USBIPHeaderRetUnlink),
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "48")]
pub struct USBIPHeaderInit {
    #[packed_field(bytes = "0..=19")]
    pub base: USBIPHeaderBasic,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "48")]
pub struct USBIPHeaderCmdSubmit {
    #[packed_field(bytes = "0..=19")]
    pub base: USBIPHeaderBasic,
    #[packed_field(bytes = "20..=23", endian = "msb")]
    pub transfer_flags: Integer<u32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "24..=27", endian = "msb")]
    pub transfer_buffer_length: Integer<i32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "28..=31", endian = "msb")]
    pub start_frame: Integer<i32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "32..=35", endian = "msb")]
    pub number_of_packets: Integer<i32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "36..=39", endian = "msb")]
    pub interval: Integer<i32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "40..=47", endian = "msb")]
    pub setup: SetupRequest,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "48")]
pub struct USBIPHeaderRetSubmit {
    #[packed_field(bytes = "0..=19")]
    pub base: USBIPHeaderBasic,
    #[packed_field(bytes = "20..=23", endian = "msb")]
    pub status: Integer<i32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "24..=27", endian = "msb")]
    pub actual_length: Integer<i32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "28..=31", endian = "msb")]
    pub start_frame: Integer<i32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "32..=35", endian = "msb")]
    pub number_of_packets: Integer<i32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "36..=39", endian = "msb")]
    pub error_count: Integer<i32, packed_bits::Bits<32>>,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "48")]
pub struct USBIPHeaderCmdUnlink {
    #[packed_field(bytes = "0..=19")]
    pub base: USBIPHeaderBasic,
    #[packed_field(bytes = "20..=23", endian = "msb")]
    pub seqnum: Integer<u32, packed_bits::Bits<32>>,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "48")]
pub struct USBIPHeaderRetUnlink {
    #[packed_field(bytes = "0..=19")]
    pub base: USBIPHeaderBasic,
    #[packed_field(bytes = "20..=23", endian = "msb")]
    pub status: Integer<i32, packed_bits::Bits<32>>,
}

/// USBIP Header Basic
#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "20")]
pub struct USBIPHeaderBasic {
    #[packed_field(bytes = "0..=3", endian = "msb")]
    pub command: Integer<u32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "4..=7", endian = "msb")]
    pub seqnum: Integer<u32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "8..=11", endian = "msb")]
    pub devid: Integer<u32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "12..=15", endian = "msb")]
    pub direction: Integer<u32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "16..=19", endian = "msb")]
    pub ep: Integer<u32, packed_bits::Bits<32>>,
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
        let fd = sockfd.as_raw_fd();

        // Create the content to send
        let data = format!("{port} {fd} {devid} {speed}");
        log::debug!("attach data: {data}");

        // Attach the device
        device.set_attribute_value("attach", data)?;
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
