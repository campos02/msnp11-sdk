use deku::{DekuRead, DekuWrite};

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct FileContext {
    pub size: u32,
    pub second_field: u32,
    pub file_size: u64,
    pub preview: u32,
}
