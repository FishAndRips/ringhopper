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

        let mut value = None;
        let mut all_same = true;
        for p in &mut cc.permutations {
            nudge(&mut p.weight, &mut result);
            match value {
                Some(n) => if n != p.weight { all_same = false; }
                None => value = Some(p.weight)
            }
        }

        if all_same && value != Some(1.0) {
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
    let fixed = fix_rounding_for_float(*float);
    if fixed != *float {
        *float = fixed;
        *was_nudged_thus_far |= true;
    }
}

/// Fix the rounding for a floating point number.
pub(crate) fn fix_rounding_for_float(float: f32) -> f32 {
    let med = fixed_med!(float);
    let mut bits = med.to_bits();

    let very_low_bits = bits & 0xFFFFF;
    if very_low_bits.count_ones() < 3 {
        bits -= very_low_bits;
        return FixedPrecision::from_bits(bits).to_num();
    }
    if (very_low_bits & 0xF0000) == 0xF0000 {
        bits += 0x100000 - very_low_bits;
        return FixedPrecision::from_bits(bits).to_num();
    }

    float
}

#[cfg(test)]
mod test;