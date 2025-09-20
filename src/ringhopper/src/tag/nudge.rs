use std::borrow::Cow;
use definitions::{ActorVariant, ContinuousDamageEffect, DamageEffect, Light, Object, PointPhysics, Projectile, Scenario, Sound};
use primitives::primitive::TagGroup;
use primitives::tag::PrimaryTagStructDyn;
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

        let mut all_same = true;

        let error = 0.001;
        let permutation_ratio_inverse = cc.permutations.items.len() as f32;

        for p in &cc.permutations.items {
            let weight = p.weight;
            let proportion = (weight * &permutation_ratio_inverse) - 1.0;

            if p.weight < 0.0 || proportion.abs() > error {
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

const MAX_SIG_FIGS: usize = 6;

pub(crate) fn fix_decimal_rounding(input: f32) -> f32 {
    use std::io::Write;

    let signum = input.signum();
    let abs = input.abs();

    // 512 digits should be more than enough to hold any 64-bit float
    let mut buf = std::io::Cursor::new([0u8; 512]);
    write!(&mut buf, "{abs}").expect("can't write float to buffer");
    let mut buf = buf.into_inner();
    let str_index = buf.iter().position(|b| *b == 0).expect("can't get float output");

    let written = &mut buf[..str_index];

    // First, find the most significant digits
    if written.len() <= MAX_SIG_FIGS {
        return input;
    }

    macro_rules! ignore_dot_iter {
        ($a:expr) => {{
            let mut first_sig_fig_found = false;
            ($a)
                .iter_mut()
                .enumerate()
                .filter_map(move |(i,b)| {
                    if *b == b'.' || (!first_sig_fig_found && *b == b'0') {
                        return None;
                    }
                    first_sig_fig_found = true;
                    Some((b,i))
                })
        }};
    }

    let mut sig_figs_end = str_index;
    let mut sig_figs_index = 0usize;
    let mut round_up = false;

    for (byte, index) in ignore_dot_iter!(written) {
        if sig_figs_index == MAX_SIG_FIGS {
            sig_figs_end = index;
            round_up = *byte >= b'5';
        }
        if sig_figs_index >= MAX_SIG_FIGS {
            *byte = b'0';
        }
        sig_figs_index += 1;
    }

    // Do rounding here
    let mut prepend_one = false;
    if round_up {
        for (byte, index) in ignore_dot_iter!(written[..sig_figs_end]).rev() {
            if *byte == b'9' {
                if index == 0 {
                    prepend_one = true;
                }
                *byte = b'0';
            } else {
                *byte += 1;
                break;
            }
        }
    }

    let mut fstr: Cow<str> = Cow::Borrowed(std::str::from_utf8(&written).expect("should be utf-8"));
    if prepend_one {
        let mut string = String::with_capacity(fstr.len() + 1);
        string += "1";
        string += fstr.as_ref();
        fstr = Cow::Owned(string)
    }

    let f: f64 = fstr.parse().map_err(|e| panic!("can't parse the float we just made `{fstr}` as a float: {e:?}")).unwrap();
    (f as f32) * signum
}

#[cfg(test)]
mod test;
