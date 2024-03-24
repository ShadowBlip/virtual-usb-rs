#![allow(warnings)]
use packed_struct::prelude::*;

/// Descriptor type (bDescriptorType, wValue [high bytes])
#[derive(PrimitiveEnum, Debug, Copy, Clone, PartialEq)]
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
            b_device_class: 0x03,
            b_device_sub_class: 0x00,
            b_device_protocol: 0x00,
            b_max_packet_size_0: 0x10,
            id_vendor: Integer::from_primitive(vendor_id),
            id_product: Integer::from_primitive(product_id),
            bcd_device: Integer::from_primitive(0x0100),
            i_manufacturer: 0x00, // String 1
            i_product: 0x00,      // String 2
            i_serial_number: 0x00,
            b_num_configurations: 0x01,
        }
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
            w_total_length: todo!(),
            b_num_interfaces: todo!(),
            b_configuration_value: todo!(),
            i_configuration: todo!(),
            bm_attributes: todo!(),
            b_max_power: todo!(),
        }
    }
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
    #[packed_field(bytes = "5")]
    pub b_interface_class: u8,
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

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "7")]
pub struct EndpointDescriptor {
    /// Size of this descriptor in bytes.
    #[packed_field(bytes = "0")]
    pub b_length: u8,
    /// Endpoint Descriptor Type = 5.
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    /// The address of the endpoint on the USB device described by this
    /// descriptor. The address is encoded as follows:
    ///
    /// * Bit 3...0: The endpoint number
    /// * Bit 6...4: Reserved, reset to zero
    /// * Bit 7: Direction, ignored for control endpoints.
    ///   * 0 = OUT endpoint
    ///   * 1 = IN endpoint
    #[packed_field(bytes = "2")]
    pub b_endpoint_address: u8,
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
    #[packed_field(bytes = "3")]
    pub bm_attributes: u8,
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

// TODO: PackedStruct does not support generics to support aribitarily sized
// arrays.
//
/// String descriptors are optional and add human readable information to the
/// other descriptors. If a device does not support string descriptors, all
/// references to string descriptors within device, configuration, and interface
/// descriptors must be set to zero.
///
/// Max character count is 126 (2 string descriptor header bytes + 126 UTF-16
/// characters).
#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0")]
pub struct StringDescriptorExample {
    #[packed_field(bytes = "0")]
    pub b_length: u8,
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    #[packed_field(element_size_bits = "16", endian = "lsb")]
    pub str: [u16; 126],
}

pub struct StringDescriptor {
    str: String,
}
