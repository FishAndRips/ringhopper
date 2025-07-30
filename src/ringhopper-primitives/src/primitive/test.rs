#[test]
fn primitive_sizes() {
    // Check the size of the internal representation of everything

    use super::*;

    fn correct_size<T: SimpleTagData + Sized>(expected: usize) {
        let actual = std::mem::size_of::<T>();
        assert_eq!(actual, expected, "expected: {expected}, got {actual} instead");
        let actual_parsed = T::simple_size();
        assert_eq!(actual_parsed, expected, "expected: {expected}, got {actual_parsed} (parsed) instead");
    }

    // Everything is made up of these
    const U8_SIZE: usize = std::mem::size_of::<u8>();
    const U16_SIZE: usize = std::mem::size_of::<u16>();
    const U32_SIZE: usize = std::mem::size_of::<u32>();
    const F32_SIZE: usize = std::mem::size_of::<f32>();

    // This is what these sizes are equal to (this will always pass on Rust); this is just here for reference purposes
    assert_eq!(U8_SIZE, 1);
    assert_eq!(U16_SIZE, 2);
    assert_eq!(U32_SIZE, 4);
    assert_eq!(F32_SIZE, 4);

    // Actually check now
    correct_size::<Address>(U32_SIZE);
    correct_size::<Angle>(F32_SIZE);
    correct_size::<ColorARGBFloat>(F32_SIZE * 4);
    correct_size::<ColorARGBInt>(U32_SIZE);
    correct_size::<ColorARGBIntBytes>(U8_SIZE * 4);
    correct_size::<ColorRGBFloat>(F32_SIZE * 3);
    correct_size::<DataC>(U32_SIZE * 5);
    correct_size::<Euler2D>(F32_SIZE * 2);
    correct_size::<Euler3D>(F32_SIZE * 3);
    correct_size::<ID>(U32_SIZE);
    correct_size::<Matrix3x3>(F32_SIZE * 3 * 3);
    correct_size::<Plane2D>(F32_SIZE * 2 + F32_SIZE);
    correct_size::<Plane3D>(F32_SIZE * 3 + F32_SIZE);
    correct_size::<Quaternion>(F32_SIZE * 4);
    correct_size::<ReflexiveC<Vector3D>>(U32_SIZE * 3);
    correct_size::<String32>(U8_SIZE * 32);
    correct_size::<TagReferenceC>(U32_SIZE * 4);
    correct_size::<Vector2D>(F32_SIZE * 2);
    correct_size::<Vector3D>(F32_SIZE * 3);
}
