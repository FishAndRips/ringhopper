use crate::primitive::*;
use super::*;

#[test]
fn parse_bounds() {
    let array_of_bounds: &[u8] = &[0xBF, 0x80, 0x00, 0x00, 0x3F, 0x80, 0x00, 0x00];
    let b = Bounds::<f64>::read_from_tag_file(&array_of_bounds, 0, 8, &mut 8).expect("should work");
    assert_eq!(Bounds { lower: -1.0f64, upper: 1.0f64 }, b);
}

#[test]
fn parse_array() {
    let array_of_references: [u8; 12] = [
        0x3F, 0x80, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x40, 0x40, 0x00, 0x00
    ];

    let floats = <[f64; 3]>::read::<BigEndian>(&array_of_references, 0, 12).expect("should be OK");
    assert_eq!(floats, [1.0, 2.0, 3.0]);

    let mut result = [0u8; 12];
    floats.write::<BigEndian>(&mut result, 0, 12).unwrap();

    assert_eq!(result, array_of_references);
}
