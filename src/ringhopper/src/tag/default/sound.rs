use std::collections::HashMap;

use primitives::error::{Error, RinghopperResult};
use primitives::primitive::{Bounds, TagGroup};
use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::{Sound, SoundClass, SoundLooping};

use crate::tag::tree::TagTree;

pub fn set_defaults_for_sound(tag: &mut dyn PrimaryTagStructDyn) {
    set_or_unset_defaults_for_sound(tag, false);
}

pub fn unset_defaults_for_sound(tag: &mut dyn PrimaryTagStructDyn) {
    set_or_unset_defaults_for_sound(tag, true);
}

fn set_or_unset_defaults_for_sound(tag: &mut dyn PrimaryTagStructDyn, undefault: bool) {
    let sound: &mut Sound = tag.as_any_mut().downcast_mut().unwrap();
    let defaults = default_min_max_distance_sounds(sound.sound_class);

    let default = |value: &mut [&mut f32], default: &[f32]| {
        if undefault {
            // Check for defaults
            for (val, default) in value.iter().zip(default.iter()) {
                if **val != *default {
                    return;
                }
            }
            // Undefault
            for val in value.iter_mut() {
                **val = 0.0;
            }
        }
        else {
            // Check for all zeroes
            for val in value.iter() {
                if **val != 0.0 {
                    return;
                }
            }
            // Default
            for (val, default) in value.iter_mut().zip(default.iter()) {
                **val = *default
            }
        }
    };

    let use_subpermutations = sound.flags.split_long_sound_into_permutations;

    for pitch_range in &mut sound.pitch_ranges {
        let actual_natural_pitch = if pitch_range.natural_pitch <= 0.0 { 1.0 } else { pitch_range.natural_pitch };

        // If the lower value is greater than actual_natural_pitch, or upper is less than it, those respective value(s)
        // get set to actual_natural_pitch.
        if undefault {
            if pitch_range.bend_bounds.lower > actual_natural_pitch || pitch_range.bend_bounds.lower == actual_natural_pitch {
                pitch_range.bend_bounds.lower = 0.0;
            }
            if pitch_range.bend_bounds.upper < actual_natural_pitch || pitch_range.bend_bounds.upper == actual_natural_pitch {
                pitch_range.bend_bounds.upper = 0.0;
            }
        }
        else {
            if pitch_range.bend_bounds.lower > actual_natural_pitch {
                pitch_range.bend_bounds.lower = actual_natural_pitch;
            }
            if pitch_range.bend_bounds.upper < actual_natural_pitch {
                pitch_range.bend_bounds.upper = actual_natural_pitch;
            }
        }

        let actual_permutation_count = if use_subpermutations {
            pitch_range.actual_permutation_count as usize
        }
        else {
            pitch_range.permutations.items.len()
        };

        // Bad value set
        if pitch_range.permutations.items.len() < actual_permutation_count {
            continue;
        }

        // Default/undefault the primary fields
        for i in &mut pitch_range.permutations.items[..actual_permutation_count] {
            default(&mut [&mut i.gain], &[1.0])
        }

        if use_subpermutations {
            if undefault {
                let subpermutations = match pitch_range.permutations.items.get_mut(actual_permutation_count..) {
                    Some(n) => n,
                    None => continue
                };
                for i in subpermutations {
                    i.gain = 0.0;
                }
            }
            else {
                for p in 0..actual_permutation_count {
                    let main = &pitch_range.permutations.items[p];
                    let gain = main.gain;

                    let mut found: HashMap<u16, bool> = HashMap::new();
                    let mut next = main.next_permutation_index;
                    loop {
                        // Done
                        let index = match next {
                            Some(n) => n,
                            None => break
                        };

                        // Infinite loop
                        if found.contains_key(&index) {
                            break;
                        }

                        found.insert(index, true);
                        match pitch_range.permutations.items.get_mut(index as usize) {
                            Some(n) => {
                                n.gain = gain;
                                next = n.next_permutation_index;
                            }
                            None => break
                        };
                    }
                }
            }
        }
        else if actual_permutation_count < u16::MAX as usize {
            if undefault {
                pitch_range.actual_permutation_count = 0;
            }
            else {
                pitch_range.actual_permutation_count = actual_permutation_count as u16;
            }
        }
    }

    default(&mut [&mut sound.distance_bounds.lower], &[defaults.lower]);
    default(&mut [&mut sound.distance_bounds.upper], &[defaults.upper]);
    default(&mut [&mut sound.random_pitch_bounds.lower], &[1.0]);
    default(&mut [&mut sound.random_pitch_bounds.upper], &[1.0]);
    default(&mut [&mut sound.zero_skip_fraction_modifier, &mut sound.one_skip_fraction_modifier], &[1.0, 1.0]);
    default(&mut [&mut sound.zero_pitch_modifier, &mut sound.one_pitch_modifier], &[1.0, 1.0]);
    default(&mut [&mut sound.zero_gain_modifier, &mut sound.one_gain_modifier], &[default_zero_gain_modifier_for_class(sound.sound_class), 1.0]);


}

fn default_zero_gain_modifier_for_class(sound_class: SoundClass) -> f32 {
    match sound_class {
        SoundClass::ObjectImpacts
        | SoundClass::ParticleImpacts
        | SoundClass::SlowParticleImpacts
        | SoundClass::UnitDialog
        | SoundClass::Music
        | SoundClass::AmbientNature
        | SoundClass::AmbientMachinery
        | SoundClass::AmbientComputers
        | SoundClass::ScriptedDialogPlayer
        | SoundClass::ScriptedDialogOther
        | SoundClass::ScriptedDialogForceUnspatialized
        | SoundClass::ScriptedEffect => 0.0,

        _ => 1.0
    }
}

#[allow(dead_code)] // TODO: remove this allow(dead_code) directive when build code is added
fn calculate_sound_looping_distance<T: TagTree>(sound_looping: &SoundLooping, tag_tree: &T) -> RinghopperResult<f32> {
    let mut maximum_distance: f32 = 0.0;

    let detail_sounds = sound_looping
        .detail_sounds
        .items
        .iter()
        .map(|t| &t.sound);

    let tracks = sound_looping
        .tracks
        .items
        .iter()
        .map(|t| [&t.alternate_end, &t.alternate_loop, &t._loop, &t.end, &t.start])
        .flatten();

    let all_sounds = detail_sounds
        .chain(tracks)
        .filter_map(|t| t.path());

    for i in all_sounds {
        if i.group() != TagGroup::Sound {
            return Err(Error::InvalidTagData("sound_looping tag contains non-sound references where there should be sounds!".to_owned()));
        }

        let sound_tag = tag_tree.open_tag_shared(i)?;
        let lock = sound_tag.lock().unwrap();
        let sound: &Sound = lock.as_any().downcast_ref().unwrap();
        maximum_distance = maximum_distance.max(sound.distance_bounds.upper);
    }

    Ok(maximum_distance)
}

fn default_min_max_distance_sounds(sound_class: SoundClass) -> Bounds<f32> {
    match sound_class {
        SoundClass::DeviceMachinery
        | SoundClass::DeviceForceField
        | SoundClass::AmbientMachinery
        | SoundClass::AmbientNature
        | SoundClass::DeviceDoor
        | SoundClass::Music
        | SoundClass::DeviceNature => Bounds { lower: 0.9, upper: 5.0 },

        SoundClass::WeaponEmpty
        | SoundClass::WeaponIdle
        | SoundClass::WeaponReady
        | SoundClass::WeaponReload
        | SoundClass::WeaponCharge
        | SoundClass::WeaponOverheat => Bounds { lower: 1.0, upper: 9.0 },

        SoundClass::ScriptedDialogOther
        | SoundClass::ScriptedDialogPlayer
        | SoundClass::GameEvent
        | SoundClass::UnitDialog
        | SoundClass::ScriptedDialogForceUnspatialized => Bounds { lower: 3.0, upper: 20.0 },

        SoundClass::FirstPersonDamage
        | SoundClass::ObjectImpacts
        | SoundClass::AmbientComputers
        | SoundClass::ParticleImpacts
        | SoundClass::DeviceComputers
        | SoundClass::SlowParticleImpacts => Bounds { lower: 0.5, upper: 3.0 },

        SoundClass::VehicleEngine
        | SoundClass::ProjectileImpact
        | SoundClass::VehicleCollision => Bounds { lower: 1.4, upper: 8.0 },

        SoundClass::WeaponFire => Bounds { lower: 4.0, upper: 70.0 },
        SoundClass::ScriptedEffect => Bounds { lower: 2.0, upper: 5.0 },
        SoundClass::ProjectileDetonation => Bounds { lower: 8.0, upper: 120.0 },
        SoundClass::UnitFootsteps => Bounds { lower: 0.9, upper: 10.0 }
    }
}
