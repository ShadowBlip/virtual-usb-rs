pub mod usb;
pub mod usbip;
pub mod vhci_hcd;
pub mod virtual_usb;

use std::{thread, time::Duration};

use usb::LangId;

use crate::{
    usb::{
        hid::{HidInterfaceBuilder, HidSubclass, InterfaceProtocol},
        ConfigurationBuilder, DeviceClass, Direction, EndpointBuilder, SynchronizationType,
        TransferType, UsageType,
    },
    vhci_hcd::load_vhci_hcd,
    virtual_usb::VirtualUSBDeviceBuilder,
};

/// Report descriptor for the Deck mouse device (/dev/hidraw0 on Deck)
pub const MOUSE_DESCRIPTOR: [u8; 65] = [
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x02, // Usage (Mouse)
    0xa1, 0x01, // Collection (Application)
    0x09, 0x01, //  Usage (Pointer)
    0xa1, 0x00, //  Collection (Physical)
    0x05, 0x09, //   Usage Page (Button)
    0x19, 0x01, //   Usage Minimum (1)
    0x29, 0x02, //   Usage Maximum (2)
    0x15, 0x00, //   Logical Minimum (0)
    0x25, 0x01, //   Logical Maximum (1)
    0x75, 0x01, //   Report Size (1)
    0x95, 0x02, //   Report Count (2)
    0x81, 0x02, //   Input (Data,Var,Abs)
    0x75, 0x06, //   Report Size (6)
    0x95, 0x01, //   Report Count (1)
    0x81, 0x01, //   Input (Cnst,Arr,Abs)
    0x05, 0x01, //   Usage Page (Generic Desktop)
    0x09, 0x30, //   Usage (X)
    0x09, 0x31, //   Usage (Y)
    0x15, 0x81, //   Logical Minimum (-127)
    0x25, 0x7f, //   Logical Maximum (127)
    0x75, 0x08, //   Report Size (8)
    0x95, 0x02, //   Report Count (2)
    0x81, 0x06, //   Input (Data,Var,Rel)
    0x95, 0x01, //   Report Count (1)
    0x09, 0x38, //   Usage (Wheel)
    0x81, 0x06, //   Input (Data,Var,Rel)
    0x05, 0x0c, //   Usage Page (Consumer Devices)
    0x0a, 0x38, 0x02, //   Usage (AC Pan)
    0x95, 0x01, //   Report Count (1)
    0x81, 0x06, //   Input (Data,Var,Rel)
    0xc0, //  End Collection
    0xc0, // End Collection
];

/// Report descriptor for the Deck keyboard device (/dev/hidraw1 on Deck)
pub const KEYBOARD_DESCRIPTOR: [u8; 39] = [
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x06, // Usage (Keyboard)
    0xa1, 0x01, // Collection (Application)
    0x05, 0x07, //  Usage Page (Keyboard)
    0x19, 0xe0, //  Usage Minimum (224)
    0x29, 0xe7, //  Usage Maximum (231)
    0x15, 0x00, //  Logical Minimum (0)
    0x25, 0x01, //  Logical Maximum (1)
    0x75, 0x01, //  Report Size (1)
    0x95, 0x08, //  Report Count (8)
    0x81, 0x02, //  Input (Data,Var,Abs)
    0x81, 0x01, //  Input (Cnst,Arr,Abs)
    0x19, 0x00, //  Usage Minimum (0)
    0x29, 0x65, //  Usage Maximum (101)
    0x15, 0x00, //  Logical Minimum (0)
    0x25, 0x65, //  Logical Maximum (101)
    0x75, 0x08, //  Report Size (8)
    0x95, 0x06, //  Report Count (6)
    0x81, 0x00, //  Input (Data,Arr,Abs)
    0xc0, // End Collection
];

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
    // Configuration values can be obtained with "sudo lsusb -v"
    let mut virtual_device = VirtualUSBDeviceBuilder::new(0x28de, 0x1205)
        .class(DeviceClass::UseInterface)
        .supported_langs(vec![LangId::EnglishUnitedStates])
        .manufacturer("Valve Software")
        .product("Steam Controller")
        .max_packet_size(64)
        .configuration(
            ConfigurationBuilder::new()
                .max_power(500)
                // Mouse
                .interface(
                    HidInterfaceBuilder::new()
                        .country_code(0)
                        .protocol(InterfaceProtocol::Mouse)
                        .subclass(HidSubclass::None)
                        .report_descriptor(65)
                        .endpoint_descriptor(
                            EndpointBuilder::new()
                                .address_num(1)
                                .direction(Direction::In)
                                .transfer_type(TransferType::Interrupt)
                                .sync_type(SynchronizationType::NoSynchronization)
                                .usage_type(UsageType::Data)
                                .max_packet_size(0x0008)
                                .build(),
                        )
                        .build(),
                )
                // Keyboard
                .interface(
                    HidInterfaceBuilder::new()
                        .country_code(33)
                        .protocol(InterfaceProtocol::Keyboard)
                        .subclass(HidSubclass::Boot)
                        .report_descriptor(39)
                        .endpoint_descriptor(
                            EndpointBuilder::new()
                                .address_num(2)
                                .direction(Direction::In)
                                .transfer_type(TransferType::Interrupt)
                                .sync_type(SynchronizationType::NoSynchronization)
                                .usage_type(UsageType::Data)
                                .max_packet_size(0x0008)
                                .build(),
                        )
                        .build(),
                )
                // Controller
                .interface(
                    HidInterfaceBuilder::new()
                        .country_code(33)
                        .protocol(InterfaceProtocol::None)
                        .subclass(HidSubclass::None)
                        .report_descriptor(38)
                        .endpoint_descriptor(
                            EndpointBuilder::new()
                                .address_num(3)
                                .direction(Direction::In)
                                .transfer_type(TransferType::Interrupt)
                                .sync_type(SynchronizationType::NoSynchronization)
                                .usage_type(UsageType::Data)
                                .max_packet_size(0x0040)
                                .build(),
                        )
                        .build(),
                )
                // CDC
                //.interface(HidInterfaceBuilder::new().build())
                // CDC Data
                //.interface(HidInterfaceBuilder::new().build())
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
        if let Some(xfer) = xfer {
            log::warn!("Got unhandled xfer: {:?}", xfer);
        }
        thread::sleep(Duration::from_millis(10));
    }

    thread::sleep(Duration::from_secs(5));
    println!("Dropping USB device");
    drop(virtual_device);
    thread::sleep(Duration::from_secs(5));
    println!("Finished!");
}
