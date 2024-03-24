use packed_struct::prelude::*;
use virtual_usb::usb::{ConfigurationDescriptor, InterfaceDescriptor};

#[derive(PackedStruct, Debug, Copy, Clone, PartialEq)]
pub struct Configuration {
    #[packed_field(element_size_bytes = "9")]
    config_desc: ConfigurationDescriptor,

    #[packed_field(element_size_bytes = "9")]
    iface0_desc: InterfaceDescriptor,
}
