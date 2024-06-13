use std::{
    error::Error,
    io::{Read, Write},
    os::fd::AsFd,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    thread,
};

use packed_struct::{
    types::{Integer, IntegerAsBytes, SizedInteger},
    PackedStruct, PackedStructSlice, PrimitiveEnum,
};
use socketpair::{socketpair_stream, SocketpairStream};

use crate::{
    usb::{
        hid::{HidDescriptorType, HidGetDescriptorRequest},
        Configuration, DescriptorType, DeviceClass, DeviceDescriptor, DeviceQualifierDescriptor,
        Interface, LangId, Recipient, SetupRequest, StandardRequest, StringDescriptor,
        ENDPOINT_MAX_COUNT, SELF_POWERED,
    },
    usbip::{
        Driver, USBDeviceSpeed, USBIPCommandHeader, USBIPHeaderBasic, USBIPHeaderCmdSubmit,
        USBIPHeaderCmdUnlink, USBIPHeaderInit, USBIPHeaderRetSubmit, USBIPHeaderRetUnlink,
        USBIPReplyHeader, UsbIpDirection, USBIP_CMD_SIZE, USBIP_CMD_SUBMIT, USBIP_CMD_UNLINK,
        USBIP_RET_SUBMIT, USBIP_RET_UNLINK,
    },
};

/// Virtual USB Device descriptors
#[derive(Debug, Clone)]
pub struct Info {
    pub device_desc: DeviceDescriptor,
    pub device_qualifier_desc: DeviceQualifierDescriptor,
    pub configs: Vec<Configuration>,
    pub string_descs: Vec<StringDescriptor>,
}

/// Commands sent over usbip unix socket
#[derive(Debug)]
pub struct Command {
    header: USBIPCommandHeader,
    payload: Vec<u8>,
}

impl Command {
    pub fn get_header(&self) -> USBIPHeaderBasic {
        self.header.get_header()
    }
}

/// Replies sent over usbip unix socket
#[derive(Debug)]
pub struct Reply {
    header: USBIPReplyHeader,
    payload: Vec<u8>,
}

impl Reply {
    /// Create a new reply from the given transfer and data payload
    pub fn from_xfer(xfer: Xfer, data: &[u8]) -> Self {
        let cmd = xfer.cmd;
        let header = cmd.base;

        // Set the payload if this is an IN command (device -> host)
        let mut payload = Vec::with_capacity(data.len());
        if header.direction == UsbIpDirection::In {
            payload = data.to_vec();
        }

        Self {
            header: USBIPReplyHeader::RetSubmit(USBIPHeaderRetSubmit {
                base: USBIPHeaderBasic {
                    command: Integer::from_primitive(USBIP_RET_SUBMIT),
                    seqnum: header.seqnum,
                    devid: header.devid,
                    direction: header.direction,
                    ep: header.ep,
                },
                status: Integer::from_primitive(0),
                actual_length: Integer::from_primitive(data.len() as i32),
                start_frame: Integer::from_primitive(0),
                number_of_packets: Integer::from_primitive(0),
                error_count: Integer::from_primitive(0),
            }),
            payload,
        }
    }
}

/// USB Transfer
#[derive(Debug, Clone)]
pub struct Xfer {
    /// Endpoint
    pub ep: u8,
    /// USB Transfer data
    pub data: Vec<u8>,
    /// Setup
    cmd: USBIPHeaderCmdSubmit,
}

impl Xfer {
    /// Returns the USB setup request for this transfer. If this in an IN
    /// request, the setup request will be empty.
    pub fn header(&self) -> Option<SetupRequest> {
        if self.cmd.setup.is_empty() {
            return None;
        }
        Some(self.cmd.setup)
    }

    /// Returns the direction of the transfer. OUT requests (host -> device)
    /// mean that the host is writing data to the device, and IN requests
    /// (device -> host) means the device has an opportunity to send data to the
    /// host.
    pub fn direction(&self) -> UsbIpDirection {
        self.cmd.base.direction
    }
}

/// Virtual USB Device
#[derive(Debug)]
pub struct VirtualUSBDevice {
    /// Information about the virtual USB device
    pub info: Info,
    /// The virtual USB port number that this device is connected to
    pub port: Option<u8>,
    /// The currently active configuration descriptor
    current_config: Option<Configuration>,
    /// Sender for writing replies to the USBIP unix socket
    replies: Option<Sender<Reply>>,
    /// Receiver for reading commands from the USBIP unix socket
    commands: Option<Receiver<Command>>,
}

impl VirtualUSBDevice {
    /// Create a new Virtual USB device with the given standard USB descriptors
    pub fn new(info: Info) -> Self {
        Self {
            info,
            port: None,
            current_config: None,
            replies: None,
            commands: None,
        }
    }

    /// Start the VirtualUSBDevice
    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let bcd_usb = self.info.device_desc.bcd_usb.to_primitive();
        let speed = VirtualUSBDevice::speed_from_bcd_usb(bcd_usb);

        // Create a unix socket pair. One side is used by the vhci-hcd kernel
        // module, and the other is used by the VirtualUSBDevice.
        let (socket, vhci_hcd_socket) = socketpair_stream()?;
        let fd = vhci_hcd_socket.as_fd();

        // Open the vhci-hcd driver
        let mut driver = Driver::new();
        driver.open()?;

        // Find the next available port on the virtual USB hub
        let port = driver.get_next_port_number()?;
        self.port = Some(port);

        // Attach the device to the port
        let devid = 1;
        if let Err(e) = driver.attach_device2(port, fd, devid, speed) {
            return Err(format!("Failed to attach device: {e:?}").into());
        }

        // Create a set of channels for communicating with the read/write threads
        let (writer_tx, writer_rx) = channel();
        self.replies = Some(writer_tx);
        let (reader_tx, reader_rx) = channel();
        self.commands = Some(reader_rx);

        // Spawn read and write threads
        let read_socket = socket.try_clone()?;
        thread::spawn(move || {
            log::debug!("Spawning read handler");
            let mut handler = ReadHandler::new(read_socket, reader_tx);
            handler.run();
        });
        let write_socket = socket.try_clone()?;
        thread::spawn(move || {
            log::debug!("Spawning write handler");
            let mut handler = WriteHandler::new(write_socket, writer_rx);
            handler.run();
        });

        Ok(())
    }

    /// Tear down the virtual USB device
    pub fn stop(&mut self) {
        // Drop the channels to force the read/write threads to stop
        self.replies = None;
        self.commands = None;
    }

    /// To handle USB transfers, call read(). Before read() returns,
    /// VirtualUSBDevice will automatically handle standard USB requests
    /// (such as GET_STATUS, GET_DESCRIPTOR, SET_CONFIGURATION requests, and all
    /// IN transfers), and will only return from read() when there's an OUT
    /// transfer that it can't handle itself. The returned Xfer object
    /// represents the USB OUT transfer to be performed, and contains these
    /// fields:
    ///
    ///  - ep: the transfer's endpoint
    ///  - setupReq: if ep==0, the Setup packet
    ///  - data: the payload data
    ///  - len: the length of data
    pub fn read(&mut self) -> Result<Option<Xfer>, Box<dyn Error>> {
        let Some(commands) = self.commands.as_ref() else {
            return Err("Device is not started".to_string().into());
        };

        // Check for any command messages from the read thread.
        match commands.try_recv() {
            Ok(cmd) => self.handle_command(&cmd),
            Err(err) => match err {
                TryRecvError::Empty => Ok(None),
                TryRecvError::Disconnected => Err("Read thread stopped".to_string().into()),
            },
        }
    }

    /// Read from the virtual USB device in a blocking way.
    /// To handle USB transfers, call read(). Before read() returns,
    /// VirtualUSBDevice will automatically handle standard USB requests
    /// (such as GET_STATUS, GET_DESCRIPTOR, SET_CONFIGURATION requests, and all
    /// IN transfers), and will only return from read() when there's an OUT
    /// transfer that it can't handle itself. The returned Xfer object
    /// represents the USB OUT transfer to be performed, and contains these
    /// fields:
    ///
    ///  - ep: the transfer's endpoint
    ///  - setupReq: if ep==0, the Setup packet
    ///  - data: the payload data
    ///  - len: the length of data
    pub fn blocking_read(&mut self) -> Result<Option<Xfer>, Box<dyn Error>> {
        let Some(commands) = self.commands.as_ref() else {
            return Err("Device is not started".to_string().into());
        };

        // Check for any command messages from the read thread.
        match commands.recv() {
            Ok(cmd) => self.handle_command(&cmd),
            Err(_) => Err("read thread stopped".to_string().into()),
        }
    }

    /// To write data to an IN endpoint, call write() with the endpoint, data,
    /// and length.
    pub fn write(&self, reply: Reply) -> Result<(), Box<dyn Error>> {
        let Some(replies) = self.replies.as_ref() else {
            return Err("Device is not started".to_string().into());
        };
        replies.send(reply)?;

        Ok(())
    }

    /// Handle the given USB command. Standard USB transfers are automatically
    /// handled. If it is not possible to handle, an [Xfer] will be returned
    /// so it can be handled at another layer.
    fn handle_command(&mut self, cmd: &Command) -> Result<Option<Xfer>, Box<dyn Error>> {
        match cmd.header {
            USBIPCommandHeader::CmdSubmit(header) => {
                if header.base.ep.to_primitive() == 0 {
                    self.handle_command_submit_ep0(cmd)
                } else {
                    self.handle_command_submit_epX(cmd)
                }
            }
            USBIPCommandHeader::CmdUnlink(_) => {
                self.handle_command_unlink(cmd)?;
                Ok(None)
            }
        }
    }

    /// Handle command submit to endpoint 0. Endpoint 0 (zero), the default
    /// endpoint, is always assumed to be a control endpoint and never has a
    /// descriptor.
    fn handle_command_submit_ep0(&mut self, cmd: &Command) -> Result<Option<Xfer>, Box<dyn Error>> {
        log::debug!("handle submit ep0");
        let USBIPCommandHeader::CmdSubmit(header) = cmd.header else {
            return Err("Invalid header for submit command".into());
        };
        let standard_type = header.setup.is_standard();

        // Handle standard requests automatically
        if standard_type {
            self.handle_command_submit_ep0_standard_request(cmd, header.setup)?;
            return Ok(None);
        }

        // Otherwise, handle as a regular endpoint command
        if let Some(mut xfer) = self.handle_command_submit_epX(cmd)? {
            // Populate the setupReq member, since it's always expected for ep==0
            xfer.cmd = header;
            return Ok(Some(xfer));
        }

        Ok(None)
    }

    /// Handle command submit to any other USB endpoint.
    #[allow(non_snake_case)]
    fn handle_command_submit_epX(&self, cmd: &Command) -> Result<Option<Xfer>, Box<dyn Error>> {
        log::debug!("handle submit epX");
        let USBIPCommandHeader::CmdSubmit(header) = cmd.header else {
            return Err("Invalid header for submit command".into());
        };
        match header.base.direction {
            // OUT command (data from host->device)
            UsbIpDirection::Out => self.handle_command_submit_epX_out(cmd),
            // IN command (data from device->host)
            UsbIpDirection::In => self.handle_command_submit_epX_in(cmd),
        }
    }

    /// Handle command submit OUT to any other USB endpoint.
    #[allow(non_snake_case)]
    fn handle_command_submit_epX_out(&self, cmd: &Command) -> Result<Option<Xfer>, Box<dyn Error>> {
        log::debug!("handle submit epX OUT");
        let USBIPCommandHeader::CmdSubmit(header) = cmd.header else {
            return Err("Invalid header for submit command".into());
        };
        let ep_idx = header.base.ep.to_primitive();
        log::debug!("handle submit epX OUT {ep_idx}");
        if ep_idx >= ENDPOINT_MAX_COUNT as u32 {
            return Err("Invalid endpoint index".into());
        }

        // Let host know that we received the data
        self.reply(cmd, &[], cmd.payload.len() as i32)?;
        let xfer = Xfer {
            // TODO: Double check this
            ep: ep_idx as u8,
            // TODO: Can we move?
            data: cmd.payload.clone(),
            cmd: header,
        };

        Ok(Some(xfer))
    }

    /// Handle command submit IN to any other USB endpoint.
    #[allow(non_snake_case)]
    fn handle_command_submit_epX_in(&self, cmd: &Command) -> Result<Option<Xfer>, Box<dyn Error>> {
        log::debug!("handle submit epX IN");
        let USBIPCommandHeader::CmdSubmit(header) = cmd.header else {
            return Err("Invalid header for submit command".into());
        };
        let ep_idx = header.base.ep.to_primitive();
        log::debug!("handle submit epX IN {ep_idx}");
        if ep_idx >= ENDPOINT_MAX_COUNT as u32 {
            return Err("Invalid endpoint index".into());
        }

        // This is an IN transfer that must be handled by user code
        let xfer = Xfer {
            ep: ep_idx as u8,
            data: cmd.payload.clone(),
            cmd: header,
        };

        Ok(Some(xfer))
    }

    /// Send data to the host for an IN endpoint
    //fn send_data_for_in_endpoint(&self, _cmd: &Command) -> Result<(), Box<dyn Error>> {
    //    // TODO: Implement this
    //    log::error!("Not implemented!");

    //    // TODO: Figure this Out
    //    // This is a GET_REPORT request for HID
    //    // bm_request_type_direction | bits   0:0   | 0b1                | "In"
    //    //      bm_request_type_kind | bits   1:2   | 0b01               | "Class"
    //    // bm_request_type_recipient | bits   3:7   | 0b00001            | "Interface"
    //    //                 b_request | bits   8:15  | 0b00000001         | "ClearFeature"
    //    //                   w_value | bits  16:31  | 0b0000000000000011 | "768" (Report ID: 0, ReportType: Feature (3))
    //    //                   w_index | bits  32:47  | 0b0000001000000000 | "2"
    //    //                  w_length | bits  48:63  | 0b0100000000000000 | "64"

    //    //self.reply(cmd, data, status);
    //    //todo!();
    //    Ok(())
    //}

    /// Handle unlinking
    fn handle_command_unlink(&self, cmd: &Command) -> Result<(), Box<dyn Error>> {
        log::debug!("handle unlink");
        let USBIPCommandHeader::CmdUnlink(_) = cmd.header else {
            return Err("Invalid header for unlink command".into());
        };

        // TODO: Do we need to do more?

        let status = -104; // 104 == ECONNRESET
        self.reply(cmd, &[], status)
    }

    /// Handle standard requests to endpoint zero
    fn handle_command_submit_ep0_standard_request(
        &mut self,
        cmd: &Command,
        req: SetupRequest,
    ) -> Result<(), Box<dyn Error>> {
        log::debug!("handle submit ep0 standard request");
        let USBIPCommandHeader::CmdSubmit(header) = cmd.header else {
            return Err("Invalid header for submit command".into());
        };

        // Handle the request based on recipient
        let direction = header.base.direction;
        let recipient = req.bm_request_type_recipient;
        match recipient {
            Recipient::Device => {
                self.handle_command_submit_ep0_standard_request_for_device(cmd, req, direction)
            }
            Recipient::Interface => {
                self.handle_command_submit_ep0_standard_request_for_iface(cmd, req, direction)
            }
            _ => {
                let err = format!("Unhandled recipient: {:?}", recipient);
                Err(err.into())
            }
        }
    }

    /// Handle standard device requests to endpoint zero
    fn handle_command_submit_ep0_standard_request_for_device(
        &mut self,
        cmd: &Command,
        req: SetupRequest,
        direction: UsbIpDirection,
    ) -> Result<(), Box<dyn Error>> {
        log::debug!("handle submit ep0 standard request for device");
        let USBIPCommandHeader::CmdSubmit(header) = cmd.header else {
            return Err("Invalid header for submit command".into());
        };

        // Handle the command based on the direction
        match direction {
            // IN command (data from device->host)
            UsbIpDirection::In => match req.b_request {
                StandardRequest::GetStatus => {
                    log::debug!("USB Request: GetStatus");
                    let Some(config) = self.current_config.as_ref() else {
                        return Err("No active configuration".to_string().into());
                    };
                    let mut reply = 0;
                    let bm_attributes = config.conf_desc.bm_attributes;

                    // If self-powered, bit 0 is 1
                    let self_powered = bm_attributes & SELF_POWERED;
                    if self_powered == 1 {
                        reply |= 1;
                    }
                    let data: [u8; 4] = reply.to_msb_bytes();

                    // Write the reply
                    self.reply(cmd, &data, 0)?;
                    Ok(())
                }
                StandardRequest::GetDescriptor => {
                    log::debug!("USB Request: GetDescriptor");
                    // Get the descriptor type
                    let desc_type = (req.w_value.to_primitive() & 0xFF00) >> 8;
                    let Some(desc_type) = DescriptorType::from_primitive(desc_type as u8) else {
                        return Err(format!("Invalid descriptor type: {desc_type}").into());
                    };
                    let desc_idx = req.w_value.to_primitive() & 0x00FF;
                    let desc_idx = desc_idx as usize;

                    // Get the reply data based on the descriptor type
                    let mut data = match desc_type {
                        DescriptorType::Device => {
                            log::debug!("USB request GetDescriptor Device");
                            log::debug!("Device: {}", self.info.device_desc);
                            self.info.device_desc.pack_to_vec()?
                        }
                        DescriptorType::Configuration => {
                            log::debug!("USB request GetDescriptor Configuration {desc_idx}");
                            let Some(config_desc) = self.info.configs.get(desc_idx) else {
                                return Err(format!(
                                    "Invalid Configuration descriptor index: {desc_idx}"
                                )
                                .into());
                            };
                            let config = config_desc as &Configuration;
                            log::debug!("Config: {config}");
                            config.pack_to_vec()?
                        }
                        DescriptorType::String => {
                            log::debug!("USB request GetDescriptor String {desc_idx}");
                            let Some(string_desc) = self.info.string_descs.get(desc_idx) else {
                                return Err(format!(
                                    "Invalid Configuration descriptor index: {desc_idx}"
                                )
                                .into());
                            };
                            let string_desc = string_desc as &StringDescriptor;
                            log::debug!("Got string: {}", string_desc.to_string());
                            string_desc.pack_to_vec()?
                        }
                        DescriptorType::DeviceQualifier => {
                            log::debug!("USB request GetDescriptor DeviceQualifier");
                            self.info.device_qualifier_desc.pack_to_vec()?
                        }
                        DescriptorType::Debug => {
                            log::debug!("USB request GetDescriptor Debug");
                            vec![]
                        }
                        _ => {
                            // Unsupported descriptor type
                            return Err(format!(
                                "Unsupported descriptor type: {:?}",
                                req.b_request
                            )
                            .into());
                        }
                    };

                    // Get the status of the reply
                    let status = if data.is_empty() { 1 } else { 0 };

                    // Truncate the data to the expected length
                    data.truncate(req.w_length.to_primitive() as usize);

                    // Write the reply
                    self.reply(cmd, data.as_slice(), status)?;
                    Ok(())
                }
                StandardRequest::SetConfiguration => {
                    log::debug!("USB Request: SetConfiguration");
                    let config_val = req.w_value.to_primitive() & 0x00FF;
                    let mut ok = false;
                    for config in self.info.configs.iter() {
                        if config_val as u8 == config.conf_desc.b_configuration_value {
                            // TODO: Don't copy
                            self.current_config = Some(config.clone());
                            ok = true;
                        }
                    }
                    if !ok {
                        return Err(format!("Invalid Configuration value: {config_val}").into());
                    }

                    // Write the reply
                    self.reply(cmd, vec![].as_slice(), 0)?;
                    Ok(())
                }
                _ => Err(
                    format!("Invalid device->host standard request: {:?}", req.b_request).into(),
                ),
            },

            // OUT command (data from host->device)
            UsbIpDirection::Out => {
                let payload_len = header.transfer_buffer_length.to_primitive();
                if payload_len != 0 {
                    return Err("Unexpected payload for EP0 standard request".into());
                }

                match req.b_request {
                    StandardRequest::SetConfiguration => {
                        log::debug!("USB Request: SetConfiguration");
                        let config_val = req.w_value.to_primitive() & 0x00FF;
                        let mut ok = false;
                        for config in self.info.configs.iter() {
                            if config_val as u8 == config.conf_desc.b_configuration_value {
                                // TODO: Don't copy
                                self.current_config = Some(config.clone());
                                ok = true;
                            }
                        }
                        if !ok {
                            return Err(format!("Invalid Configuration value: {config_val}").into());
                        }

                        // Write the reply
                        self.reply(cmd, vec![].as_slice(), 0)?;
                        Ok(())
                    }
                    _ => Err(
                        format!("Invalid host->device standard request: {:?}", req.b_request)
                            .into(),
                    ),
                }
            }
        }
    }

    /// Handle standard device requests to endpoint zero
    fn handle_command_submit_ep0_standard_request_for_iface(
        &mut self,
        cmd: &Command,
        req: SetupRequest,
        direction: UsbIpDirection,
    ) -> Result<(), Box<dyn Error>> {
        log::debug!("handle submit ep0 standard request for interface");

        match direction {
            // IN command (data from device->host)
            UsbIpDirection::In => match req.b_request {
                StandardRequest::GetDescriptor => {
                    log::debug!("USB Request: GetDescriptor");
                    // Get the interface descriptor this request is for
                    let Some(config) = self.current_config.as_ref() else {
                        let err = "No current configuration set to get interface descriptor";
                        return Err(err.into());
                    };

                    // Get the interface descriptor from the config
                    let iface_idx = req.w_index.to_primitive() as usize;
                    let Some(iface) = config.interfaces.get(iface_idx) else {
                        let err = format!("No interface exists in config with index {iface_idx}");
                        return Err(err.into());
                    };

                    // Handle the request based on the interface type
                    match iface {
                        Interface::Hid(hid_iface) => {
                            let hid_req = HidGetDescriptorRequest::from(req);
                            log::debug!("GetDescriptor for HID: {hid_req}");
                            let desc_idx = hid_req.b_descriptor_index as usize;

                            // Handle the request based on type
                            match hid_req.b_descriptor_type {
                                HidDescriptorType::Hid => {
                                    todo!()
                                }
                                HidDescriptorType::Report => {
                                    let Some(desc) = hid_iface.report_descriptors.get(desc_idx)
                                    else {
                                        let err = format!(
                                            "No report descriptor exists with index {desc_idx}"
                                        );
                                        return Err(err.into());
                                    };

                                    // Write the reply
                                    self.reply(cmd, desc, 0)?;
                                    Ok(())
                                }
                                HidDescriptorType::Physical => {
                                    todo!()
                                }
                            }
                        }
                    }
                }
                _ => todo!(),
            },
            // OUT command (data from host->device)
            UsbIpDirection::Out => todo!(),
        }
    }

    /// Reply to the given command and write it to the USBIP unix socket.
    fn reply(&self, cmd: &Command, data: &[u8], status: i32) -> Result<(), Box<dyn Error>> {
        // Get the write channel to send replies
        let Some(replies) = self.replies.as_ref() else {
            return Err("Write thread is not running to send replies".into());
        };

        // Get the base header from the command
        let header = match cmd.header {
            USBIPCommandHeader::CmdSubmit(submit) => submit.base,
            USBIPCommandHeader::CmdUnlink(unlink) => unlink.base,
        };

        // Build a reply based on the type of command
        let reply = match header.command.to_primitive() {
            USBIP_CMD_SUBMIT => {
                // Validate our arguments for SUBMIT replies:
                //   - For IN transfers, either we're sending data (len>0) and have a
                //   valid data pointer
                //     (data!=null), or we're not sending data (len==0)
                //   - For OUT transfers, we can't respond with any data, but the `len`
                //   argument is used
                //     to populate `actual_length` -- the amount of data sent to the
                //     device
                match header.direction {
                    UsbIpDirection::In => {
                        if data.is_empty() {
                            return Err("No data to send IN reply".into());
                        }
                    }
                    UsbIpDirection::Out => {
                        // TODO: Double check this
                    }
                }

                // Set the payload if this is an IN command
                let mut payload = Vec::with_capacity(data.len());
                if header.direction == UsbIpDirection::In {
                    payload = data.to_vec();
                }

                // Build a reply
                Reply {
                    header: USBIPReplyHeader::RetSubmit(USBIPHeaderRetSubmit {
                        base: USBIPHeaderBasic {
                            command: Integer::from_primitive(USBIP_RET_SUBMIT),
                            seqnum: header.seqnum,
                            devid: header.devid,
                            direction: header.direction,
                            ep: header.ep,
                        },
                        status: Integer::from_primitive(0),
                        actual_length: Integer::from_primitive(data.len() as i32),
                        start_frame: Integer::from_primitive(0),
                        number_of_packets: Integer::from_primitive(0),
                        error_count: Integer::from_primitive(0),
                    }),
                    payload,
                }
            }
            USBIP_CMD_UNLINK => Reply {
                header: USBIPReplyHeader::RetUnlink(USBIPHeaderRetUnlink {
                    base: USBIPHeaderBasic {
                        command: Integer::from_primitive(USBIP_RET_UNLINK),
                        seqnum: header.seqnum,
                        devid: header.devid,
                        direction: header.direction,
                        ep: header.ep,
                    },
                    status: Integer::from_primitive(status),
                }),
                payload: Vec::with_capacity(0),
            },
            _ => return Err("Unknown command to reply to".into()),
        };

        // Send the reply to the write thread
        replies.send(reply)?;

        Ok(())
    }

    /// Returns the USB speed from the given bcdUSB value in the device
    /// descriptor.
    fn speed_from_bcd_usb(bcd_usb: u16) -> u32 {
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

/// [WriteHandler] waits for write commands from the [VirtualUSBDevice] and
/// writes the data to the usbip socket.
struct WriteHandler {
    socket: SocketpairStream,
    virt_device: Receiver<Reply>,
}

impl WriteHandler {
    fn new(socket: SocketpairStream, device: Receiver<Reply>) -> Self {
        Self {
            socket,
            virt_device: device,
        }
    }

    /// Run the write handler
    fn run(&mut self) {
        loop {
            // Wait for writes from the virtual USB device.
            let reply = match self.virt_device.recv() {
                Ok(msg) => msg,
                Err(_) => {
                    log::debug!("Channel closed. Stopping write handler.");
                    break;
                }
            };

            // Write the reply to the unix socket
            if let Err(e) = self.write(reply) {
                log::debug!("Error writing reply: {e:?}");
                break;
            }
        }
    }

    /// Write the given reply to the unix socket
    fn write(&mut self, reply: Reply) -> Result<(), Box<dyn Error>> {
        log::debug!("Got reply to write");
        // Write the message header to the socket
        let result = match reply.header {
            USBIPReplyHeader::RetSubmit(submit) => {
                log::debug!("Write: {submit}");
                self.socket.write(&submit.pack()?)
            }
            USBIPReplyHeader::RetUnlink(unlink) => {
                log::debug!("Write: {unlink}");
                self.socket.write(&unlink.pack()?)
            }
        };
        if let Err(e) = result {
            return Err(format!("Failed to write message header: {e:?}").into());
        }
        if reply.payload.is_empty() {
            return Ok(());
        }

        // Write the message payload to the socket if one exists
        log::debug!("Writing payload with size: {}", reply.payload.len());
        log::debug!("Payload: {:x?}", reply.payload.as_slice());
        match self.socket.write(reply.payload.as_slice()) {
            Ok(bytes_written) => log::debug!("Wrote {bytes_written} bytes"),
            Err(e) => {
                return Err(format!("Failed to write message payload: {e:?}").into());
            }
        }

        Ok(())
    }
}

/// [ReadHandler] handles reading data from the usbip socket and sending it
/// to the [VirtualUSBDevice].
struct ReadHandler {
    socket: SocketpairStream,
    virt_device: Sender<Command>,
}

impl ReadHandler {
    fn new(socket: SocketpairStream, device: Sender<Command>) -> Self {
        Self {
            socket,
            virt_device: device,
        }
    }

    /// Run the read handler
    fn run(&mut self) {
        loop {
            // Read commands from the unix socket
            let cmd = match self.read() {
                Ok(cmd) => cmd,
                Err(e) => {
                    log::debug!("Error reading commands: {e:?}");
                    break;
                }
            };

            // Send the command to the virtual USB device
            if self.virt_device.send(cmd).is_err() {
                log::debug!("Channel closed. Stopping read handler.");
                break;
            }
        }
    }

    /// Read messages from the unix socket
    fn read(&mut self) -> Result<Command, Box<dyn Error>> {
        // Read data from the device into a buffer
        let mut buf = [0; USBIP_CMD_SIZE];

        // Read commands from the socket
        if let Err(e) = self.socket.read_exact(&mut buf) {
            return Err(format!("Failed to read from VHCI-HCD socket: {e:?}").into());
        }

        let header = USBIPHeaderInit::unpack(&buf)?;
        log::debug!("Got header: {header:?}");

        // Unpack the appropriate header based on the command
        let header = match header.base.command.to_primitive() {
            USBIP_CMD_SUBMIT => USBIPCommandHeader::CmdSubmit(USBIPHeaderCmdSubmit::unpack(&buf)?),
            USBIP_CMD_UNLINK => USBIPCommandHeader::CmdUnlink(USBIPHeaderCmdUnlink::unpack(&buf)?),
            _ => {
                let cmd_num = header.base.command.to_primitive();
                let err = format!("Unknown USBIP command: {cmd_num}");
                return Err(err.into());
            }
        };

        // TODO: Remove this
        log::debug!("Read command from socket");
        match header {
            USBIPCommandHeader::CmdSubmit(header) => {
                log::debug!("{header}");
                if !header.setup.is_empty() {
                    log::debug!("{}", header.setup);
                }
            }
            USBIPCommandHeader::CmdUnlink(header) => {
                log::debug!("{header}");
            }
        }

        // Build the command based on the header
        let mut cmd = match header {
            USBIPCommandHeader::CmdSubmit(submit) => {
                // Set the payload length if this is data coming from the host
                let mut payload_length = 0;
                if submit.base.direction == UsbIpDirection::Out {
                    payload_length = submit.transfer_buffer_length.to_primitive() as usize;
                }
                Command {
                    header,
                    payload: Vec::with_capacity(payload_length),
                }
            }
            USBIPCommandHeader::CmdUnlink(_) => Command {
                header,
                payload: Vec::with_capacity(0),
            },
        };

        // Read the payload if one exists
        let payload_size = cmd.payload.capacity();
        if payload_size > 0 {
            log::debug!("Reading payload with size: {}", payload_size);
            cmd.payload.resize(payload_size, 0);
            let payload_buf = cmd.payload.as_mut_slice();
            self.socket.read_exact(payload_buf)?;
        }
        log::debug!("Cmd: {cmd:?}");

        Ok(cmd)
    }
}

/// [VirtualUSBDevice] builder for constructing a new custom virtual USB device
pub struct VirtualUSBDeviceBuilder {
    info: Info,
}

impl VirtualUSBDeviceBuilder {
    /// Create a new virtual usb device builder
    pub fn new(vendor_id: u16, product_id: u16) -> Self {
        Self {
            info: Info {
                device_desc: DeviceDescriptor::new(vendor_id, product_id),
                device_qualifier_desc: DeviceQualifierDescriptor::new(),
                configs: Vec::new(),
                string_descs: Vec::new(),
            },
        }
    }

    /// Construct the new virtual USB device
    pub fn build(&self) -> VirtualUSBDevice {
        VirtualUSBDevice::new(self.info.clone())
    }

    /// Set the device class for the device
    pub fn class(&mut self, class: DeviceClass) -> &mut Self {
        self.info.device_desc.b_device_class = class as u8;
        self
    }

    /// Set the device subclass for the device
    pub fn subclass(&mut self, subclass: u8) -> &mut Self {
        self.info.device_desc.b_device_sub_class = subclass;
        self
    }

    /// Add the given supported languages
    pub fn supported_langs(&mut self, langs: Vec<LangId>) -> &mut Self {
        self.info.string_descs.insert(0, langs.into());
        self
    }

    /// Add the given configuration
    pub fn configuration(&mut self, config: Configuration) -> &mut Self {
        self.info.configs.push(config);
        self.info.device_desc.b_num_configurations = self.info.configs.len() as u8;
        self
    }

    /// Set the manufacturer string for the device
    pub fn manufacturer(&mut self, manufacturer: &str) -> &mut Self {
        let idx = self.info.string_descs.len();
        self.info.string_descs.push(manufacturer.into());
        self.info.device_desc.i_manufacturer = idx as u8;
        self
    }

    /// Set the product string for the device
    pub fn product(&mut self, product: &str) -> &mut Self {
        let idx = self.info.string_descs.len();
        self.info.string_descs.push(product.into());
        self.info.device_desc.i_product = idx as u8;
        self
    }

    /// Set the serial number string for the device
    pub fn serial(&mut self, serial: &str) -> &mut Self {
        let idx = self.info.string_descs.len();
        self.info.string_descs.push(serial.into());
        self.info.device_desc.i_serial_number = idx as u8;
        self
    }

    /// Add the given string descriptors (max 127 bytes each)
    pub fn strings(&mut self, strings: Vec<&str>) -> &mut Self {
        for string in strings {
            self.info.string_descs.push(string.into());
        }
        self
    }

    /// Set the device's max packet size
    pub fn max_packet_size(&mut self, size: u8) -> &mut Self {
        self.info.device_desc.b_max_packet_size_0 = size;
        self
    }
}
