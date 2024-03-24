use std::{error::Error, os::fd::AsFd};

use packed_struct::{types::SizedInteger, PackedStruct};
use socketpair::{socketpair_stream, SocketpairStream};

use crate::{
    usb::{ConfigurationDescriptor, DeviceDescriptor, DeviceQualifierDescriptor},
    usbip::{Driver, USBDeviceSpeed},
};

/// Virtual USB Device descriptors
pub struct Info {
    pub device_desc: DeviceDescriptor,
    pub device_qualifier_desc: DeviceQualifierDescriptor,
    pub config_descs: Vec<ConfigurationDescriptor>,
    pub string_descs: Vec<u8>,
}

/// Virtual USB Device
pub struct VirtualUSBDevice {
    pub info: Info,
    pub port: Option<u8>,
    pub socket: Option<SocketpairStream>,
}

impl VirtualUSBDevice {
    pub fn new(info: Info) -> Self {
        Self {
            info,
            port: None,
            socket: None,
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let bcd_usb = self.info.device_desc.bcd_usb.to_primitive();
        let speed = self.speed_from_bcd_usb(bcd_usb);

        // Create a unix socket pair. One side is used by the vhci-hcd kernel
        // module, and the other is used by the VirtualUSBDevice.
        let (socket, vhci_hcd_socket) = socketpair_stream()?;
        self.socket = Some(socket);
        let fd = vhci_hcd_socket.as_fd();

        // Open the vhci-hcd driver
        let mut driver = Driver::new();
        driver.open()?;

        // TODO: Implement find free port
        let port = 0;
        self.port = Some(port);

        // Attach the device to the port
        let devid = 1;
        driver.attach_device2(port, fd, devid, speed)?;

        Ok(())
    }

    /// Returns the USB speed from the given bcdUSB value in the device
    /// descriptor.
    fn speed_from_bcd_usb(&self, bcd_usb: u16) -> u32 {
        match bcd_usb {
            0x0100 => USBDeviceSpeed::USBSpeedFull as u32,
            0x0110 => USBDeviceSpeed::USBSpeedFull as u32,
            0x0200 => USBDeviceSpeed::USBSpeedHigh as u32,
            0x0300 => USBDeviceSpeed::USBSpeedSuper as u32,
            0x0310 => USBDeviceSpeed::USBSpeedSuperPlus as u32,
            0x0320 => USBDeviceSpeed::USBSpeedSuperPlus as u32,
            _ => USBDeviceSpeed::USBSpeedUnknown as u32,
        }
    }
}
