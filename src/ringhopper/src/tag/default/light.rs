use primitives::primitive::{Color, ColorRGBFloat};
use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::Light;

pub fn set_defaults_for_light(tag: &mut dyn PrimaryTagStructDyn) {
    set_or_unset_defaults_for_light(tag, false);
}

pub fn unset_defaults_for_light(tag: &mut dyn PrimaryTagStructDyn) {
    set_or_unset_defaults_for_light(tag, true);
}

fn set_or_unset_defaults_for_light(tag: &mut dyn PrimaryTagStructDyn, undefault: bool) {
    let light: &mut Light = tag.as_any_mut().downcast_mut().unwrap();

    let zeroed_out = ColorRGBFloat {
        red: 0.0,
        green: 0.0,
        blue: 0.0
    };
    let default = ColorRGBFloat {
        red: 1.0,
        green: 1.0,
        blue: 1.0
    };

    let lower_rgb = light.color.color.lower.rgb_float();
    let upper_rgb = light.color.color.upper.rgb_float();

    let from;
    let to;

    if undefault {
        from = default;
        to = zeroed_out;
    }
    else {
        from = zeroed_out;
        to = default;
    }

    if lower_rgb == from && upper_rgb == from {
        light.color.color.lower.red = to.red;
        light.color.color.lower.green = to.green;
        light.color.color.lower.blue = to.blue;
        light.color.color.upper.red = to.red;
        light.color.color.upper.green = to.green;
        light.color.color.upper.blue = to.blue;
    }
}
