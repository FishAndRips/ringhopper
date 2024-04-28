use primitives::primitive::Angle;
use ringhopper_structs::LensFlare;
use super::*;

#[test]
fn test_lens_flare() {
    let mut lens_flare = LensFlare::default();

    let zero = Angle::from_degrees(0.0);
    let non_default = Angle::from_degrees(180.0);
    let default = Angle::from_degrees(360.0);
    let buggy_default = BUGGY_RADIUS_DEFAULT;

    assert_eq!(zero, lens_flare.rotation_function_scale, "not initialized at 0°");

    set_defaults_for_lens_flare(&mut lens_flare);
    assert_eq!(default, lens_flare.rotation_function_scale, "defaulting from 0° -> 360° doesn't work");
    unset_defaults_for_lens_flare(&mut lens_flare);
    assert_eq!(zero, lens_flare.rotation_function_scale, "undefaulting from 360° -> 0° doesn't work");

    lens_flare.rotation_function_scale = buggy_default;
    set_defaults_for_lens_flare(&mut lens_flare);
    assert_eq!(default, lens_flare.rotation_function_scale, "defaulting from 360 rad -> 360° doesn't work");
    lens_flare.rotation_function_scale = buggy_default;
    unset_defaults_for_lens_flare(&mut lens_flare);
    assert_eq!(zero, lens_flare.rotation_function_scale, "undefaulting from 360 rad -> 0° doesn't work");

    lens_flare.rotation_function_scale = non_default;
    set_defaults_for_lens_flare(&mut lens_flare);
    assert_eq!(non_default, lens_flare.rotation_function_scale, "180° shouldn't default");
    unset_defaults_for_lens_flare(&mut lens_flare);
    assert_eq!(non_default, lens_flare.rotation_function_scale, "180° shouldn't undefault");
}
