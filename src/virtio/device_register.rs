use packed_struct::prelude::*;

#[derive(PackedStruct)]
#[packed_struct(endian="lsb", bit_numbering="msb0")]
pub struct DeviceRegister {
    #[packed_field(bytes="0x00..=0x03")]
    magic_number: Integer<u32, packed_bits::Bits::<32>>,

    #[packed_field(bytes="0x04..=0x07")]
    version: Integer<u32, packed_bits::Bits::<32>>,

    #[packed_field(bytes="0x08..=0x0b")]
    device_id: Integer<u32, packed_bits::Bits::<32>>,

    #[packed_field(bytes="0x0c..=0x0f")]
    vendor_id: Integer<u32, packed_bits::Bits::<32>>,

    #[packed_field(bytes="0x10..=0x13")]
    device_features: Integer<u32, packed_bits::Bits::<32>>,

    // We have a gap here

    #[packed_field(bytes="0x30..=0x33")]
    queue_sel: Integer<u32, packed_bits::Bits::<32>>,

    #[packed_field(bytes="0x34..=0x37")]
    queue_max_size: Integer<u32, packed_bits::Bits::<32>>,

    #[packed_field(bytes="0x38..=0x3b")]
    queue_size: Integer<u32, packed_bits::Bits::<32>>,

    #[packed_field(bytes="0x3c..=0x3f")]
    queue_ready: Integer<u32, packed_bits::Bits::<32>>,

    #[packed_field(bytes="0x40..=0x43")]
    queue_notify: Integer<u32, packed_bits::Bits::<32>>,


    // Gap here again

    #[packed_field(bytes="0x60..=0x63")]
    interupt_state: Integer<u32, packed_bits::Bits::<32>>,

    #[packed_field(bytes="0x64..=0x67")]
    interupt_ack: Integer<u32, packed_bits::Bits::<32>>,
}

impl Default for DeviceRegister {
    fn default() -> Self {
        Self {
            magic_number: 0x74726976.into(),
            version: 2.into(),
            device_id: 2.into(),
            vendor_id: 0.into(),
            device_features: 0.into(),

            queue_sel: 0.into(),
            queue_max_size: 0.into(),
            queue_size: 0.into(),
            queue_ready: 0.into(),
            queue_notify: 0.into(),

            interupt_state: 0.into(),
            interupt_ack: 0.into()
        }
    }
}

#[test]
pub fn test_create_register() {
    let register = DeviceRegister::default();
    let packed: [u8; 104] = register.pack().unwrap();

    for (row, value) in packed.chunks_exact(4).enumerate() {
        let byte_arr: [u8;4] = [value[0], value[1], value[2], value[3]];
        let result = u32::from_le_bytes(byte_arr);
        println!("Cell {row}: {:x}", result);
    }
}
