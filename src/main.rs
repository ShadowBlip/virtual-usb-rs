pub mod usb;
pub mod usbip;
pub mod vhci_hcd;
pub mod virtual_usb;

use std::{thread, time::Duration};

use usb::LangId;

use crate::{
    usb::{
        hid::{HidInterfaceBuilder, HidSubclass, InterfaceProtocol},
        ConfigurationBuilder, DeviceClass,
    },
    vhci_hcd::load_vhci_hcd,
    virtual_usb::VirtualUSBDeviceBuilder,
};

/// Valve Software Steam Controller
/// Report descriptor for the Deck Controller device (/dev/hidraw3 on Deck)
pub const CONTROLLER_DESCRIPTOR: [u8; 38] = [
    0x06, 0xff, 0xff, // Usage Page (Vendor Usage Page 0xffff)
    0x09, 0x01, // Usage (Vendor Usage 0x01)
    0xa1, 0x01, // Collection (Application)
    0x09, 0x02, //  Usage (Vendor Usage 0x02)
    0x09, 0x03, //  Usage (Vendor Usage 0x03)
    0x15, 0x00, //  Logical Minimum (0)
    0x26, 0xff, 0x00, //  Logical Maximum (255)
    0x75, 0x08, //  Report Size (8)
    0x95, 0x40, //  Report Count (64)
    0x81, 0x02, //  Input (Data,Var,Abs)
    0x09, 0x06, //  Usage (Vendor Usage 0x06)
    0x09, 0x07, //  Usage (Vendor Usage 0x07)
    0x15, 0x00, //  Logical Minimum (0)
    0x26, 0xff, 0x00, //  Logical Maximum (255)
    0x75, 0x08, //  Report Size (8)
    0x95, 0x40, //  Report Count (64)
    0xb1, 0x02, //  Feature (Data,Var,Abs)
    0xc0, // End Collection
];

fn main() {
    use simple_logger::SimpleLogger;
    SimpleLogger::new().init().unwrap();

    if let Err(e) = load_vhci_hcd() {
        log::error!("{:?}", e);
        return;
    }

    // Create a virtual Steam Deck Controller
    let mut virtual_device = VirtualUSBDeviceBuilder::new(0x28de, 0x1205)
        .class(DeviceClass::UseInterface)
        .supported_langs(vec![LangId::EnglishUnitedStates])
        .manufacturer("Valve Software")
        .product("Steam Controller")
        .max_packet_size(64)
        .configuration(
            ConfigurationBuilder::new()
                .max_power(500)
                .interface(
                    HidInterfaceBuilder::new()
                        .country_code(33)
                        .protocol(InterfaceProtocol::None)
                        .subclass(HidSubclass::None)
                        .report_descriptor(&CONTROLLER_DESCRIPTOR)
                        .build(),
                )
                .interface(HidInterfaceBuilder::new().build())
                .interface(HidInterfaceBuilder::new().build())
                .interface(HidInterfaceBuilder::new().build())
                .interface(HidInterfaceBuilder::new().build())
                .build(),
        )
        .build();
    if let Err(e) = virtual_device.start() {
        println!("Error starting device: {e:?}");
        return;
    }

    loop {
        let xfer = match virtual_device.read() {
            Ok(xfer) => xfer,
            Err(e) => {
                log::error!("Error reading from device: {e:?}");
                break;
            }
        };
        thread::sleep(Duration::from_millis(10));
    }

    thread::sleep(Duration::from_secs(5));
    println!("Dropping USB device");
    drop(virtual_device);
    thread::sleep(Duration::from_secs(5));
    println!("Finished!");
}
