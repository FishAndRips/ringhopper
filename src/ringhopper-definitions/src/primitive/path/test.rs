use crate::{parse::TagData, primitive::TagGroup};

use super::TagReference;

#[test]
fn parse_tag_reference() {
    let bytes: &[u8] = &mut [
        0x77, 0x65, 0x61, 0x70, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x15, 0x00, 0x00, 0x00, 0x00,
        0x77, 0x65, 0x61, 0x70, 0x6F, 0x6E, 0x73, 0x5C, 0x70, 0x69, 0x73, 0x74, 0x6F, 0x6C, 0x5C, 0x70, 0x69, 0x73, 0x74, 0x6F, 0x6C, 0x00
    ];

    let mut cursor = 0x10;
    let tag = TagReference::read_from_tag_file(&bytes, 0, cursor, &mut cursor).expect("this is valid");

    assert_eq!(TagGroup::Weapon, tag.group());
    assert_eq!("weapons\\pistol\\pistol.weapon", tag.path().unwrap().to_win32_path());
    assert_eq!(cursor, bytes.len());

    let mut new_bytes = vec![0u8; 16];
    tag.write_to_tag_file(&mut new_bytes, 0, 16).unwrap();
    assert_eq!(bytes, &new_bytes[..]);
}
