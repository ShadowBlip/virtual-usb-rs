pub mod descriptor;
pub mod hid_report;

use std::{thread, time::Duration};

use packed_struct::{PackedStruct, PackedStructSlice};
use virtual_usb::{
    usb::{
        hid::{HidInterfaceBuilder, HidReportRequest, HidRequest, HidSubclass, InterfaceProtocol},
        ConfigurationBuilder, DeviceClass, Direction, EndpointBuilder, LangId, SynchronizationType,
        TransferType, Type, UsageType,
    },
    usbip::UsbIpDirection,
    vhci_hcd::load_vhci_hcd,
    virtual_usb::{Reply, VirtualUSBDeviceBuilder, Xfer},
};

use crate::{
    descriptor::{CONTROLLER_DESCRIPTOR, KEYBOARD_DESCRIPTOR, MOUSE_DESCRIPTOR},
    hid_report::{PackedInputDataReport, ReportType},
};

fn main() {
    use simple_logger::SimpleLogger;
    SimpleLogger::new().init().unwrap();

    // Ensure the vhci_hcd kernel module is loaded
    if let Err(e) = load_vhci_hcd() {
        log::error!("{:?}", e);
        return;
    }

    // Create a virtual Steam Deck Controller
    // Configuration values can be obtained from a real device with "sudo lsusb -v"
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
                        .report_descriptor(&MOUSE_DESCRIPTOR)
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
                        .report_descriptor(&KEYBOARD_DESCRIPTOR)
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
                        .report_descriptor(&CONTROLLER_DESCRIPTOR)
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

    let should_send_reports = true;
    let state = PackedInputDataReport::default();
    loop {
        // Read from the device
        let xfer = match virtual_device.blocking_read() {
            Ok(xfer) => xfer,
            Err(e) => {
                log::error!("Error reading from device: {e:?}");
                break;
            }
        };

        // Handle any non-standard transfers
        if let Some(xfer) = xfer {
            let reply = handle_xfer(xfer, &state, should_send_reports);

            // Write to the device if a reply is necessary
            if let Some(reply) = reply {
                if let Err(e) = virtual_device.write(reply) {
                    log::error!("Error writing reply: {e:?}");
                }
            }
        }

        //std::thread::sleep(Duration::from_millis(10));
    }

    thread::sleep(Duration::from_secs(5));
    println!("Dropping USB device");
    drop(virtual_device);
    thread::sleep(Duration::from_secs(5));
    println!("Finished!");
}

/// Handle any non-standard OUT transfers (host -> device)
fn handle_xfer(
    xfer: Xfer,
    state: &PackedInputDataReport,
    should_send_reports: bool,
) -> Option<Reply> {
    log::warn!("Got unhandled xfer: {:?}", xfer);
    let endpoint = xfer.ep;
    log::warn!("Request for endpoint: {endpoint}");

    match xfer.direction() {
        // TODO: Make our own direction enum
        UsbIpDirection::Out => {
            handle_xfer_out(xfer);
            None
        }
        UsbIpDirection::In => handle_xfer_in(xfer, state, should_send_reports),
    }
}

/// Handle any non-standard IN transfers (device -> host) for the gamepad iface
fn handle_xfer_in(
    xfer: Xfer,
    state: &PackedInputDataReport,
    should_send_reports: bool,
) -> Option<Reply> {
    // IN transfers do not have a setup request.
    let endpoint = xfer.ep;
    log::warn!("Got IN xfer: {xfer:?} for endpoint {endpoint}");

    // Create a reply based on the endpoint
    let reply = match endpoint {
        // Gamepad
        3 => {
            if should_send_reports {
                handle_xfer_in_gamepad(xfer, state)
            } else {
                Reply::from_xfer(xfer, &[])
            }
        }
        // All other endpoints, write empty data for now
        _ => Reply::from_xfer(xfer, &[]),
    };

    Some(reply)
}

fn handle_xfer_in_gamepad(xfer: Xfer, state: &PackedInputDataReport) -> Reply {
    // Pack the state
    let report_data = match state.pack() {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to pack input data report: {e:?}");
            return Reply::from_xfer(xfer, &[]);
        }
    };

    Reply::from_xfer(xfer, &report_data)
}

/// Handle any non-standard OUT transfers (host -> device) for the gamepad iface
fn handle_xfer_out(xfer: Xfer) {
    let Some(setup) = xfer.header() else {
        log::warn!("No setup request in OUT xfer");
        return;
    };

    // Only handle Class requests
    if setup.request_type() != Type::Class {
        log::warn!("Unknown request type");
        return;
    }

    // Interpret the setup request as an HID request
    let request = HidRequest::from(setup);

    match request {
        HidRequest::Unknown => {
            log::warn!("Unknown HID request!");
        }
        HidRequest::SetIdle(req) => {
            log::warn!("SetIdle: {req}");
        }
        // The host wants to set the given report on the device
        HidRequest::SetReport(req) => {
            log::warn!("SetReport: {req}");
            let data = xfer.data;
            log::warn!("Got SetReport data: {data:?}");

            // The first byte contains the report type
            let Some(first_byte) = data.first() else {
                log::warn!("Unable to determine report type from empty report");
                return;
            };

            let Ok(report_type) = ReportType::try_from(*first_byte) else {
                log::warn!("Invalid report type: {first_byte}");
                return;
            };

            log::warn!("Got SetReport with type: {report_type:?}");
        }
        // Ignore other types of requests
        _ => {}
    }
}
