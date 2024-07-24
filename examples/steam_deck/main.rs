pub mod descriptor;
pub mod hid_report;

use std::{thread, time::Duration};

use packed_struct::{
    types::{Integer, SizedInteger},
    PackedStruct, PackedStructSlice,
};
use virtual_usb::{
    usb::{
        hid::{
            HidInterfaceBuilder, HidReportRequest, HidReportType, HidRequest, HidSubclass,
            InterfaceProtocol,
        },
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

struct DeckController {
    state: PackedInputDataReport,
    /// Steam will send 'SetReport' commands with a report type, so it can fetch
    /// a particular result with 'GetReport'
    current_report: ReportType,
    lizard_mode_enabled: bool,
    serial_number: String,
}

impl DeckController {
    fn new() -> Self {
        Self {
            state: PackedInputDataReport::default(),
            current_report: ReportType::InputData,
            lizard_mode_enabled: false,
            serial_number: "INPU7PLUMB3R".to_string(),
        }
    }

    /// Handle any non-standard transfers
    fn handle_xfer(&mut self, xfer: Xfer, should_send_reports: bool) -> Option<Reply> {
        match xfer.direction() {
            // TODO: Make our own direction enum
            UsbIpDirection::Out => {
                self.handle_xfer_out(xfer);
                None
            }
            UsbIpDirection::In => self.handle_xfer_in(xfer, should_send_reports),
        }
    }

    /// Handle any non-standard IN transfers (device -> host) for the gamepad iface
    fn handle_xfer_in(&self, xfer: Xfer, should_send_reports: bool) -> Option<Reply> {
        // IN transfers do not have a setup request.
        let endpoint = xfer.ep;

        // If a setup header exists, we need to reply to it.
        if let Some(setup) = xfer.header() {
            // Only handle Class requests
            if setup.request_type() != Type::Class {
                log::warn!("Unknown request type");
                return Some(Reply::from_xfer(xfer, &[]));
            }

            // Interpret the setup request as an HID request
            let request = HidRequest::from(setup);

            let reply = match request {
                HidRequest::Unknown => {
                    log::warn!("Unknown HID request!");
                    Reply::from_xfer(xfer, &[])
                }
                HidRequest::GetReport(req) => {
                    log::warn!("GetReport: {req}");
                    let interface = req.interface.to_primitive();
                    log::warn!("Got GetReport data for iface {interface}");
                    let report_type = req.report_type;

                    // Handle GetReport
                    match report_type {
                        HidReportType::Input => Reply::from_xfer(xfer, &[]),
                        HidReportType::Output => Reply::from_xfer(xfer, &[]),
                        HidReportType::Feature => {
                            // Reply based on the currently set report
                            match self.current_report {
                                ReportType::GetAttrib => {
                                    log::info!("Sending attribute data");
                                    // No idea what these bytes mean, but this is
                                    // what is sent from the real device.
                                    let data = [
                                        ReportType::GetAttrib as u8,
                                        0x2d,
                                        0x01,
                                        0x05,
                                        0x12,
                                        0x00,
                                        0x00,
                                        0x02,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x0a,
                                        0x2b,
                                        0x12,
                                        0xa9,
                                        0x62,
                                        0x04,
                                        0xad,
                                        0xf1,
                                        0xe4,
                                        0x65,
                                        0x09,
                                        0x2e,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x0b,
                                        0xa0,
                                        0x0f,
                                        0x00,
                                        0x00,
                                        0x0d,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x0c,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x0e,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                        0x00,
                                    ];
                                    Reply::from_xfer(xfer, &data)
                                }
                                ReportType::GetSerial => {
                                    // Reply with the serial number
                                    // [ReportType::GetSerial, 0x14, 0x01, ..serial?]?
                                    log::info!("Sending serial number: {}", self.serial_number);
                                    let mut data = vec![ReportType::GetSerial as u8, 0x14, 0x01];
                                    let mut serial_data = self.serial_number.as_bytes().to_vec();
                                    data.append(&mut serial_data);
                                    data.resize(64, 0);
                                    Reply::from_xfer(xfer, data.as_slice())
                                }
                                // Don't care about other types
                                _ => Reply::from_xfer(xfer, &[]),
                            }
                        }
                    }
                }
                // Ignore other types of requests
                _ => Reply::from_xfer(xfer, &[]),
            };

            return Some(reply);
        };

        // Create a reply based on the endpoint
        let reply = match endpoint {
            // Gamepad
            3 => {
                if should_send_reports {
                    self.handle_xfer_in_gamepad(xfer)
                } else {
                    Reply::from_xfer(xfer, &[])
                }
            }
            // All other endpoints, write empty data for now
            _ => {
                //log::info!("Got IN xfer request for endpoint: {endpoint}");
                Reply::from_xfer(xfer, &[])
            }
        };

        Some(reply)
    }

    // Handle IN transfers (device -> host) for the gamepad interface
    fn handle_xfer_in_gamepad(&self, xfer: Xfer) -> Reply {
        // Pack the state
        let report_data = match self.state.pack() {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to pack input data report: {e:?}");
                return Reply::from_xfer(xfer, &[]);
            }
        };

        Reply::from_xfer(xfer, &report_data)
    }

    /// Handle any non-standard OUT transfers (host -> device) for the gamepad iface.
    /// Out transfers do not have any replies.
    fn handle_xfer_out(&mut self, xfer: Xfer) {
        // OUT transfers (host -> device) are generally always to ep 0
        log::info!("Got OUT transfer for endpoint: {}", xfer.ep);

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
                let interface = req.interface.to_primitive();
                let data = xfer.data;
                log::warn!("Got SetReport data for iface {interface}: {data:?}");

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
                // https://github.com/libsdl-org/SDL/blob/f0363a0466f72655a1081fb96a90e1b9602ee571/src/joystick/hidapi/SDL_hidapi_steamdeck.c
                match report_type {
                    ReportType::InputData => (),
                    ReportType::SetMappings => (),
                    // ClearMappings gets called to take the controller out of lizard
                    // mode so that Steam can control it directly.
                    ReportType::ClearMappings => {
                        log::info!("Disabling lizard mode");
                        self.lizard_mode_enabled = false;
                    }
                    ReportType::GetMappings => (),
                    ReportType::GetAttrib => {
                        log::info!("Attribute requested");
                        self.current_report = ReportType::GetAttrib;
                    }
                    ReportType::GetAttribLabel => (),
                    // DefaultMappings sets the device in lizard mode, so it can run
                    // without Steam.
                    ReportType::DefaultMappings => {
                        log::info!("Setting lizard mode enabled");
                        self.lizard_mode_enabled = true;
                    }
                    ReportType::FactoryReset => (),
                    // When Steam boots up, it writes to a register with this data:
                    // Got SetReport data: [135, 3, 8, 7, 0, 0, 0, ...]
                    ReportType::WriteRegister => (),
                    ReportType::ClearRegister => (),
                    ReportType::ReadRegister => (),
                    ReportType::GetRegisterLabel => (),
                    ReportType::GetRegisterMax => (),
                    ReportType::GetRegisterDefault => (),
                    ReportType::SetMode => (),
                    ReportType::DefaultMouse => (),
                    ReportType::TriggerHapticPulse => (),
                    ReportType::RequestCommStatus => (),
                    // Configure the next GET_REPORT call to return the serial
                    // number.
                    ReportType::GetSerial => {
                        log::info!("Serial number requested");
                        self.current_report = ReportType::GetSerial;
                    }
                    ReportType::TriggerHapticCommand => (),
                    ReportType::TriggerRumbleCommand => (),
                }
            }
            // Ignore other types of requests
            _ => {}
        }
    }
}

fn main() {
    use simple_logger::SimpleLogger;
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

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
                // Mouse (iface 0)
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
                // Keyboard (iface 1)
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
                // Controller (iface 2)
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

    let mut interval = 0;
    let should_send_reports = true;
    let mut deck = DeckController::new();
    loop {
        // Increment the frame
        let frame = deck.state.frame.to_primitive();
        deck.state.frame = Integer::from_primitive(frame.wrapping_add(1));

        // Toggle the A button because FUN
        interval += 1;
        deck.state.b = interval > 5000;
        if interval > 10000 {
            interval = 0;
        }

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
            let reply = deck.handle_xfer(xfer, should_send_reports);

            // Write to the device if a reply is necessary
            if let Some(reply) = reply {
                if let Err(e) = virtual_device.write(reply) {
                    log::error!("Error writing reply: {e:?}");
                }
            }
        }

        std::thread::sleep(Duration::from_millis(1));
    }

    thread::sleep(Duration::from_secs(5));
    println!("Dropping USB device");
    drop(virtual_device);
    thread::sleep(Duration::from_secs(5));
    println!("Finished!");
}
