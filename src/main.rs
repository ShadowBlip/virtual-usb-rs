pub mod usb;
pub mod usbip;
pub mod vhci_hcd;
pub mod virtual_usb;

use std::{thread, time::Duration};

use usb::LangId;

use crate::{
    usb::{ConfigurationBuilder, HidInterfaceBuilder},
    vhci_hcd::load_vhci_hcd,
    virtual_usb::VirtualUSBDeviceBuilder,
};

fn main() {
    use simple_logger::SimpleLogger;
    SimpleLogger::new().init().unwrap();

    if let Err(e) = load_vhci_hcd() {
        log::error!("{:?}", e);
        return;
    }

    // Create a virtual Steam Deck Controller
    let mut virtual_device = VirtualUSBDeviceBuilder::new(0x28de, 0x1205)
        .supported_langs(vec![LangId::EnglishUnitedStates])
        .manufacturer("Valve Software")
        .product("Steam Controller")
        .max_packet_size(64)
        .configuration(
            ConfigurationBuilder::new()
                .max_power(500)
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
