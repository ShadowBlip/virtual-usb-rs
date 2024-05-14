//! HID (Human Interface Device)
//! https://www.usb.org/sites/default/files/hid1_11.pdf

use packed_struct::prelude::*;

use super::{Interface, InterfaceClass};

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
    Mouse = 0x01,
    Keyboard = 0x02,
}

/// [Interface] builder for constructing an HID (Human Interface Device)
/// interface descriptor.
pub struct HidInterfaceBuilder {
    iface: Interface,
    descriptor: HidDescriptor,
    report_descriptors: Vec<(HidReportDescriptor, Vec<u8>)>,
}

impl HidInterfaceBuilder {
    pub fn new() -> Self {
        let mut iface = Interface::new();
        iface.iface_desc.b_interface_class = InterfaceClass::Hid as u8;

        Self {
            iface,
            descriptor: HidDescriptor::new(),
            report_descriptors: Vec::new(),
        }
    }

    /// Construct the new Interface configuration. Will panic if unable to
    /// pack interface into byte array.
    pub fn build(&self) -> Interface {
        let mut iface = self.iface.clone();
        let mut data = self.descriptor.pack_to_vec().unwrap();

        // Append the descriptor data to the interface
        iface.data.append(&mut data);

        // Append the report descriptors to the interface
        for (header, report) in self.report_descriptors.clone().iter_mut() {
            // Pack the header and append the descriptor
            let mut data = header.pack_to_vec().unwrap();
            data.append(report);
            iface.data.append(&mut data);
        }

        iface
    }

    /// Set the interface subclass
    pub fn subclass(&mut self, subclass: HidSubclass) -> &mut Self {
        self.iface.iface_desc.b_interface_subclass = subclass as u8;
        self
    }

    /// Set the interface protocol
    pub fn protocol(&mut self, protocol: InterfaceProtocol) -> &mut Self {
        self.iface.iface_desc.b_interface_protocol = protocol as u8;
        self
    }

    /// Set the identifying country code of the localized hardware
    pub fn country_code(&mut self, code: u8) -> &mut Self {
        self.descriptor.b_country_code = code;
        self
    }

    /// Set the given report descriptor bytes on the interface
    pub fn report_descriptor(&mut self, descriptor: &[u8]) -> &mut Self {
        // Create a new report descriptor header
        let mut report_descriptor = HidReportDescriptor::new();
        report_descriptor.w_descriptor_length = Integer::from_primitive(descriptor.len() as u16);

        // Add the header and descriptor data
        self.report_descriptors
            .push((report_descriptor, descriptor.to_vec()));

        // Increment the number of descriptors in the interface
        self.descriptor.b_num_descriptors += 1;

        self
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
            bcd_hid: Integer::from_primitive(0),
            b_country_code: 33,
            b_num_descriptors: 0,
        }
    }
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "3")]
pub struct HidReportDescriptor {
    #[packed_field(bytes = "0")]
    pub b_descriptor_type: u8,
    #[packed_field(bytes = "1..=2", endian = "lsb")]
    pub w_descriptor_length: Integer<u16, packed_bits::Bits<16>>,
}

impl HidReportDescriptor {
    pub fn new() -> Self {
        Self {
            b_descriptor_type: 34, // Report
            w_descriptor_length: Integer::from_primitive(0),
        }
    }
}
