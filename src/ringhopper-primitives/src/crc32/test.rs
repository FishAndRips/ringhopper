const DATA : &'static [u8] = &[0x02, 0x12, 0x31, 0x24, 0x12, 0x49, 0x78, 0x12, 0x43, 0x78, 0x91, 0x46, 0x15, 0x12, 0x41, 0x23];
const DATA_CRC32: u32 = !0xA213D313u32;

use super::{crc32, crc32_with_init};

#[test]
fn crc32_empty_test() {
    // No bytes should = 0
    assert_eq!(!0, crc32(&[]));
}

#[test]
fn crc32_full_test() {
    // Calculate with the entire data at once.
    assert_eq!(DATA_CRC32, crc32(&DATA));
}

#[test]
fn crc32_with_init_test() {
    // Calculate the CRC32 with two halves of the data.
    assert_eq!(DATA_CRC32, crc32_with_init(crc32(&DATA[0..8]), &DATA[8..]));

    // Now try all bytes
    let mut crc: u32 = !0;
    for d in DATA {
        crc = crc32_with_init(crc, &[*d]);
    }
    assert_eq!(DATA_CRC32, crc);
}
