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
pub const USBIP_VHCI_BUS_TYPE: &str = "platform";
pub const USBIP_VHCI_DEVICE_NAME: &str = "vhci_hcd.0";

/// Request direction. This is always from the perspective of the host (i.e. host computer)
#[derive(PrimitiveEnum_u32, Debug, Copy, Clone, PartialEq)]
pub enum UsbIpDirection {
    Out = 0,
    In = 1,
}

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

impl USBIPCommandHeader {
    /// Returns the USBIP header from the command header
    pub fn get_header(&self) -> USBIPHeaderBasic {
        match self {
            USBIPCommandHeader::CmdSubmit(cmd) => cmd.base,
            USBIPCommandHeader::CmdUnlink(cmd) => cmd.base,
        }
    }
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
    /// usbip_header_basic, ‘command’ shall be 0x00000001
    #[packed_field(bytes = "0..=19")]
    pub base: USBIPHeaderBasic,
    /// transfer_flags: possible values depend on the USBIP_URB transfer_flags.
    /// Refer to include/uapi/linux/usbip.h and USB Request Block (URB). Refer
    /// to usbip_pack_cmd_submit() and tweak_transfer_flags() in
    /// drivers/usb/usbip/ usbip_common.c.
    #[packed_field(bytes = "20..=23", endian = "msb")]
    pub transfer_flags: Integer<u32, packed_bits::Bits<32>>,
    /// transfer_buffer_length: use URB transfer_buffer_length
    #[packed_field(bytes = "24..=27", endian = "msb")]
    pub transfer_buffer_length: Integer<i32, packed_bits::Bits<32>>,
    /// start_frame: use URB start_frame; initial frame for ISO transfer; shall
    /// be set to 0 if not ISO transfer
    #[packed_field(bytes = "28..=31", endian = "msb")]
    pub start_frame: Integer<i32, packed_bits::Bits<32>>,
    /// number_of_packets: number of ISO packets; shall be set to 0xffffffff if
    /// not ISO transfer
    #[packed_field(bytes = "32..=35", endian = "msb")]
    pub number_of_packets: Integer<i32, packed_bits::Bits<32>>,
    /// interval: maximum time for the request on the server-side host controller
    #[packed_field(bytes = "36..=39", endian = "msb")]
    pub interval: Integer<i32, packed_bits::Bits<32>>,
    /// setup: data bytes for USB setup, filled with zeros if not used.
    #[packed_field(bytes = "40..=47", endian = "msb")]
    pub setup: SetupRequest,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "48")]
pub struct USBIPHeaderRetSubmit {
    /// usbip_header_basic, ‘command’ shall be 0x00000003
    #[packed_field(bytes = "0..=19")]
    pub base: USBIPHeaderBasic,
    /// status: zero for successful URB transaction, otherwise some kind of
    /// error happened.
    #[packed_field(bytes = "20..=23", endian = "msb")]
    pub status: Integer<i32, packed_bits::Bits<32>>,
    /// actual_length: number of URB data bytes; use URB actual_length
    #[packed_field(bytes = "24..=27", endian = "msb")]
    pub actual_length: Integer<i32, packed_bits::Bits<32>>,
    /// start_frame: use URB start_frame; initial frame for ISO transfer; shall
    /// be set to 0 if not ISO transfer
    #[packed_field(bytes = "28..=31", endian = "msb")]
    pub start_frame: Integer<i32, packed_bits::Bits<32>>,
    /// number_of_packets: number of ISO packets; shall be set to 0xffffffff if
    /// not ISO transfer
    #[packed_field(bytes = "32..=35", endian = "msb")]
    pub number_of_packets: Integer<i32, packed_bits::Bits<32>>,
    /// error_count
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
    /// command
    #[packed_field(bytes = "0..=3", endian = "msb")]
    pub command: Integer<u32, packed_bits::Bits<32>>,
    /// seqnum: sequential number that identifies requests and corresponding
    /// responses; incremented per connection
    #[packed_field(bytes = "4..=7", endian = "msb")]
    pub seqnum: Integer<u32, packed_bits::Bits<32>>,
    /// devid: specifies a remote USB device uniquely instead of busnum and
    /// devnum; for client (request), this value is ((busnum << 16) | devnum);
    /// for server (response), this shall be set to 0
    #[packed_field(bytes = "8..=11", endian = "msb")]
    pub devid: Integer<u32, packed_bits::Bits<32>>,
    /// only used by client, for server this shall be 0
    #[packed_field(bytes = "12..=15", endian = "msb", ty = "enum")]
    pub direction: UsbIpDirection,
    /// ep: endpoint number only used by client, for server this shall be 0; for
    /// UNLINK, this shall be 0
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

/// Representation of a virtual USB port from the vhci-hcd "status" property
#[derive(Debug, Clone, Default)]
pub struct VirtualUsbPort {
    pub hub: String,
    pub port: u8,
    pub status: u8,
    pub speed: u8,
    pub device: u32,
    pub sock_fd: u32,
    pub local_bus_id: String,
}

impl TryFrom<&str> for VirtualUsbPort {
    type Error = &'static str;

    /// Try to build a [VirtualUsbPort] from the given line of the "status"
    /// property of the vhci-hcd device.
    ///
    /// E.g.
    ///   hub port sta spd dev      sockfd local_busid
    ///   hs  0000 004 000 00000000 000000 0-0
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut port = VirtualUsbPort::default();
        let mut parts = value.split_whitespace();

        // hub
        let Some(hub) = parts.next().map(|part| part.to_string()) else {
            return Err("Unable to parse hub value");
        };
        port.hub = hub;

        // port
        let Some(port_num) = parts.next().and_then(|num| num.parse::<u8>().ok()) else {
            return Err("Unable to parse port number");
        };
        port.port = port_num;

        // status
        let Some(status) = parts.next().and_then(|value| value.parse::<u8>().ok()) else {
            return Err("Unable to parse port status");
        };
        port.status = status;

        // speed
        let Some(speed) = parts.next().and_then(|spd| spd.parse::<u8>().ok()) else {
            return Err("Unable to parse port speed");
        };
        port.speed = speed;

        // dev
        let Some(dev) = parts.next().and_then(|dev| dev.parse::<u32>().ok()) else {
            return Err("Unable to parse port device");
        };
        port.device = dev;

        // sockfd
        let Some(fd) = parts.next().and_then(|fd| fd.parse::<u32>().ok()) else {
            return Err("Unable to parse port socket file descriptor");
        };
        port.sock_fd = fd;

        // local_busid
        let Some(id) = parts.next().map(|id| id.to_string()) else {
            return Err("Unable to parse local bus id");
        };
        port.local_bus_id = id;

        Ok(port)
    }
}

/// Driver for interfacing with the sysfs API for vhci-hcd.
#[derive(Default)]
pub struct Driver {
    /* /sys/devices/platform/vhci_hcd */
    hc_device: Option<Device>,
    _n_controllers: i32,
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
        #[cfg(feature = "log")]
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
        #[cfg(feature = "log")]
        log::debug!("attach data: {data}");

        // Attach the device
        device.set_attribute_value("attach", data)?;
        #[cfg(feature = "log")]
        log::debug!("attached port: {port}");

        Ok(())
    }

    /// Returns a list of all USB ports from the virtual USB hub
    pub fn get_ports(&self) -> Result<Vec<VirtualUsbPort>, Box<dyn Error>> {
        let Some(ref device) = self.hc_device else {
            return Err("Device driver has not been opened".into());
        };

        // Read the "status" property and convert it to a string to parse
        let result = device.attribute_value("status");
        if result.is_none() {
            return Err("Unable to find status attribute".into());
        }
        let status = result.unwrap().to_string_lossy().to_string();
        #[cfg(feature = "log")]
        log::debug!("Status: {status:?}");

        // Prepare the vector of ports based on ports available
        let nports = self.get_nports()?;
        let mut ports = Vec::with_capacity(nports as usize);

        // Parse each line of the status output and create a VirtualUsbPort
        // E.g.
        //   hub port sta spd dev      sockfd local_busid
        //   hs  0000 004 000 00000000 000000 0-0
        //   hs  0001 004 000 00000000 000000 0-0
        //   ..
        for line in status.lines() {
            if line.starts_with("hub") {
                continue;
            }

            let port = match VirtualUsbPort::try_from(line) {
                Ok(port) => port,
                Err(e) => {
                    #[cfg(feature = "log")]
                    log::warn!("Failed to parse port from status: {e:?}");
                    continue;
                }
            };

            #[cfg(feature = "log")]
            log::debug!("Found port: {port:?}");
            ports.push(port);
        }

        Ok(ports)
    }

    /// Returns the next available USB port on the virtual USB hub
    pub fn get_next_port_number(&self) -> Result<u8, Box<dyn Error>> {
        let ports = self.get_ports()?;
        for port in ports {
            if port.status == 4 {
                return Ok(port.port);
            }
        }

        Err("Unable to find available port".into())
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
