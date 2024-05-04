use std::{thread, time::Duration};

use virtual_usb::{
    usb::{DeviceDescriptor, DeviceQualifierDescriptor},
    virtual_usb::{Info, VirtualUSBDevice},
};

mod descriptor;

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
        configs: Vec::new(),
        string_descs: Vec::new(),
    };
    let mut virtual_device = VirtualUSBDevice::new(info);
    if let Err(e) = virtual_device.start() {
        println!("Error: {e:?}");
    }

    println!("Finished!");
}
