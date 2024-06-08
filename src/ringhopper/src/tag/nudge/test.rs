use crate::tag::nudge::fix_rounding_for_float;

#[test]
pub fn test_nudgification() {
    fn test_nudge(expected: f32, value_to_nudge: f32) {
        assert_ne!(expected, value_to_nudge, "values already equal");
        assert_eq!(expected, fix_rounding_for_float(value_to_nudge), "nudging failed");
    }
    fn test_no_nudge(value_to_nudge: f32) {
        assert_eq!(value_to_nudge, fix_rounding_for_float(value_to_nudge), "nudged when it shouldn't");
    }

    test_nudge(1.0, 0.9999995);
    test_nudge(1.0, 1.0000003);

    test_nudge(2.0, 2.000004);
    test_nudge(2.0, 1.9999994);

    test_nudge(1.75, 1.7500007152557373046875);
    test_nudge(1.75, 1.74999964237213134765625);

    test_no_nudge(33.333332061767578125);
    test_no_nudge(2631.512451171875);
}