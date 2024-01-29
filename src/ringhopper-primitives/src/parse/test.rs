use super::*;

#[test]
fn usize_is_u32_when_parsing_tags() {
    let expected_size = 0x12345678usize;

    let mut data = [0u8; 4];
    expected_size.write::<BigEndian>(&mut data, 0, 4).expect("should work");
    assert_eq!(expected_size, usize::read_from_tag_file(&data, 0, 4, &mut 0).unwrap());

    let too_big_size = match MAX_ARRAY_LENGTH.checked_add(1) {
        Some(n) => n,
        None => return // can't test array limit if the array limit is usize's integer limit (e.g. we're on 32-bit) so just return now
    };
    assert!(matches!(too_big_size.write::<BigEndian>(&mut data, 0, 4), Err(Error::ArrayLimitExceeded)));
    assert_eq!(expected_size, usize::read_from_tag_file(&data, 0, 4, &mut 0).unwrap()); // is unchanged
}
