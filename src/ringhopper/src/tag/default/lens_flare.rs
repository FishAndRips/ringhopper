use primitives::{primitive::Angle, tag::PrimaryTagStructDyn};
use ringhopper_structs::LensFlare;

use crate::tag::nudge::fix_rounding_for_float;

/// In older tools, 360 radians (not degrees) was the default angle.
///
/// 360 radians is approximately ~20626.5 degrees.
///
/// This was fixed originally in Invader and officially in CEA.
const BUGGY_RADIUS_DEFAULT: Angle = Angle::from_radians(360.0);

pub fn set_defaults_for_lens_flare(tag: &mut dyn PrimaryTagStructDyn) {
    let lens_flare: &mut LensFlare = tag.as_any_mut().downcast_mut().unwrap();

    let is_default = lens_flare.rotation_function_scale == Default::default() || lens_flare.rotation_function_scale == BUGGY_RADIUS_DEFAULT;
    if is_default {
        lens_flare.rotation_function_scale = Angle::from_degrees(360.0);
    }
}

pub fn unset_defaults_for_lens_flare(tag: &mut dyn PrimaryTagStructDyn) {
    let lens_flare: &mut LensFlare = tag.as_any_mut().downcast_mut().unwrap();

    let is_set = fix_rounding_for_float(lens_flare.rotation_function_scale.to_degrees()) == 360.0 || lens_flare.rotation_function_scale == BUGGY_RADIUS_DEFAULT;
    if is_set {
        lens_flare.rotation_function_scale = Default::default();
    }
}

#[cfg(test)]
mod test;
