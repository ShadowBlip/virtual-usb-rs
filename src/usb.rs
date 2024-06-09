//! Reference:
//! https://github.com/toasterllc/Toastbox/blob/d3b1770c6816eb648ee2e0a754c2dd9c3bd5342f/USB.h

//#![allow(warnings)]
pub mod cdc;
pub mod hid;

use std::fmt::Display;

use packed_struct::prelude::*;

pub const ENDPOINT_MAX_COUNT_OUT: u8 = 16;
pub const ENDPOINT_MAX_COUNT_IN: u8 = 16;
pub const ENDPOINT_MAX_COUNT: u8 = 32;

/// Request type (bRequest)
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum StandardRequest {
    GetStatus = 0,
    ClearFeature = 1,
    _Reserved0 = 2,
    SetFeature = 3,
    _Reserved1 = 4,
    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SynchFrame = 12,
}

/// Request direction. This is always from the perspective of the host (i.e. host computer)
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum Direction {
    Out = 0,
    In = 1,
}

#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum Type {
    Standard = 0,
    Class = 1,
    Vendor = 2,
    Reserved = 3,
}

#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum Recipient {
    Device = 0x00,
    Interface = 0x01,
    Endpoint = 0x02,
    Other = 0x03,
}

// Configuration Characteristics
pub const REMOTE_WAKEUP: u8 = 1 << 5;
pub const SELF_POWERED: u8 = 1 << 6;

/// Setup Request
#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "8")]
pub struct SetupRequest {
    /// byte 0
    #[packed_field(bits = "0", ty = "enum")]
    pub bm_request_type_direction: Direction,
    #[packed_field(bits = "1..=2", ty = "enum")]
    pub bm_request_type_kind: Type,
    #[packed_field(bits = "3..=7", ty = "enum")]
    pub bm_request_type_recipient: Recipient,
    // byte 1
    #[packed_field(bytes = "1", ty = "enum")]
    pub b_request: StandardRequest,
    // byte 2-3
    #[packed_field(bytes = "2..=3", endian = "lsb")]
    pub w_value: Integer<u16, packed_bits::Bits<16>>,
    // byte 4-5
    #[packed_field(bytes = "4..=5", endian = "lsb")]
    pub w_index: Integer<u16, packed_bits::Bits<16>>,
    // byte 6-7
    #[packed_field(bytes = "6..=7", endian = "lsb")]
    pub w_length: Integer<u16, packed_bits::Bits<16>>,
}

/// Descriptor type (bDescriptorType, wValue [high bytes])
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum DescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    DeviceQualifier = 6,
    OtherSpeedConfiguration = 7,
    InterfacePower = 8,
}

/// Class code (assigned by the USB-IF).
/// https://www.usb.org/defined-class-codes
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum DeviceClass {
    UseInterface = 0x00,
    Cdc = 0x02,
    Hub = 0x09,
    Billboard = 0x11,
    Diagnostic = 0xdc,
    Miscellaneous = 0xef,
    VendorSpecific = 0xff,
}

/// The Device Descriptor is the root of the descriptor tree and contains basic
/// device information. The unique numbers, idVendor and idProduct, identify the
/// connected device. It is 18 bytes in size.
#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "18")]
pub struct DeviceDescriptor {
    /// Size of this descriptor in bytes.
    #[packed_field(bytes = "0")]
    pub b_length: u8,
    /// Device Descriptor Type = 1.
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    /// USB Specification Release Number in Binary-Coded Decimal (i.e., 2.10 is 210h).
    /// This field identifies the release of the USB Specification with which the
    /// device and its descriptors are compliant.
    #[packed_field(bytes = "2..=3", endian = "lsb")]
    pub bcd_usb: Integer<u16, packed_bits::Bits<16>>,
    /// Class code (assigned by the USB-IF).
    /// https://www.usb.org/defined-class-codes
    #[packed_field(bytes = "4")]
    pub b_device_class: u8,
    /// Subclass code (assigned by the USB-IF).
    /// https://www.usb.org/defined-class-codes
    #[packed_field(bytes = "5")]
    pub b_device_sub_class: u8,
    /// Protocol code (assigned by the USB-IF).
    /// https://www.usb.org/defined-class-codes
    #[packed_field(bytes = "6")]
    pub b_device_protocol: u8,
    /// Maximum packet size for Endpoint zero (only 8, 16, 32, or 64 are valid).
    #[packed_field(bytes = "7")]
    pub b_max_packet_size_0: u8,
    /// Vendor ID (assigned by the USB-IF).
    #[packed_field(bytes = "8..=9", endian = "lsb")]
    pub id_vendor: Integer<u16, packed_bits::Bits<16>>,
    /// Product ID (assigned by the manufacturer).
    #[packed_field(bytes = "10..=11", endian = "lsb")]
    pub id_product: Integer<u16, packed_bits::Bits<16>>,
    /// Device release number in binary-coded decimal.
    #[packed_field(bytes = "12..=13", endian = "lsb")]
    pub bcd_device: Integer<u16, packed_bits::Bits<16>>,
    /// Index of string descriptor describing manufacturer. Set to '0' if no
    /// string descriptors are provided.
    #[packed_field(bytes = "14")]
    pub i_manufacturer: u8,
    /// Index of string descriptor describing product. Set to '0' if no string
    /// descriptors are provided.
    #[packed_field(bytes = "15")]
    pub i_product: u8,
    /// Index of string descriptor describing the device's serial number. Set
    /// to '0' if no string descriptors are provided.
    #[packed_field(bytes = "16")]
    pub i_serial_number: u8,
    /// Number of possible configurations.
    #[packed_field(bytes = "17")]
    pub b_num_configurations: u8,
}

impl DeviceDescriptor {
    pub fn new(vendor_id: u16, product_id: u16) -> Self {
        Self {
            b_length: 18,
            b_descriptor_type: DescriptorType::Device as u8,
            bcd_usb: Integer::from_primitive(0x0200),
            b_device_class: 0x00,
            b_device_sub_class: 0x00,
            b_device_protocol: 0x00,
            b_max_packet_size_0: 0x10,
            id_vendor: Integer::from_primitive(vendor_id),
            id_product: Integer::from_primitive(product_id),
            bcd_device: Integer::from_primitive(0x0100),
            i_manufacturer: 0x01, // String 1
            i_product: 0x02,      // String 2
            i_serial_number: 0x00,
            b_num_configurations: 0x01,
        }
    }
}

impl Default for DeviceDescriptor {
    fn default() -> Self {
        Self::new(0x1234, 0x5678)
    }
}

/// A high-speed capable device that has different device information for
/// full-speed and high-speed must have a Device Qualifier Descriptor. For
/// example, if the device is currently operating at full-speed, the Device
/// Qualifier returns information about how it would operate at high-speed and
/// vice-versa.
///
/// The fields for the vendor, product, device, manufacturer, and serial number
/// are not included. This information is constant for a device regardless of
/// the supported speeds.
#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "10")]
pub struct DeviceQualifierDescriptor {
    /// Size of this descriptor in bytes.
    #[packed_field(bytes = "0")]
    pub b_length: u8,
    /// Device Qualifier Descriptor Type = 6.
    #[packed_field(bytes = "1")]
    pub b_type: u8,
    /// USB Specification Release Number in Binary-Coded Decimal (i.e., 2.10 is
    /// 210h). This field identifies the release of the USB Specification with
    /// which the device and its descriptors are compliant. At least V2.00 is
    /// required to use this descriptor.
    #[packed_field(bytes = "2..=3", endian = "lsb")]
    pub bcd_usb: Integer<u16, packed_bits::Bits<16>>,
    /// Class code (assigned by the USB-IF).
    /// https://www.usb.org/defined-class-codes
    #[packed_field(bytes = "4")]
    pub b_device_class: u8,
    /// Subclass code (assigned by the USB-IF).
    /// https://www.usb.org/defined-class-codes
    #[packed_field(bytes = "5")]
    pub b_device_sub_class: u8,
    /// Protocol code (assigned by the USB-IF).
    /// https://www.usb.org/defined-class-codes
    #[packed_field(bytes = "6")]
    pub b_device_protocol: u8,
    /// Maximum packet size for other speed.
    #[packed_field(bytes = "7")]
    pub b_max_packet_size_0: u8,
    /// Number of other-speed configurations.
    #[packed_field(bytes = "8")]
    pub b_num_configurations: u8,
    /// Reserved for future use, must be zero.
    #[packed_field(bytes = "9")]
    pub b_reserved: u8,
}

impl DeviceQualifierDescriptor {
    pub fn new() -> Self {
        Self {
            b_length: 10,
            b_type: DescriptorType::DeviceQualifier as u8,
            bcd_usb: Integer::from_primitive(0x0200),
            b_device_class: 0x03,
            b_device_sub_class: 0x00,
            b_device_protocol: 0x00,
            b_max_packet_size_0: 0x10,
            b_num_configurations: 0x01,
            b_reserved: 0x00,
        }
    }
}

impl Default for DeviceQualifierDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration is a higher-level structure for building a USB payload from
/// [ConfigurationDescriptor] and one or more [InterfaceDescriptor].
#[derive(Debug, Clone, PartialEq)]
pub struct Configuration {
    pub conf_desc: ConfigurationDescriptor,
    pub interfaces: Vec<Interface>,
}

impl Configuration {
    pub fn new(conf_desc: ConfigurationDescriptor, interfaces: Vec<Interface>) -> Self {
        Self {
            conf_desc,
            interfaces,
        }
    }

    /// Pack the configuration into a byte array
    pub fn pack_to_vec(&self) -> Result<Vec<u8>, PackingError> {
        // Get the size of the total configuration to allocate the
        // byte array to the correct size.
        let size = self.get_size();
        let mut result: Vec<u8> = Vec::with_capacity(size);

        // Update the config total size and num interfaces
        let mut config = self.conf_desc;
        config.b_num_interfaces = self.interfaces.len() as u8;
        config.w_total_length = Integer::from_primitive(size as u16);

        // Pack the config descriptor
        let mut bytes = config.pack_to_vec()?;
        result.append(&mut bytes);

        // Pack and append each interface descriptor
        for iface in self.interfaces.iter() {
            result.append(&mut iface.pack_to_vec()?);
        }

        Ok(result)
    }

    /// Returns the byte serialized size of the configuration
    pub fn get_size(&self) -> usize {
        let mut size = 9;
        for iface in self.interfaces.iter() {
            size += iface.get_size();
        }
        size
    }
}

impl Display for Configuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.conf_desc)
    }
}

/// [Configuration] builder for generating a new USB configuration
pub struct ConfigurationBuilder {
    config: Configuration,
}

impl ConfigurationBuilder {
    /// Create a new configuration builder for building a USB config
    pub fn new() -> Self {
        Self {
            config: Configuration {
                conf_desc: ConfigurationDescriptor::new(),
                interfaces: vec![],
            },
        }
    }

    /// Construct the new USB configuration
    pub fn build(&self) -> Configuration {
        self.config.clone()
    }

    /// Set the maximum power for this device.
    /// Maximum power consumption of the USB device from the bus in this specific
    /// configuration when the device is fully operational. Expressed in 2mA
    /// units (i.e., 50 = 100mA).
    pub fn max_power(&mut self, max_power_mA: u16) -> &mut Self {
        self.config.conf_desc.b_max_power = (max_power_mA / 2) as u8;
        self
    }

    /// Set the interface for this configuration
    pub fn interface(&mut self, mut interface: Interface) -> &mut Self {
        // Set the interface number
        interface.iface_desc.b_interface_number = self.config.interfaces.len() as u8;

        // Add the interface to the config and update the number of interfaces
        self.config.interfaces.push(interface);
        self.config.conf_desc.b_num_interfaces = self.config.interfaces.len() as u8;

        // Update the total size
        let mut size = 9; // Start with the size of the config desc header
        for iface in self.config.interfaces.iter() {
            size += iface.get_size();
        }
        self.config.conf_desc.w_total_length = Integer::from_primitive(size as u16);

        self
    }
}

impl Default for ConfigurationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The Configuration Descriptor contains information about the device power
/// requirements and the number of interfaces it can support. A device can have
/// multiple configurations. The host can select the configuration that best
/// matches the requirements of the application software.
#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "9")]
pub struct ConfigurationDescriptor {
    /// Size of this descriptor in bytes.
    #[packed_field(bytes = "0")]
    pub b_length: u8,
    /// Configuration Descriptor Type = 2.
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    /// Total length of data returned for this configuration. Includes the
    /// combined length of all descriptors (configuration, interface, endpoint,
    /// and class or vendor specific) returned for this configuration.
    #[packed_field(bytes = "2..=3", endian = "lsb")]
    pub w_total_length: Integer<u16, packed_bits::Bits<16>>,
    /// Number of interfaces supported by this configuration.
    #[packed_field(bytes = "4")]
    pub b_num_interfaces: u8,
    /// Value to select this configuration with SetConfiguration().
    #[packed_field(bytes = "5")]
    pub b_configuration_value: u8,
    /// Index of string descriptor describing this configuration.
    #[packed_field(bytes = "6")]
    pub i_configuration: u8,
    /// A device configuration that uses power from the bus and a local source
    /// reports a non-zero value in bMaxPower to indicate the amount of bus
    /// power required and sets D6. The actual power source at runtime can be
    /// determined using the GetStatus(DEVICE) request. If a device configuration
    /// supports remote wakeup, D5 is set to 1.
    #[packed_field(bytes = "7")]
    pub bm_attributes: u8,
    /// Maximum power consumption of the USB device from the bus in this specific
    /// configuration when the device is fully operational. Expressed in 2mA
    /// units (i.e., 50 = 100mA).
    #[packed_field(bytes = "8")]
    pub b_max_power: u8,
}

impl ConfigurationDescriptor {
    pub fn new() -> Self {
        Self {
            b_length: 9,
            b_descriptor_type: DescriptorType::Configuration as u8,
            w_total_length: Integer::from_primitive(0),
            b_num_interfaces: 0,
            b_configuration_value: 1,
            i_configuration: 0,
            bm_attributes: 0x80,
            b_max_power: 0,
        }
    }
}

impl Default for ConfigurationDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Interface {
    iface_desc: InterfaceDescriptor,
    data: Vec<u8>,
}

impl Interface {
    /// Create a new interface descriptor
    pub fn new() -> Self {
        Self {
            iface_desc: InterfaceDescriptor::new(),
            data: Vec::new(),
        }
    }

    /// Serialize the interface into bytes
    pub fn pack_to_vec(&self) -> Result<Vec<u8>, PackingError> {
        // Get the size of the total interface configuration to allocate the
        // byte array to the correct size.
        let size = 9 + self.data.len();

        let mut result: Vec<u8> = Vec::with_capacity(size);
        let mut bytes = self.iface_desc.pack_to_vec()?;
        result.append(&mut bytes);
        let mut data = self.data.clone();
        result.append(&mut data);

        Ok(result)
    }

    /// Returns the byte serialized size of the interface
    pub fn get_size(&self) -> usize {
        9 + self.data.len()
    }

    /// Returns the interface class
    pub fn get_class(&self) -> InterfaceClass {
        self.iface_desc.b_interface_class
    }
}

impl Default for Interface {
    fn default() -> Self {
        Self::new()
    }
}

/// USB defines class code information that is used to identify a device’s
/// functionality and to nominally load a device driver based on that
/// functionality.
/// Source: https://www.usb.org/defined-class-codes
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum InterfaceClass {
    Audio = 0x01,
    Cdc = 0x02,
    Hid = 0x03,
    Physical = 0x05,
    Image = 0x06,
    Printer = 0x07,
    MassStorage = 0x08,
    CdcData = 0x0a,
    SmartCard = 0x0b,
    ContentSecurity = 0x0d,
    Video = 0x0e,
    PersonalHealthcare = 0x0f,
    AudioVideo = 0x10,
    UsbTypeCBridge = 0x12,
    UsbBulkDisplayProtocol = 0x13,
    MctpOverUsbProtocol = 0x14,
    I3C = 0x3c,
    Diagnostic = 0xdc,
    WirelessController = 0xe0,
    Miscellaneous = 0xef,
    ApplicationSpecific = 0xfe,
    VendorSpecific = 0xff,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "9")]
pub struct InterfaceDescriptor {
    /// Size of this descriptor in bytes.
    #[packed_field(bytes = "0")]
    pub b_length: u8,
    /// Interface Descriptor Type = 4.
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    /// The number of this interface.
    #[packed_field(bytes = "2")]
    pub b_interface_number: u8,
    /// Value used to select an alternate setting for the interface identified
    /// in the prior field. Allows an interface to change the settings on the fly.
    #[packed_field(bytes = "3")]
    pub b_alternate_setting: u8,
    /// Number of endpoints used by this interface (excluding endpoint zero).
    #[packed_field(bytes = "4")]
    pub b_num_endpoints: u8,
    /// Class code (assigned by the USB-IF).
    #[packed_field(bytes = "5", ty = "enum")]
    pub b_interface_class: InterfaceClass,
    /// Subclass code (assigned by the USB-IF).
    #[packed_field(bytes = "6")]
    pub b_interface_subclass: u8,
    /// Protocol code (assigned by the USB).
    #[packed_field(bytes = "7")]
    pub b_interface_protocol: u8,
    /// Index of string descriptor describing this interface.
    #[packed_field(bytes = "8")]
    pub i_interface: u8,
}

impl InterfaceDescriptor {
    pub fn new() -> Self {
        Self {
            b_length: 9,
            b_descriptor_type: DescriptorType::Interface as u8,
            b_interface_number: 0,
            b_alternate_setting: 0,
            b_num_endpoints: 1,
            b_interface_class: InterfaceClass::Hid,
            b_interface_subclass: 0,
            b_interface_protocol: 0,
            i_interface: 0,
        }
    }
}

impl Default for InterfaceDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

/// [EndpointDescriptor] builder for constructing an endpoint descriptor
pub struct EndpointBuilder {
    descriptor: EndpointDescriptor,
}

impl EndpointBuilder {
    pub fn new() -> Self {
        Self {
            descriptor: EndpointDescriptor::new(),
        }
    }

    /// Construct the new Endpoint configuration.
    pub fn build(&self) -> EndpointDescriptor {
        self.descriptor
    }

    /// Set the endpoint address number. Should be greater than 0.
    pub fn address_num(&mut self, num: u8) -> &mut Self {
        self.descriptor.b_endpoint_address_num = Integer::from_primitive(num);
        self
    }

    /// Set the endpoint direction
    pub fn direction(&mut self, direction: Direction) -> &mut Self {
        self.descriptor.b_endpoint_address_direction = direction;
        self
    }

    /// Set the endpoint transfer type
    pub fn transfer_type(&mut self, xfer_type: TransferType) -> &mut Self {
        self.descriptor.bm_attributes_xfer_type = xfer_type;
        self
    }

    /// Set the endpoint synchronization type
    pub fn sync_type(&mut self, sync_type: SynchronizationType) -> &mut Self {
        self.descriptor.bm_attributes_sync_type = sync_type;
        self
    }

    /// Set the endpoint usage type
    pub fn usage_type(&mut self, usage_type: UsageType) -> &mut Self {
        self.descriptor.bm_attributes_usage_type = usage_type;
        self
    }

    /// Set the endpoint max packet size
    pub fn max_packet_size(&mut self, size: u16) -> &mut Self {
        self.descriptor.w_max_packet_size = Integer::from_primitive(size);
        self
    }

    /// Interval for polling endpoint for data transfers. Expressed in frames
    /// or micro-frames depending on the operating speed (1ms, or 125μs units).
    pub fn interval(&mut self, interval: u8) -> &mut Self {
        self.descriptor.b_interval = interval;
        self
    }
}

impl Default for EndpointBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Transfer type
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum TransferType {
    Control = 0,
    Isochronous = 1,
    Bulk = 2,
    Interrupt = 3,
}

/// Synchronization type
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum SynchronizationType {
    NoSynchronization = 0,
    Asynchronous = 1,
    Adaptive = 2,
    Synchronous = 3,
}

/// Usage type
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum UsageType {
    Data = 0,
    Feedback = 1,
    ImplicitFeedback = 2,
    Reserved = 3,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "7")]
pub struct EndpointDescriptor {
    /// Size of this descriptor in bytes.
    #[packed_field(bytes = "0")]
    pub b_length: u8,
    /// Endpoint Descriptor Type = 5.
    #[packed_field(bytes = "1", ty = "enum")]
    pub b_descriptor_type: DescriptorType,
    /// Byte 2
    /// The address of the endpoint on the USB device described by this
    /// descriptor. The address is encoded as follows:
    ///
    /// * Bit 3...0: The endpoint number
    /// * Bit 6...4: Reserved, reset to zero
    /// * Bit 7: Direction, ignored for control endpoints.
    ///   * 0 = OUT endpoint
    ///   * 1 = IN endpoint
    #[packed_field(bits = "16", ty = "enum")]
    pub b_endpoint_address_direction: Direction,
    #[packed_field(bits = "17..=19", endian = "lsb")]
    pub b_endpoint_address_reserved: Integer<u8, packed_bits::Bits<3>>,
    #[packed_field(bits = "20..=23", endian = "lsb")]
    pub b_endpoint_address_num: Integer<u8, packed_bits::Bits<4>>,
    /// Byte 3
    /// The endpoint attribute when configured through bConfigurationValue.
    ///
    /// * Bits 1..0: Transfer Type
    ///   * 00 = Control
    ///   * 01 = Isochronous
    ///   * 10 = Bulk
    ///   * 11 = Interrupt
    ///
    /// For non-isochronous endpoints, bits 5..2 must be set to zero. For
    /// isochronous endpoints, they are defined as:
    ///
    /// * Bits 3..2: Synchronization Type
    ///   * 00 = No Synchronization
    ///   * 01 = Asynchronous
    ///   * 10 = Adaptive
    ///   * 11 = Synchronous
    /// * Bits 5..4: Usage Type
    ///   * 00 = Data
    ///   * 01 = Feedback
    ///   * 10 = Implicit feedback
    ///   * 11 = Reserved
    ///
    /// All other bits are reserved and must be reset to zero.
    #[packed_field(bits = "24..=25")]
    pub bm_attributes_reserved: Integer<u8, packed_bits::Bits<2>>,
    #[packed_field(bits = "26..=27", ty = "enum")]
    pub bm_attributes_usage_type: UsageType,
    #[packed_field(bits = "28..=29", ty = "enum")]
    pub bm_attributes_sync_type: SynchronizationType,
    #[packed_field(bits = "30..=31", ty = "enum")]
    pub bm_attributes_xfer_type: TransferType,
    /// Is the maximum packet size of this endpoint. For isochronous endpoints,
    /// this value is used to reserve the time on the bus, required for the
    /// per-(micro)frame data payloads.
    ///
    /// * Bits 10..0 = max. packet size (in bytes).
    ///
    /// For high-speed isochronous and interrupt endpoints:
    ///
    /// * Bits 12..11 = number of additional transaction opportunities per micro-frame:
    ///   * 00 = None (1 transaction per micro-frame)
    ///   * 01 = 1 additional (2 per micro-frame)
    ///   * 10 = 2 additional (3 per micro-frame)
    ///   * 11 = Reserved
    /// * Bits 15..13 are reserved and must be set to zero.
    #[packed_field(bytes = "4..=5", endian = "lsb")]
    pub w_max_packet_size: Integer<u16, packed_bits::Bits<16>>,
    /// Interval for polling endpoint for data transfers. Expressed in frames
    /// or micro-frames depending on the operating speed (1ms, or 125μs units).
    #[packed_field(bytes = "6")]
    pub b_interval: u8,
}

impl EndpointDescriptor {
    pub fn new() -> Self {
        Self {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address_num: Integer::from_primitive(1),
            b_endpoint_address_reserved: Integer::from_primitive(0),
            b_endpoint_address_direction: Direction::Out,
            bm_attributes_xfer_type: TransferType::Control,
            bm_attributes_sync_type: SynchronizationType::NoSynchronization,
            bm_attributes_usage_type: UsageType::Data,
            bm_attributes_reserved: Integer::from_primitive(0),
            w_max_packet_size: Integer::from_primitive(0),
            b_interval: 1,
        }
    }
}

impl Default for EndpointDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

/// String descriptors are optional and add human readable information to the
/// other descriptors. If a device does not support string descriptors, all
/// references to string descriptors within device, configuration, and interface
/// descriptors must be set to zero.
///
/// Max character count is 126 (2 string descriptor header bytes + 126 UTF-16
/// characters).
#[derive(Debug, Clone)]
pub struct StringDescriptor {
    data: Vec<u8>,
    str: Option<String>,
}

impl StringDescriptor {
    pub fn pack_to_vec(&self) -> Result<Vec<u8>, PackingError> {
        let b_length = self.data.len() as u8 + 2;
        let b_descriptor_type = DescriptorType::String.to_primitive();
        let mut str_bytes = self.data.clone();
        if self.data.len() > 126 {
            return Err(PackingError::InvalidValue);
        }

        let mut desc = Vec::with_capacity(b_length as usize);
        desc.push(b_length);
        desc.push(b_descriptor_type);
        desc.append(&mut str_bytes);

        Ok(desc)
    }
}

impl Display for StringDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.str.clone().unwrap_or_default())
    }
}

impl From<String> for StringDescriptor {
    fn from(value: String) -> Self {
        Self {
            data: value.as_bytes().to_vec(),
            str: Some(value),
        }
    }
}

impl From<&str> for StringDescriptor {
    fn from(value: &str) -> Self {
        Self {
            data: value.as_bytes().to_vec(),
            str: Some(value.to_string()),
        }
    }
}

impl From<Vec<LangId>> for StringDescriptor {
    fn from(value: Vec<LangId>) -> Self {
        let mut bytes: Vec<u8> = Vec::with_capacity(value.len() * 2);
        for lang in value {
            let lang_id = lang as u16;
            let mut lang_bytes = lang_id.to_lsb_bytes().to_vec();
            bytes.append(&mut lang_bytes);
        }
        Self {
            data: bytes,
            str: None,
        }
    }
}

/// 16-bit language ID (LANGID) defined by the USB-IF
pub enum LangId {
    Afrikaans = 0x0436,
    Albanian = 0x041c,
    ArabicSaudiArabia = 0x0401,
    ArabicIraq = 0x0801,
    ArabicEgypt = 0x0c01,
    ArabicLibya = 0x1001,
    ArabicAlgeria = 0x1401,
    ArabicMorocco = 0x1801,
    ArabicTunisia = 0x1c01,
    ArabicOman = 0x2001,
    ArabicYemen = 0x2401,
    ArabicSyria = 0x2801,
    ArabicJordan = 0x2c01,
    ArabicLebanon = 0x3001,
    ArabicKuwait = 0x3401,
    ArabicUAE = 0x3801,
    ArabicBahrain = 0x3c01,
    ArabicQatar = 0x4001,
    Armenian = 0x042b,
    Assamese = 0x044d,
    AzeriLatin = 0x042c,
    AzeriCyrillic = 0x082c,
    Basque = 0x042d,
    Belarussian = 0x0423,
    Bengali = 0x0445,
    Bulgarian = 0x0402,
    Burmese = 0x0455,
    Catalan = 0x0403,
    ChineseTaiwan = 0x0404,
    ChinesePRC = 0x0804,
    ChineseHongKongSARPRC = 0x0c04,
    ChineseSingapore = 0x1004,
    ChineseMacauSAR = 0x1404,
    Croatian = 0x041a,
    Czech = 0x0405,
    Danish = 0x0406,
    DutchNetherlands = 0x0413,
    DutchBelgium = 0x0813,
    EnglishUnitedStates = 0x0409,
    EnglishUnitedKingdom = 0x0809,
    EnglishAustralian = 0x0c09,
    EnglishCanadian = 0x1009,
    EnglishNewZealand = 0x1409,
    EnglishIreland = 0x1809,
    EnglishSouthAfrica = 0x1c09,
    EnglishJamaica = 0x2009,
    EnglishCaribbean = 0x2409,
    EnglishBelize = 0x2809,
    EnglishTrinidad = 0x2c09,
    EnglishZimbabwe = 0x3009,
    EnglishPhilippines = 0x3409,
    Estonian = 0x0425,
    Faeroese = 0x0438,
    Farsi = 0x0429,
    Finnish = 0x040b,
    FrenchStandard = 0x040c,
    FrenchBelgian = 0x080c,
    FrenchCanadian = 0x0c0c,
    FrenchSwitzerland = 0x100c,
    FrenchLuxembourg = 0x140c,
    FrenchMonaco = 0x180c,
    Georgian = 0x0437,
    GermanStandard = 0x0407,
    GermanSwitzerland = 0x0807,
    GermanAustria = 0x0c07,
    GermanLuxembourg = 0x1007,
    GermanLiechtenstein = 0x1407,
    Greek = 0x0408,
    Gujarati = 0x0447,
    Hebrew = 0x040d,
    Hindi = 0x0439,
    Hungarian = 0x040e,
    Icelandic = 0x040f,
    Indonesian = 0x0421,
    ItalianStandard = 0x0410,
    ItalianSwitzerland = 0x0810,
    Japanese = 0x0411,
    Kannada = 0x044b,
    KashmiriIndia = 0x0860,
    Kazakh = 0x043f,
    Konkani = 0x0457,
    Korean = 0x0412,
    KoreanJohab = 0x0812,
    Latvian = 0x0426,
    Lithuanian = 0x0427,
    LithuanianClassic = 0x0827,
    Macedonian = 0x042f,
    MalayMalaysian = 0x043e,
    MalayBruneiDarussalam = 0x083e,
    Malayalam = 0x044c,
    Manipuri = 0x0458,
    Marathi = 0x044e,
    NepaliIndia = 0x0861,
    NorwegianBokmal = 0x0414,
    NorwegianNynorsk = 0x0814,
    Oriya = 0x0448,
    Polish = 0x0415,
    PortugueseBrazil = 0x0416,
    PortugueseStandard = 0x0816,
    Punjabi = 0x0446,
    Romanian = 0x0418,
    Russian = 0x0419,
    Sanskrit = 0x044f,
    SerbianCyrillic = 0x0c1a,
    SerbianLatin = 0x081a,
    Sindhi = 0x0459,
    Slovak = 0x041b,
    Slovenian = 0x0424,
    SpanishTraditionalSort = 0x040a,
    SpanishMexican = 0x080a,
    SpanishModernSort = 0x0c0a,
    SpanishGuatemala = 0x100a,
    SpanishCostaRica = 0x140a,
    SpanishPanama = 0x180a,
    SpanishDominicanRepublic = 0x1c0a,
    SpanishVenezuela = 0x200a,
    SpanishColombia = 0x240a,
    SpanishPeru = 0x280a,
    SpanishArgentina = 0x2c0a,
    SpanishEcuador = 0x300a,
    SpanishChile = 0x340a,
    SpanishUruguay = 0x380a,
    SpanishParaguay = 0x3c0a,
    SpanishBolivia = 0x400a,
    SpanishElSalvador = 0x440a,
    SpanishHonduras = 0x480a,
    SpanishNicaragua = 0x4c0a,
    SpanishPuertoRico = 0x500a,
    Sutu = 0x0430,
    SwahiliKenya = 0x0441,
    Swedish = 0x041d,
    SwedishFinland = 0x081d,
    Tamil = 0x0449,
    TatarTatarstan = 0x0444,
    Telugu = 0x044a,
    Thai = 0x041e,
    Turkish = 0x041f,
    Ukrainian = 0x0422,
    UrduPakistan = 0x0420,
    UrduIndia = 0x0820,
    UzbekLatin = 0x0443,
    UzbekCyrillic = 0x0843,
    Vietnamese = 0x042a,
    HIDUsageDataDescriptor = 0x04ff,
    HIDVendorDefined1 = 0xf0ff,
    HIDVendorDefined2 = 0xf4ff,
    HIDVendorDefined3 = 0xf8ff,
    HIDVendorDefined4 = 0xfcff,
}
