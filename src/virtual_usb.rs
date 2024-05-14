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
        Configuration, DescriptorType, DeviceClass, DeviceDescriptor, DeviceQualifierDescriptor,
        LangId, Recipient, RecipientMask, Request, SelfPowered, SetupRequest, StringDescriptor,
        Type, TypeMask,
    },
    usbip::{
        Driver, USBDeviceSpeed, USBIPCommandHeader, USBIPHeaderBasic, USBIPHeaderCmdSubmit,
        USBIPHeaderCmdUnlink, USBIPHeaderInit, USBIPHeaderRetSubmit, USBIPHeaderRetUnlink,
        USBIPReplyHeader, USBIP_CMD_SIZE, USBIP_CMD_SUBMIT, USBIP_CMD_UNLINK, USBIP_DIR_IN,
        USBIP_DIR_OUT, USBIP_RET_SUBMIT, USBIP_RET_UNLINK,
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
    payload_length: usize,
}

/// Replies sent over usbip unix socket
#[derive(Debug)]
pub struct Reply {
    header: USBIPReplyHeader,
    payload: Vec<u8>,
}

/// USB Transfer
#[derive(Debug)]
pub struct Xfer {
    /// Endpoint
    ep: u8,
    /// Setup
    setup: SetupRequest,
    /// USB Transfer data
    data: Vec<u8>,
}

/// Virtual USB Device
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

        // TODO: Implement find free port
        let port = 0;
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
                self.handle_command_unlink(cmd);
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
        let standard_type = header.setup.bm_request_type & TypeMask == Type::Standard as u8;

        // Handle standard requests automatically
        if standard_type {
            self.handle_command_submit_ep0_standard_request(cmd, header.setup)?;
            return Ok(None);
        }

        // Otherwise, handle as a regular endpoint command
        if let Some(mut xfer) = self.handle_command_submit_epX(cmd)? {
            // Populate the setupReq member, since it's always expected for ep==0
            xfer.setup = header.setup;
            return Ok(Some(xfer));
        }

        Ok(None)
    }

    /// Handle command submit to any other USB endpoint.
    fn handle_command_submit_epX(&self, cmd: &Command) -> Result<Option<Xfer>, Box<dyn Error>> {
        log::debug!("handle submit epX");
        todo!()
    }

    /// Handle unlinking
    fn handle_command_unlink(&self, cmd: &Command) {
        log::debug!("handle unlink");
        //
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

        // NOTE: We only support requests to the device for now
        let recipient = req.bm_request_type & RecipientMask;
        if recipient != Recipient::Device.as_u8() {
            return Err(format!("invalid recipient: {}", recipient).into());
        }

        // Handle the command based on the direction
        match header.base.direction.to_primitive() {
            // IN command (data from device->host)
            USBIP_DIR_IN => match req.b_request {
                Request::GetStatus => {
                    log::debug!("USB Request: GetStatus");
                    let Some(config) = self.current_config.as_ref() else {
                        return Err("No active configuration".to_string().into());
                    };
                    let mut reply = 0;
                    let bm_attributes = config.conf_desc.bm_attributes;

                    // If self-powered, bit 0 is 1
                    let self_powered = bm_attributes & SelfPowered;
                    if self_powered == 1 {
                        reply |= 1;
                    }
                    let data: [u8; 4] = reply.to_msb_bytes();

                    // Write the reply
                    self.reply(cmd, &data, 0)?;
                    Ok(())
                }
                Request::GetDescriptor => {
                    log::debug!("USB Request: GetDescriptor");
                    // Get the descriptor type
                    let desc_type = (req.w_value.to_primitive() & 0xFF00) >> 8;
                    let Some(desc_type) = DescriptorType::from_primitive(desc_type as u8) else {
                        return Err(format!("Invalid descriptor type: {desc_type}").into());
                    };
                    let desc_idx = req.w_value.to_primitive() & 0x00FF;
                    let desc_idx = desc_idx as usize;

                    // Get the reply data based on the descriptor type
                    let data = match desc_type {
                        DescriptorType::Device => {
                            log::debug!("USB request GetDescriptor Device");
                            self.info.device_desc.pack_to_vec()?
                        }
                        DescriptorType::Configuration => {
                            log::debug!("USB request GetDescriptor Configuration");
                            let Some(config_desc) = self.info.configs.get(desc_idx) else {
                                return Err(format!(
                                    "Invalid Configuration descriptor index: {desc_idx}"
                                )
                                .into());
                            };
                            let config = config_desc as &Configuration;
                            config.pack_to_vec()?
                        }
                        DescriptorType::String => {
                            log::debug!("USB request GetDescriptor String");
                            let Some(string_desc) = self.info.string_descs.get(desc_idx) else {
                                return Err(format!(
                                    "Invalid Configuration descriptor index: {desc_idx}"
                                )
                                .into());
                            };
                            let string_desc = string_desc as &StringDescriptor;
                            string_desc.pack_to_vec()?
                        }
                        DescriptorType::DeviceQualifier => {
                            log::debug!("USB request GetDescriptor DeviceQualifier");
                            self.info.device_qualifier_desc.pack_to_vec()?
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

                    // Write the reply
                    self.reply(cmd, data.as_slice(), status)?;
                    Ok(())
                }
                Request::SetConfiguration => {
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
            USBIP_DIR_OUT => {
                let payload_len = header.transfer_buffer_length.to_primitive();
                if payload_len != 0 {
                    return Err("Unexpected payload for EP0 standard request".into());
                }

                match req.b_request {
                    Request::SetConfiguration => {
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
            _ => Err("Unknown direction in header".to_string().into()),
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
                match header.direction.to_primitive() {
                    USBIP_DIR_IN => {
                        if data.is_empty() {
                            return Err("No data to send IN reply".into());
                        }
                    }
                    USBIP_DIR_OUT => {
                        // TODO: Double check this
                    }
                    _ => {}
                }

                // Set the payload if this is an IN command
                let mut payload = Vec::with_capacity(data.len());
                if header.direction.to_primitive() == USBIP_DIR_IN {
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
        // Write the message header to the socket
        let result = match reply.header {
            USBIPReplyHeader::RetSubmit(submit) => self.socket.write(&submit.pack()?),
            USBIPReplyHeader::RetUnlink(unlink) => self.socket.write(&unlink.pack()?),
        };
        if let Err(e) = result {
            return Err(format!("Failed to write message header: {e:?}").into());
        }
        if reply.payload.is_empty() {
            return Ok(());
        }

        // Write the message payload to the socket
        if let Err(e) = self.socket.write(reply.payload.as_slice()) {
            return Err(format!("Failed to write message payload: {e:?}").into());
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

        // Unpack the appropriate header based on the command
        let header = match header.base.command.to_primitive() {
            USBIP_CMD_SUBMIT => USBIPCommandHeader::CmdSubmit(USBIPHeaderCmdSubmit::unpack(&buf)?),
            USBIP_CMD_UNLINK => USBIPCommandHeader::CmdUnlink(USBIPHeaderCmdUnlink::unpack(&buf)?),
            _ => {
                return Err("Unknown USBIP command".into());
            }
        };

        // TODO: Remove this
        match header {
            USBIPCommandHeader::CmdSubmit(header) => log::debug!("{header}"),
            USBIPCommandHeader::CmdUnlink(header) => log::debug!("{header}"),
        }

        // Build the command based on the header
        let mut cmd = match header {
            USBIPCommandHeader::CmdSubmit(submit) => {
                // Set the payload length if this is data coming from the host
                let mut payload_length = 0;
                if submit.base.direction.to_primitive() == USBIP_DIR_OUT {
                    payload_length = submit.transfer_buffer_length.to_primitive() as usize;
                }
                Command {
                    header,
                    payload: Vec::with_capacity(payload_length),
                    payload_length,
                }
            }
            USBIPCommandHeader::CmdUnlink(_) => Command {
                header,
                payload: Vec::new(),
                payload_length: 0,
            },
        };

        // Read the payload if one exists
        if cmd.payload_length > 0 {
            log::debug!("Reading payload");
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
        self.info.string_descs.push(manufacturer.into());
        let mut idx = self.info.string_descs.len() - 1;
        if idx == 0 {
            idx = 1;
        }
        self.info.device_desc.i_manufacturer = idx as u8;
        self
    }

    /// Set the product string for the device
    pub fn product(&mut self, product: &str) -> &mut Self {
        self.info.string_descs.push(product.into());
        let mut idx = self.info.string_descs.len() - 1;
        if idx == 0 {
            idx = 1;
        }
        self.info.device_desc.i_product = idx as u8;
        self
    }

    /// Set the serial number string for the device
    pub fn serial(&mut self, serial: &str) -> &mut Self {
        self.info.string_descs.push(serial.into());
        let mut idx = self.info.string_descs.len() - 1;
        if idx == 0 {
            idx = 1;
        }
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
