use primitives::primitive::CompressedFloat;

#[test]
fn test_compressed_float_ranges() {
    let c = CompressedFloat { data: 0xFFFF };
    assert_eq!(-1.0, f32::from(c));
    let c = CompressedFloat { data: 0x7FFF };
    assert_eq!(1.0, f32::from(c));
    let c = CompressedFloat { data: 0x0000 };
    assert_eq!(0.0, f32::from(c));

    let mut last_value = 0.0;
    for data in 0x0001..0x7FFF {
        let value = CompressedFloat { data };
        let this_value = f32::from(value);

        assert!(this_value > last_value, "0x{data:04X} vs 0x{data:04X}-1 is not greater (expected {this_value} > {last_value})");

        last_value = this_value;
    }

    let mut last_value = -1.0;
    for data in (0x8000..0xFFFF).rev() {
        let value = CompressedFloat { data };
        let this_value = f32::from(value);

        assert!(this_value > last_value, "0x{data:04X} vs 0x{data:04X}-1 is not greater (expected {this_value} > {last_value})");

        last_value = this_value;
    }
}
