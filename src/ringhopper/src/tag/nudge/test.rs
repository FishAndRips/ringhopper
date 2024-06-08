use crate::tag::nudge::fix_decimal_rounding;

#[test]
pub fn test_nudgification() {
    fn test_nudge(expected: f32, value_to_nudge: f32) {
        assert_ne!(expected, value_to_nudge, "values already equal");
        assert_eq!(expected, fix_decimal_rounding(value_to_nudge), "nudging failed");
    }
    fn test_no_nudge(value_to_nudge: f32) {
        assert_eq!(value_to_nudge, fix_decimal_rounding(value_to_nudge), "nudged when it shouldn't");
    }

    test_nudge(1.75, 1.75000035762786865234375);
    test_nudge(1.75, 1.74999964237213134765625);

    test_nudge(0.0005, 0.000500000081956386566162109375);
    test_nudge(0.0005, 0.00049999985);
    test_nudge(0.001, 0.0009999999);
    test_nudge(0.01, 0.010009766);

    test_nudge(1.0, 0.9999995);
    test_nudge(1.0, 1.0000003);

    test_no_nudge(33.333332061767578125);
}