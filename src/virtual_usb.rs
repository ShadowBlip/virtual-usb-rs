use std::{
    error::Error,
    io::{Read, Write},
    os::fd::AsFd,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    thread,
};

use packed_struct::{types::SizedInteger, PackedStruct, PackedStructSlice};
use socketpair::{socketpair_stream, SocketpairStream};

use crate::{
    usb::{
        ConfigurationDescriptor, DeviceDescriptor, DeviceQualifierDescriptor, Recipient,
        RecipientMask, Request, RequestType, SetupRequest, Type, TypeMask,
    },
    usbip::{
        Driver, USBDeviceSpeed, USBIPHeader, USBIPHeaderCmdSubmit, USBIPHeaderCmdUnlink,
        USBIPHeaderInit, USBIP_CMD_SIZE, USBIP_CMD_SUBMIT, USBIP_CMD_UNLINK, USBIP_DIR_OUT,
    },
};

/// Virtual USB Device descriptors
pub struct Info {
    pub device_desc: DeviceDescriptor,
    pub device_qualifier_desc: DeviceQualifierDescriptor,
    pub config_descs: Vec<ConfigurationDescriptor>,
    pub string_descs: Vec<u8>,
}

/// Commands sent over usbip unix socket
#[derive(Debug)]
pub struct Command {
    header: USBIPHeader,
    payload: Vec<u8>,
    payload_length: usize,
}

/// Replies sent over usbip unix socket
#[derive(Debug)]
pub struct Reply {
    header: USBIPHeader,
    payload: Vec<u8>,
    payload_length: usize,
}

/// Transfer
#[derive(Debug)]
pub struct Xfer {
    ep: u8,
    setup: SetupRequest,
    data: Vec<u8>,
}

/// Virtual USB Device
pub struct VirtualUSBDevice {
    pub info: Info,
    pub port: Option<u8>,
    pub replies: Option<Sender<Reply>>,
    pub commands: Option<Receiver<Command>>,
}

impl VirtualUSBDevice {
    /// Create a new Virtual USB device with the given standard USB descriptors
    pub fn new(info: Info) -> Self {
        Self {
            info,
            port: None,
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
        let writer_tx_clone = writer_tx.clone();
        self.replies = Some(writer_tx);
        let (reader_tx, reader_rx) = channel();
        self.commands = Some(reader_rx);

        // Spawn read and write threads
        let read_socket = socket.try_clone()?;
        thread::spawn(move || {
            println!("Spawning read handler");
            let mut handler = ReadHandler::new(read_socket, reader_tx, writer_tx_clone);
            handler.run();
        });
        let write_socket = socket.try_clone()?;
        thread::spawn(move || {
            println!("Spawning write handler");
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
    pub fn read(&self) -> Result<Option<Xfer>, Box<dyn Error>> {
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

    fn handle_command(&self, cmd: &Command) -> Result<Option<Xfer>, Box<dyn Error>> {
        match cmd.header {
            USBIPHeader::CmdSubmit(header) => {
                if header.base.ep.to_primitive() == 0 {
                    return self.handle_command_submit_ep0(cmd);
                } else {
                    return Ok(self.handle_command_submit_epx(cmd));
                }
            }
            USBIPHeader::CmdUnlink(_) => self.handle_command_unlink(cmd),
        }

        Ok(None)
    }

    fn handle_command_submit_ep0(&self, cmd: &Command) -> Result<Option<Xfer>, Box<dyn Error>> {
        println!("handle submit out");
        let USBIPHeader::CmdSubmit(header) = cmd.header else {
            return Err("Invalid header for submit command".into());
        };
        let standard_type = header.setup.bm_request_type & TypeMask == Type::Standard as u8;

        // Handle standard requests
        if standard_type {
            self.handle_command_submit_ep0_standard_request(cmd, header.setup)?;
            return Ok(None);
        }

        // Otherwise, handle as a regular endpoint command
        if let Some(mut xfer) = self.handle_command_submit_epx(cmd) {
            // Populate the setupReq member, since it's always expected for ep==0
            xfer.setup = header.setup;
            return Ok(Some(xfer));
        }

        Ok(None)
    }

    fn handle_command_submit_epx(&self, cmd: &Command) -> Option<Xfer> {
        todo!()
    }

    fn handle_command_unlink(&self, cmd: &Command) {
        //
    }

    fn handle_command_submit_ep0_standard_request(
        &self,
        cmd: &Command,
        req: SetupRequest,
    ) -> Result<(), Box<dyn Error>> {
        println!("handle submit ep0 standard request");
        let USBIPHeader::CmdSubmit(header) = cmd.header else {
            return Err("Invalid header for submit command".into());
        };

        // We only support requests to the device for now
        let recipient = req.bm_request_type & RecipientMask;
        if recipient != Recipient::Device.as_u8() {
            return Err(format!("invalid recipient: {}", recipient).into());
        }

        match header.base.direction.to_primitive() {
            // IN command (data from device->host)
            USBIP_DIR_IN => match req.b_request {
                Request::GetStatus => {
                    println!("USB Request: GetStatus");
                    // TODO: Check if the USB device has a config descriptor
                    let reply = 0;
                    // If self-powered, bit 0 is 1
                }
                Request::ClearFeature => todo!(),
                Request::_Reserved0 => todo!(),
                Request::SetFeature => todo!(),
                Request::_Reserved1 => todo!(),
                Request::SetAddress => todo!(),
                Request::GetDescriptor => todo!(),
                Request::SetDescriptor => todo!(),
                Request::GetConfiguration => todo!(),
                Request::SetConfiguration => todo!(),
                Request::GetInterface => todo!(),
                Request::SetInterface => todo!(),
                Request::SynchFrame => todo!(),
            },
            // OUT command (data from host->device)
            USBIP_DIR_OUT => {}
            _ => {
                return Err("Unknown direction in header".to_string().into());
            }
        }
        todo!()
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
                    println!("Channel closed. Stopping write handler.");
                    break;
                }
            };

            // Write the reply to the unix socket
            if let Err(e) = self.write(reply) {
                println!("Error writing reply: {e:?}");
                break;
            }
        }
    }

    /// Write the given reply to the unix socket
    fn write(&mut self, reply: Reply) -> Result<(), Box<dyn Error>> {
        // Write the message header to the socket
        let result = match reply.header {
            USBIPHeader::CmdSubmit(submit) => self.socket.write(&submit.pack()?),
            USBIPHeader::CmdUnlink(unlink) => self.socket.write(&unlink.pack()?),
        };
        if let Err(e) = result {
            return Err(format!("Failed to write message header: {e:?}").into());
        }

        // Write the message payload to the socket
        if let Err(e) = self.socket.write(reply.payload.as_slice()) {
            return Err(format!("Failed to write message payload: {e:?}").into());
        }

        Ok(())
    }
}

/// [ReadHandler] handles reading data from the usbip socket. Standard USB
/// requests (such as GET_STATUS, GET_DESCRIPTOR, SET_CONFIGURATION requests,
/// and all IN transfers) will automatically be handled and replied to using
/// the given write handler channel.
struct ReadHandler {
    socket: SocketpairStream,
    virt_device: Sender<Command>,
    write_handler: Sender<Reply>,
}

impl ReadHandler {
    fn new(
        socket: SocketpairStream,
        device: Sender<Command>,
        write_handler: Sender<Reply>,
    ) -> Self {
        Self {
            socket,
            virt_device: device,
            write_handler,
        }
    }

    /// Run the read handler
    fn run(&mut self) {
        loop {
            // Read commands from the unix socket
            let cmd = match self.read() {
                Ok(cmd) => cmd,
                Err(e) => {
                    println!("Error reading commands: {e:?}");
                    break;
                }
            };

            // Send the command to the virtual USB device
            if self.virt_device.send(cmd).is_err() {
                println!("Channel closed. Stopping read handler.");
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
        println!("{header}");

        // Unpack the appropriate header based on the command
        let header = match header.base.command.to_primitive() {
            USBIP_CMD_SUBMIT => USBIPHeader::CmdSubmit(USBIPHeaderCmdSubmit::unpack(&buf)?),
            USBIP_CMD_UNLINK => USBIPHeader::CmdUnlink(USBIPHeaderCmdUnlink::unpack(&buf)?),
            _ => {
                return Err("Unknown USBIP command".into());
            }
        };

        // Build the command based on the header
        let mut cmd = match header {
            USBIPHeader::CmdSubmit(submit) => {
                // Set the payload length if this is data coming from the host
                let mut payload_length = 0;
                if submit.base.direction.to_primitive() == USBIP_DIR_OUT {
                    payload_length = submit.transfer_buffer_length.to_primitive() as usize;
                }
                Command {
                    header,
                    payload: Vec::new(),
                    payload_length,
                }
            }
            USBIPHeader::CmdUnlink(_) => Command {
                header,
                payload: Vec::new(),
                payload_length: 0,
            },
        };

        // Read the payload if one exists
        if cmd.payload_length > 0 {
            println!("Reading payload");
            let payload_buf = cmd.payload.as_mut_slice();
            self.socket.read_exact(payload_buf)?;
        }
        println!("Cmd: {cmd:?}");

        // Handle the command

        Ok(cmd)
    }

    /// Build a reply from the given command
    fn reply(&self, cmd: &Command) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
