use packed_struct::prelude::*;

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
#[packed_struct(bit_numbering = "msb0", size_bytes = "6")]
pub struct HIDDescriptor {
    #[packed_field(bytes = "0")]
    pub b_length: u8,
    #[packed_field(bytes = "1")]
    pub b_descriptor_type: u8,
    #[packed_field(bytes = "2..=3", endian = "lsb")]
    pub bcd_hid: Integer<u16, packed_bits::Bits<16>>,
    #[packed_field(bytes = "4")]
    pub b_country_code: u8,
    #[packed_field(bytes = "5")]
    pub b_num_descriptors: u8,
}

pub struct HIDReport {
    pub b_descriptor_type: u8,
    pub w_descriptor_length: u16,
}
