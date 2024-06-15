use crate::parse::TagData;

use super::*;

#[test]
fn parse_string32() {
    let valid_bytes: [u8; 32] = ['v' as u8, 'a' as u8, 'l' as u8, 'i' as u8, 'd' as u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let valid = String32::from_bytes_lossy(&valid_bytes);
    assert_eq!("valid", valid);
    assert_eq!("valid", valid.as_str());

    let valid_dirty_bytes: [u8; 32] = ['v' as u8, 'a' as u8, 'l' as u8, 'i' as u8, 'd' as u8, 0, 'o' as u8, 'h' as u8, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let valid_dirty = String32::from_bytes_lossy(&valid_dirty_bytes);
    assert_eq!("valid", valid_dirty);
    assert_eq!("valid", valid_dirty.as_str());
    assert_eq!(valid, valid_dirty);

    let invalid_bytes: [u8; 32] = ['v' as u8, 'a' as u8, 'l' as u8, 'i' as u8, 'd' as u8, 0x90, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let invalid = String32::from_bytes_lossy(&invalid_bytes);
    assert_eq!("valid_", invalid);
    assert_eq!("valid_", invalid.as_str());

    let long_string = "asfdkhljaesrfewragaewrkguieqw4i7w43qe5iy9oewsrayuoirewquigwre5gisgisaerfuhgewrq5oiuh453iop;juji;ls";
    assert!(matches!(String32::from_str(long_string), Err(Error::String32SizeLimitExceeded)));

    assert_eq!(valid, String32::from_str("valid").expect("should be ok"));
    assert_eq!(valid, String32::read_from_tag_file(&valid_bytes, 0, valid_bytes.len(), &mut 0).unwrap());
    assert_eq!(valid_dirty, String32::read_from_tag_file(&valid_dirty_bytes, 0, valid_dirty_bytes.len(), &mut 0).unwrap());
    assert_eq!(invalid, String32::read_from_tag_file(&invalid_bytes, 0, invalid_bytes.len(), &mut 0).unwrap());
}

#[test]
fn parse_utf16_string() {
    let some_string = &[
        0, 0, 0, 10,
        0, 0, 0, 0,
        0, 0, 0, 0,
        0, 0, 0, 0,
        0, 0, 0, 0,
        'H' as u8, 0, 'e' as u8, 0, 'y' as u8, 0, '!' as u8, 0, 0, 0,
    ];

    let string = UTF16String::read_from_tag_file(some_string, 0, 0x14, &mut 0x14).unwrap();
    assert_eq!("Hey!", string.get_string().unwrap().as_str());
}