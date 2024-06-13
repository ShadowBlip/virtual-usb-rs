//! HID (Human Interface Device)
//! https://www.usb.org/sites/default/files/hid1_11.pdf

use std::fmt::Display;

use packed_struct::prelude::*;

use super::{
    Direction, EndpointDescriptor, Interface, InterfaceClass, InterfaceDescriptor, Recipient,
    SetupRequest, StandardRequest, Type,
};

/// HID class-specific descriptor request type (wValue)
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum HidDescriptorType {
    Hid = 0x21,
    Report = 0x22,
    Physical = 0x23,
}

/// GetDescriptor representation of a SetupRequest for Human Interface Devices
#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "8")]
pub struct HidGetDescriptorRequest {
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
    // byte 2-3 (wValue)
    #[packed_field(bytes = "2")]
    pub b_descriptor_index: u8,
    #[packed_field(bytes = "3", ty = "enum")]
    pub b_descriptor_type: HidDescriptorType,
    // byte 4-5 (wIndex)
    #[packed_field(bytes = "4..=5", endian = "lsb")]
    pub w_interface_number: Integer<u16, packed_bits::Bits<16>>,
    // byte 6-7 (wLength)
    #[packed_field(bytes = "6..=7", endian = "lsb")]
    pub w_descriptor_length: Integer<u16, packed_bits::Bits<16>>,
}

impl From<SetupRequest> for HidGetDescriptorRequest {
    fn from(value: SetupRequest) -> Self {
        let data = value.pack().unwrap();
        HidGetDescriptorRequest::unpack(&data).unwrap()
    }
}

/// HID class-specific request type (bRequest)
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum HidRequestType {
    Unknown = 0x00,
    /// The Get_Report request allows the host to receive a report via the Control pipe.
    GetReport = 0x01,
    /// The Get_Idle request reads the current idle rate for a particular Input report (see:
    /// Set_Idle request).
    GetIdle = 0x02,
    /// The Get_Protocol request reads which protocol is currently active (either the boot
    /// protocol or the report protocol.)
    GetProtocol = 0x03,
    _Reverved0 = 0x04,
    _Reverved1 = 0x05,
    _Reverved2 = 0x06,
    _Reverved3 = 0x07,
    _Reverved4 = 0x08,
    /// The Set_Report request allows the host to send a report to the device, possibly
    /// setting the state of input, output, or feature controls.
    SetReport = 0x09,
    /// The Set_Idle request silences a particular report on the Interrupt In pipe until a
    /// new event occurs or the specified amount of time passes
    SetIdle = 0x0a,
    /// The Set_Protocol switches between the boot protocol and the report protocol (or
    /// vice versa).
    SetProtocol = 0x0b,
}

impl From<StandardRequest> for HidRequestType {
    fn from(value: StandardRequest) -> Self {
        match value.to_primitive() {
            0x01 => Self::GetReport,
            0x02 => Self::GetIdle,
            0x03 => Self::GetProtocol,
            0x04 => Self::_Reverved0,
            0x05 => Self::_Reverved1,
            0x06 => Self::_Reverved2,
            0x07 => Self::_Reverved3,
            0x08 => Self::_Reverved4,
            0x09 => Self::SetReport,
            0x0a => Self::SetIdle,
            0x0b => Self::SetProtocol,
            _ => Self::Unknown,
        }
    }
}

/// A Human Interface Device (HID) USB request
pub enum HidRequest {
    Unknown,
    GetReport(HidReportRequest),
    SetReport(HidReportRequest),
    SetIdle(HidSetIdleRequest),
}

// TODO: implement TryFrom instead
impl From<SetupRequest> for HidRequest {
    fn from(setup: SetupRequest) -> Self {
        let request_type = HidRequestType::from(setup.b_request);
        match request_type {
            HidRequestType::GetReport => Self::GetReport(setup.into()),
            HidRequestType::GetIdle => todo!(),
            HidRequestType::GetProtocol => todo!(),
            HidRequestType::SetReport => Self::SetReport(setup.into()),
            HidRequestType::SetIdle => Self::SetIdle(setup.into()),
            HidRequestType::SetProtocol => todo!(),
            _ => Self::Unknown,
        }
    }
}

/// SetIdle request
#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "8")]
pub struct HidSetIdleRequest {
    /// byte 0
    #[packed_field(bits = "0", ty = "enum")]
    pub bm_request_type_direction: Direction,
    #[packed_field(bits = "1..=2", ty = "enum")]
    pub bm_request_type_kind: Type,
    #[packed_field(bits = "3..=7", ty = "enum")]
    pub bm_request_type_recipient: Recipient,
    // byte 1
    #[packed_field(bytes = "1", ty = "enum")]
    pub b_request: HidRequestType,
    // byte 2-3 (wValue)
    #[packed_field(bytes = "2")]
    pub report_id: u8,
    #[packed_field(bytes = "3")]
    pub duration: u8,
    // byte 4-5 (wIndex)
    #[packed_field(bytes = "4..=5", endian = "lsb")]
    pub interface: Integer<u16, packed_bits::Bits<16>>,
    // byte 6-7 (wLength)
    #[packed_field(bytes = "6..=7", endian = "lsb")]
    pub _unused: Integer<u16, packed_bits::Bits<16>>,
}

impl From<SetupRequest> for HidSetIdleRequest {
    fn from(value: SetupRequest) -> Self {
        let data = value.pack().unwrap();
        HidSetIdleRequest::unpack(&data).unwrap()
    }
}

/// HID report type
#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum HidReportType {
    Input = 0x01,
    Output = 0x02,
    Feature = 0x03,
}

/// SetReport request
#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "8")]
pub struct HidReportRequest {
    /// byte 0
    #[packed_field(bits = "0", ty = "enum")]
    pub bm_request_type_direction: Direction,
    #[packed_field(bits = "1..=2", ty = "enum")]
    pub bm_request_type_kind: Type,
    #[packed_field(bits = "3..=7", ty = "enum")]
    pub bm_request_type_recipient: Recipient,
    // byte 1
    #[packed_field(bytes = "1", ty = "enum")]
    pub b_request: HidRequestType,
    // byte 2-3 (wValue)
    #[packed_field(bytes = "2")]
    pub report_id: u8,
    #[packed_field(bytes = "3", ty = "enum")]
    pub report_type: HidReportType,
    // byte 4-5 (wIndex)
    #[packed_field(bytes = "4..=5", endian = "lsb")]
    pub interface: Integer<u16, packed_bits::Bits<16>>,
    // byte 6-7 (wLength)
    #[packed_field(bytes = "6..=7", endian = "lsb")]
    pub report_length: Integer<u16, packed_bits::Bits<16>>,
}

impl From<SetupRequest> for HidReportRequest {
    fn from(value: SetupRequest) -> Self {
        let data = value.pack().unwrap();
        HidReportRequest::unpack(&data).unwrap()
    }
}

impl HidReportRequest {
    pub fn new(report: &[u8]) -> Self {
        Self {
            bm_request_type_direction: Direction::In,
            bm_request_type_kind: Type::Class,
            bm_request_type_recipient: Recipient::Interface,
            b_request: HidRequestType::GetReport,
            report_id: 0,
            report_type: HidReportType::Input,
            interface: Integer::from_primitive(2), // TODO: don't hardcode this
            report_length: Integer::from_primitive(report.len() as u16),
        }
    }
}

/// Subclass codes for HID descriptors
pub enum HidSubclass {
    None = 0x00,
    Boot = 0x01,
}

/// A variety of protocols are supported HID devices. The bInterfaceProtocol
/// member of an Interface descriptor only has meaning if the bInterfaceSubClass
/// member declares that the device supports a boot interface, otherwise it is 0.
pub enum InterfaceProtocol {
    None = 0x00,
    Keyboard = 0x01,
    Mouse = 0x02,
}

/// Human Interface Device (HID) interface definition
#[derive(Debug, Clone)]
pub struct HidInterface {
    pub iface: InterfaceDescriptor,
    pub descriptor: HidDescriptor,
    pub report_descriptors: Vec<&'static [u8]>,
    pub report_descriptor_info: Vec<HidReportDescriptorInfo>,
    pub endpoint_descriptors: Vec<EndpointDescriptor>,
}

impl HidInterface {
    pub fn new() -> Self {
        let iface = InterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: super::DescriptorType::Interface as u8,
            b_interface_number: 0,
            b_alternate_setting: 0,
            b_num_endpoints: 1,
            b_interface_class: InterfaceClass::Hid,
            b_interface_subclass: 0,
            b_interface_protocol: 0,
            i_interface: 0,
        };

        Self {
            iface,
            descriptor: HidDescriptor::new(),
            report_descriptors: Vec::new(),
            report_descriptor_info: Vec::new(),
            endpoint_descriptors: Vec::new(),
        }
    }

    /// Serialize the interface into bytes
    pub fn pack_to_vec(&self) -> Result<Vec<u8>, PackingError> {
        // Get the size of the total interface configuration to allocate the
        // byte array to the correct size.
        let size = self.get_size();
        let mut result: Vec<u8> = Vec::with_capacity(size);

        // Pack the interface
        let mut bytes = self.iface.pack_to_vec()?;
        result.append(&mut bytes);

        // Pack the HID descriptor
        let mut bytes = self.descriptor.pack_to_vec()?;
        result.append(&mut bytes);

        // Pack the report descriptors
        for report_desc in self.report_descriptor_info.iter() {
            let mut bytes = report_desc.pack_to_vec()?;
            result.append(&mut bytes);
        }

        // Pack the endpoint descriptors
        for endpoint_desc in self.endpoint_descriptors.iter() {
            let mut bytes = endpoint_desc.pack_to_vec()?;
            result.append(&mut bytes);
        }

        Ok(result)
    }

    /// Returns the byte serialized size of the interface
    pub fn get_size(&self) -> usize {
        // InterfaceDesc + HidDesc + (HidReportDesc * count) + (EndpointDesc * count)
        9 + 6 + (3 * self.report_descriptor_info.len()) + (7 * self.endpoint_descriptors.len())
    }

    /// Returns the interface class
    pub fn get_class(&self) -> InterfaceClass {
        self.iface.b_interface_class
    }

    /// Set the interface number for this interface
    pub fn set_interface_number(&mut self, num: u8) {
        self.iface.b_interface_number = num;
    }
}

impl Display for HidInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut text = vec![format!("{}", self.iface), format!("{}", self.descriptor)];
        for desc in self.report_descriptor_info.iter() {
            text.push(format!("{}", desc));
        }
        for desc in self.endpoint_descriptors.iter() {
            text.push(format!("{}", desc));
        }
        write!(f, "{}", text.join("\n"))
    }
}

impl Default for HidInterface {
    fn default() -> Self {
        Self::new()
    }
}

/// [Interface] builder for constructing an HID (Human Interface Device)
/// interface descriptor.
pub struct HidInterfaceBuilder {
    iface: HidInterface,
}

impl HidInterfaceBuilder {
    pub fn new() -> Self {
        Self {
            iface: HidInterface::default(),
        }
    }

    /// Construct the new Interface configuration.
    pub fn build(&self) -> Interface {
        log::debug!("HID Interface: {}", self.iface);
        Interface::Hid(self.iface.clone())
    }

    /// Set the interface subclass
    pub fn subclass(&mut self, subclass: HidSubclass) -> &mut Self {
        self.iface.iface.b_interface_subclass = subclass as u8;
        self
    }

    /// Set the interface protocol
    pub fn protocol(&mut self, protocol: InterfaceProtocol) -> &mut Self {
        self.iface.iface.b_interface_protocol = protocol as u8;
        self
    }

    /// Set the identifying country code of the localized hardware
    pub fn country_code(&mut self, code: u8) -> &mut Self {
        self.iface.descriptor.b_country_code = code;
        self
    }

    /// Set the given report descriptor bytes on the interface
    pub fn report_descriptor(&mut self, report_desc: &'static [u8]) -> &mut Self {
        // Create a new report descriptor header
        let size = report_desc.len();
        let mut info = HidReportDescriptorInfo::new();
        info.b_descriptor_type = DescriptorType::Report;
        info.w_descriptor_length = Integer::from_primitive(size as u16);

        // Add the header and descriptor data
        self.iface.report_descriptors.push(report_desc);
        self.iface.report_descriptor_info.push(info);

        // Increment the number of descriptors in the interface
        self.iface.descriptor.b_num_descriptors += 1;
        self.iface.descriptor.b_length += 3; // Add to the total size

        self
    }

    /// Add the given endpoint to the interface
    pub fn endpoint_descriptor(&mut self, descriptor: EndpointDescriptor) -> &mut Self {
        self.iface.endpoint_descriptors.push(descriptor);
        self.iface.iface.b_num_endpoints = self.iface.endpoint_descriptors.len() as u8;
        self
    }
}

impl Default for HidInterfaceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "6")]
pub struct HidDescriptor {
    /// Numeric expression that is the total size of the HID descriptor.
    #[packed_field(bytes = "0")]
    pub b_length: u8,
    /// Constant name specifying type of HID descriptor.
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    /// Numeric expression identifying the HID Class Specification release.
    #[packed_field(bytes = "2..=3", endian = "lsb")]
    pub bcd_hid: Integer<u16, packed_bits::Bits<16>>,
    /// Numeric expression identifying country code of the localized hardware.
    #[packed_field(bytes = "4")]
    pub b_country_code: u8,
    /// Numeric expression specifying the number of class descriptors (always
    /// at least one i.e. Report descriptor.)
    #[packed_field(bytes = "5")]
    pub b_num_descriptors: u8,
}

impl HidDescriptor {
    pub fn new() -> Self {
        Self {
            b_length: 6,
            b_descriptor_type: 33,
            bcd_hid: Integer::from_primitive(0x0110), // 1.10
            b_country_code: 0,
            b_num_descriptors: 0,
        }
    }
}

impl Default for HidDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq)]
pub enum DescriptorType {
    Report = 34,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "3")]
pub struct HidReportDescriptorInfo {
    #[packed_field(bytes = "0", ty = "enum")]
    pub b_descriptor_type: DescriptorType,
    #[packed_field(bytes = "1..=2", endian = "lsb")]
    pub w_descriptor_length: Integer<u16, packed_bits::Bits<16>>,
}

impl HidReportDescriptorInfo {
    pub fn new() -> Self {
        Self {
            b_descriptor_type: DescriptorType::Report,
            w_descriptor_length: Integer::from_primitive(0),
        }
    }
}

impl Default for HidReportDescriptorInfo {
    fn default() -> Self {
        Self::new()
    }
}
