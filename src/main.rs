pub mod usb;
pub mod usbip;
pub mod vhci_hcd;
pub mod virtual_usb;

use std::{thread, time::Duration};

use packed_struct::prelude::*;
use usb::{ConfigurationDescriptor, InterfaceDescriptor};

use crate::{
    usb::{DeviceDescriptor, DeviceQualifierDescriptor},
    virtual_usb::{Info, VirtualUSBDevice},
};

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
pub struct Configuration {
    #[packed_field(element_size_bytes = "9")]
    config_desc: ConfigurationDescriptor,

    #[packed_field(element_size_bytes = "9")]
    iface0_desc: InterfaceDescriptor,
}

fn main() {
    use simple_logger::SimpleLogger;
    SimpleLogger::new().init().unwrap();

    //if let Err(e) = load_vhci_hcd() {
    //    log::error!("{:?}", e);
    //    return;
    //}

    // Create a new virtual usb device
    let info = Info {
        device_desc: DeviceDescriptor::new(0x1234, 0x5678),
        device_qualifier_desc: DeviceQualifierDescriptor::new(),
        config_descs: Vec::new(),
        string_descs: Vec::new(),
    };
    let mut virtual_device = VirtualUSBDevice::new(info);
    if let Err(e) = virtual_device.start() {
        println!("Error: {e:?}");
    }

    virtual_device.read();

    thread::sleep(Duration::from_secs(5));
    println!("Finished!");
}
