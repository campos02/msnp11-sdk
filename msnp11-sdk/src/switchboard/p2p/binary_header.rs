use deku::{DekuRead, DekuWrite};

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct BinaryHeader {
    pub session_id: u32,
    pub identifier: u32,
    pub data_offset: u64,
    pub total_data_size: u64,
    pub length: u32,
    pub flag: u32,
    pub ack_identifier: u32,
    pub ack_unique_id: u32,
    pub ack_data_size: u64,
}
