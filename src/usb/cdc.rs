use packed_struct::prelude::*;

pub enum CdcSubclass {
    None = 0x00,
    DirectLineControlModel = 0x01,
}

///// [Interface] builder for constructing an CDC (Communication Device Class)
///// interface descriptor.
//pub struct CdcInterfaceBuilder {
//    iface: Interface,
//}
//
//impl CdcInterfaceBuilder {
//    pub fn new() -> Self {
//        let mut iface = Interface::new();
//        iface.iface_desc.b_interface_class = InterfaceClass::Cdc;
//
//        Self { iface }
//    }
//
//    /// Construct the new Interface configuration
//    pub fn build(&self) -> Interface {
//        self.iface.clone()
//    }
//
//    /// Set the interface subclass
//    pub fn subclass(&mut self, subclass: u8) -> &mut Self {
//        self.iface.iface_desc.b_interface_subclass = subclass;
//        self
//    }
//}

//pub struct CDC {
//    header_func_descs: Vec<HeaderFunctionalDescriptor>,
//    call_management_func_descs: Vec<CallManagementFunctionalDescriptor>,
//    acm_func_descs: Vec<AbstractControlManagementFunctionalDescriptor>,
//    union_func_descs: Vec<UnionFunctionalDescriptor>,
//    endpoint_descs: Vec<EndpointDescriptor>,
//}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "5")]
pub struct HeaderFunctionalDescriptor {
    #[packed_field(bytes = "0")]
    pub b_function_length: u8,
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    #[packed_field(bytes = "2")]
    pub b_descriptor_subtype: u8,
    #[packed_field(bytes = "3..=4", endian = "lsb")]
    pub bcd_cdc: Integer<u16, packed_bits::Bits<16>>,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "4")]
pub struct AbstractControlManagementFunctionalDescriptor {
    #[packed_field(bytes = "0")]
    pub b_function_length: u8,
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    #[packed_field(bytes = "2")]
    pub b_descriptor_subtype: u8,
    #[packed_field(bytes = "3")]
    pub bm_capabilities: u8,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "5")]
pub struct UnionFunctionalDescriptor {
    #[packed_field(bytes = "0")]
    pub b_function_length: u8,
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    #[packed_field(bytes = "2")]
    pub b_descriptor_subtype: u8,
    #[packed_field(bytes = "3")]
    pub b_master_interface: u8,
    #[packed_field(bytes = "4")]
    pub b_slave_interface0: u8,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "5")]
pub struct CallManagementFunctionalDescriptor {
    #[packed_field(bytes = "0")]
    pub b_function_length: u8,
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    #[packed_field(bytes = "2")]
    pub b_descriptor_subtype: u8,
    #[packed_field(bytes = "3")]
    pub bm_capabilities: u8,
    #[packed_field(bytes = "4")]
    pub b_data_interface: u8,
}

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "7")]
pub struct LineCoding {
    #[packed_field(bytes = "0..=3", endian = "lsb")]
    pub dw_dte_rate: Integer<u32, packed_bits::Bits<32>>,
    #[packed_field(bytes = "4")]
    pub b_char_format: u8,
    #[packed_field(bytes = "5")]
    pub b_parity_type: u8,
    #[packed_field(bytes = "6")]
    pub b_data_bits: u8,
}
