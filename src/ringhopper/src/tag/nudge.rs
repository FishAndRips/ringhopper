use std::str::FromStr;
use definitions::{ActorVariant, ContinuousDamageEffect, DamageEffect, Light, Object, PointPhysics, Projectile, Scenario, Sound};
use primitives::primitive::TagGroup;
use primitives::tag::PrimaryTagStructDyn;
use crate::FixedPrecision;
use crate::tag::object::{downcast_base_object_mut, is_object};

/// Return `true` if the tag group can be nudged.
pub fn is_nudgeable(group: TagGroup) -> bool {
    get_nudgeable_function(group).is_some()
}

/// Nudge the tag, fixing floating point precision issues from tag extraction.
///
/// Returns `true` if the tag was nudged.
///
/// # Hint
///
/// If I/O performance is a concern, use [`is_nudgeable`] to determine if a tag group has nudgeable values.
pub fn nudge_tag(a: &mut dyn PrimaryTagStructDyn) -> bool {
    if let Some(n) = get_nudgeable_function(a.group()) {
        n(a)
    }
    else {
        false
    }
}

fn get_nudgeable_function(tag_group: TagGroup) -> Option<fn(&mut dyn PrimaryTagStructDyn) -> bool> {
    match tag_group {
        TagGroup::ActorVariant => Some(|tag| nudge_actor_variant(tag.as_any_mut().downcast_mut().unwrap())),
        TagGroup::ContinuousDamageEffect => Some(|tag| nudge_continuous_damage_effect(tag.as_any_mut().downcast_mut().unwrap())),
        TagGroup::DamageEffect => Some(|tag| nudge_damage_effect(tag.as_any_mut().downcast_mut().unwrap())),
        TagGroup::PointPhysics => Some(|tag| nudge_point_physics(tag.as_any_mut().downcast_mut().unwrap())),
        TagGroup::Projectile => Some(|tag| nudge_projectile(tag.as_any_mut().downcast_mut().unwrap())),
        TagGroup::Scenario => Some(|tag| nudge_scenario(tag.as_any_mut().downcast_mut().unwrap())),
        TagGroup::Light => Some(|tag| nudge_light(tag.as_any_mut().downcast_mut().unwrap())),
        TagGroup::Sound => Some(|tag| nudge_sound(tag.as_any_mut().downcast_mut().unwrap())),
        n if is_object(n) => Some(|tag| nudge_object(downcast_base_object_mut(tag).unwrap())),
        _ => None
    }
}

fn nudge_object(object: &mut Object) -> bool {
    let mut result = false;
    for cc in &mut object.change_colors {
        if cc.permutations.items.is_empty() {
            continue
        }

        for p in &mut cc.permutations {
            nudge(&mut p.weight, &mut result);
        }

        let permutation_ratio = fixed_med!(1) / fixed_med!(cc.permutations.items.len());
        let mut all_same = true;

        const ERROR: FixedPrecision = match FixedPrecision::from_str("0.001") { Ok(n) => n, Err(_) => unreachable!() };

        for p in &cc.permutations.items {
            let fixed_weight = fixed_med!(p.weight);
            let proportion = (fixed_weight / permutation_ratio) - fixed_med!(1);

            if fixed_weight.is_negative() || proportion > ERROR {
                all_same = false;
                break;
            }
        }

        if all_same {
            result = true;
            for p in &mut cc.permutations {
                p.weight = 1.0;
            }
        }
    }
    result
}

fn nudge_sound(sound: &mut Sound) -> bool {
    let mut result = false;
    nudge(&mut sound.maximum_bend_rate, &mut result);
    result
}

fn nudge_actor_variant(a: &mut ActorVariant) -> bool {
    let mut result = false;
    nudge(&mut a.grenades.grenade_velocity, &mut result);
    result
}
fn nudge_continuous_damage_effect(a: &mut ContinuousDamageEffect) -> bool {
    let mut result = false;
    nudge(&mut a.camera_shaking.wobble_period, &mut result);
    result
}
fn nudge_damage_effect(a: &mut DamageEffect) -> bool {
    let mut result = false;
    nudge(&mut a.camera_shaking.wobble_period, &mut result);
    result
}

fn nudge_point_physics(a: &mut PointPhysics) -> bool {
    let mut result = false;
    nudge(&mut a.air_friction, &mut result);
    nudge(&mut a.water_friction, &mut result);
    result
}
fn nudge_projectile(a: &mut Projectile) -> bool {
    let mut result = false;
    nudge(&mut a.minimum_velocity, &mut result);
    nudge(&mut a.initial_velocity, &mut result);
    nudge(&mut a.final_velocity, &mut result);

    for i in &mut a.material_response {
        nudge(&mut i.potential_and.lower, &mut result);
        nudge(&mut i.potential_and.upper, &mut result);
    }
    result
}
fn nudge_scenario(a: &mut Scenario) -> bool {
    let mut result = false;
    for i in &mut a.cutscene_titles {
        nudge(&mut i.fade_in_time, &mut result);
        nudge(&mut i.fade_out_time, &mut result);
        nudge(&mut i.up_time, &mut result);
    }
    result
}
fn nudge_light(a: &mut Light) -> bool {
    let mut result = false;
    nudge(&mut a.effect_parameters.duration, &mut result);
    result
}

fn nudge(float: &mut f32, was_nudged_thus_far: &mut bool) {
    let fixed = fix_decimal_rounding(*float);
    if fixed != *float {
        *float = fixed;
        *was_nudged_thus_far |= true;
    }
}

pub(crate) fn fix_decimal_rounding(float: f32) -> f32 {
    use std::io::Write;

    // Too much rounding error.
    if float < -32766.0 || float > 32766.0 || float == 0.0 {
        return float
    }

    let sign = float.signum();
    let float_positive = float * sign;

    let mut number = std::io::Cursor::new(['0' as u8; 128]);
    number.write(&['0' as u8]).unwrap(); // ensure we have one extra digit for carrying
    write!(&mut number, "{float_positive}").unwrap();
    let mut number = number.into_inner();

    // Find the decimal
    let Some(decimal_location) = number.iter().position(|c| *c == '.' as u8) else {
        return float;
    };

    // Find the left side of the number
    let decimals = &mut number[decimal_location + 1..];
    let Some(first_non_zero) = decimals.iter().position(|c| *c != '0' as u8) else {
        return float;
    };
    let decimals = &mut decimals[first_non_zero..];

    // This is so many places away that we're probably just dealing with a whole number that got rekt by floats...
    if float_positive > 1.0 && first_non_zero > 4 {
        decimals.fill('0' as u8);
    }

    // Find the right side of the number
    let right_side = match decimals.iter().rev().position(|c| *c != '0' as u8) {
        Some(n) => decimals.len() - n,
        None => decimals.len()
    };

    let decimals = &mut decimals[..right_side];
    if decimals.len() < 3 {
        return float;
    }

    let decimals_as_num = std::str::from_utf8(decimals).unwrap();
    let first_000 = decimals_as_num.find("000");
    let first_999 = decimals_as_num.find("999");

    if first_000.is_some() && (first_999.is_none() || first_000 < first_999) {
        decimals[first_000.unwrap()..].fill('0' as u8);
    }
    else if let Some(first_999) = first_999 {
        decimals[first_999..].fill('0' as u8);
        let actual_position = first_999 + first_non_zero + decimal_location + 1;
        for i in (0..actual_position).rev() {
            let character = &mut number[i];

            match *character {
                x if x == '9' as u8 => {
                    *character = '0' as u8;
                },
                x if x == '.' as u8 => {
                    continue;
                },
                x => {
                    *character = x + 1;
                    break;
                }
            }
        }
    }
    else {
        return float;
    }

    f32::from_str(std::str::from_utf8(&number).unwrap()).unwrap() * sign
}

#[cfg(test)]
mod test;