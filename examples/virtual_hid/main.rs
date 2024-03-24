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
        config_descs: todo!(),
        string_descs: Vec::new(),
    };
    let virtual_device = VirtualUSBDevice::new(info);

    println!("Finished!");
}
