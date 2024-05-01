use primitives::{dynamic::DynamicTagDataArray, primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::{Sound, SoundChannelCount, SoundFormat, SoundPermutation, SoundPitchRange};

use crate::tag::{sound::{sample_rate_to_u32, SoundPermutationMetadata}, tree::TagTree};

use super::{VerifyContext, VerifyResult};

pub fn verify_sound<T: TagTree + Send + Sync>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &VerifyContext<T>, result: &mut VerifyResult) {
    let sound: &Sound = tag.as_any().downcast_ref().unwrap();

    let error_count_start = result.errors.len();
    check_errors_with_sound(sound, result);
    if result.errors.len() != error_count_start {
        if sound_tag_is_fubar(sound) {
            result.errors.push("These errors can NOT be automatically repaired.".to_owned());
        }
        else {
            result.errors.push("These errors can be automatically repaired with the bludgeon command.".to_owned());
        }
    }

    for (p, pitch_range) in ziperator!(sound.pitch_ranges) {
        let actual_natural_pitch = if pitch_range.natural_pitch <= 0.0 { 1.0 } else { pitch_range.natural_pitch };

        let pitch_range_ok = if pitch_range.bend_bounds.lower > actual_natural_pitch && pitch_range.bend_bounds.lower != 0.0 {
            false
        }
        else if pitch_range.bend_bounds.upper < actual_natural_pitch && pitch_range.bend_bounds.upper != 0.0 {
            false
        }
        else {
            true
        };

        if !pitch_range_ok {
            result.warnings.push(format!("Pitch range #{p}'s bend bounds does not fit the natural pitch value ({actual_natural_pitch}) and will be adjusted."));
        }
    }
}

pub fn sound_is_playable(sound: &Sound) -> bool {
    let mut result = VerifyResult::default();
    check_errors_with_sound(sound, &mut result);
    result.is_ok()
}

fn check_errors_with_sound(sound: &Sound, verify_result: &mut VerifyResult) {
    verify_split_permutation_flag_should_be_set_but_is_not(sound, verify_result);
    verify_sound_permutation_indices(sound, verify_result);
    verify_sound_formats(sound, verify_result);
    verify_sound_block_size(sound, verify_result);
    verify_sound_metadata(sound, verify_result);
    verify_actual_permutation_count_is_correctly_set(sound, verify_result);
}

pub(crate) fn sound_tag_actually_contains_split_permutations(sound: &Sound) -> bool {
    if sound.flags.split_long_sound_into_permutations {
        return true;
    }

    for pitch_range in &sound.pitch_ranges {
        for permutation in &pitch_range.permutations {
            if next_subpermutation_index(&permutation).is_some() {
                return true;
            }
        }
    }

    false
}

fn verify_split_permutation_flag_should_be_set_but_is_not(sound: &Sound, verify_result: &mut VerifyResult) {
    if sound.flags.split_long_sound_into_permutations != sound_tag_actually_contains_split_permutations(sound) {
        verify_result.errors.push("Detected split permutations, but the split permutations flag isn't set".to_owned());
    }
}


/// Return `true` if the sound tag is fucked up beyond all repair.
pub(crate) fn sound_tag_is_fubar(sound: &Sound) -> bool {
    let mut result = VerifyResult::default();

    verify_sound_block_size(sound, &mut result);

    if !result.is_ok() {
        return true;
    }

    if verify_sound_metadata(sound, &mut result) == Some(false) {
        return true;
    }

    if verify_sound_formats(sound, &mut result) == Some(false) {
        return true;
    }

    false
}

/// Return `true` if correctly set, and false if not
fn verify_actual_permutation_count_is_correctly_set(sound: &Sound, result: &mut VerifyResult) -> bool {
    let mut issues_found = false;

    for (pr, pitch_range) in ziperator!(sound.pitch_ranges) {
        let actual = find_actual_permutation_count(sound, pitch_range);
        let expected = pitch_range.actual_permutation_count;
        if expected != actual {
            result.errors.push(format!("Actual permutation count of pitch range #{pr} is incorrect (expected {expected} but calculated {actual})"));
            issues_found = true;
        }
    }

    !issues_found
}

fn next_subpermutation_index(permutation: &SoundPermutation) -> Option<u16> {
    match permutation.next_permutation_index {
        Some(0) | None => None,
        Some(n) => Some(n)
    }
}

/// Return `None` if the permutation count cannot be calculated.
pub(crate) fn find_actual_permutation_count(sound: &Sound, pitch_range: &SoundPitchRange) -> u16 {
    let permutation_count_16_bit = u16::try_from(pitch_range.permutations.items.len()).expect("should be < 65535");
    let split_permutations = sound_tag_actually_contains_split_permutations(sound);

    if split_permutations {
        // Go through every index and find the lowest next_permutation_index value.
        let mut lowest_next_permutation_index = permutation_count_16_bit;

        for i in &pitch_range.permutations {
            if let Some(n) = next_subpermutation_index(i) {
                lowest_next_permutation_index = lowest_next_permutation_index.min(n);
            }
        }

        lowest_next_permutation_index
    }
    else {
        permutation_count_16_bit
    }
}

fn verify_sound_block_size(sound: &Sound, result: &mut VerifyResult) {
    let channel_count = match sound.channel_count {
        SoundChannelCount::Mono => 1,
        SoundChannelCount::Stereo => 2
    };

    for (pr, pitch_range) in ziperator!(sound.pitch_ranges) {
        for (pe, permutation) in ziperator!(pitch_range.permutations) {
            match permutation.format {
                SoundFormat::PCM => {
                    let data = permutation.samples.bytes.len();
                    let sample_size = 2 * channel_count;
                    if (data % sample_size) != 0 {
                        result.errors.push(format!("Permutation #{pe} (`{}`) of pitch range #{pr} has an incorrect size (not divisible by {sample_size})", permutation.name.as_str()));
                        continue;
                    }
                },
                SoundFormat::XboxADPCM => {
                    let block_size = 36 * channel_count;
                    let data = permutation.samples.bytes.len();
                    if (data % block_size) != 0 {
                        result.errors.push(format!("Permutation #{pe} (`{}`) of pitch range #{pr} has an incorrect size (not divisible by {block_size})", permutation.name.as_str()));
                        continue;
                    }
                }
                _ => ()
            }
        }
    }
}

/// Return `true` if any issues are repairable, or `None` if no issues were found.
fn verify_sound_metadata(sound: &Sound, result: &mut VerifyResult) -> Option<bool> {
    let mut first_sample_rate = None;
    let mut sample_rate_repairable = true;

    let mut first_channel_count = None;
    let mut channel_count_repairable = true;

    let mut all_metadata_successfully_queried = true;

    let issues_before = result.errors.len();

    for (pr, pitch_range) in ziperator!(sound.pitch_ranges) {
        for (pe, permutation) in ziperator!(pitch_range.permutations) {
            let metadata = match SoundPermutationMetadata::read_from_sound_permutation(sound, permutation) {
                Ok(n) => n,
                Err(e) => {
                    result.errors.push(format!("Permutation #{pe} (`{}`) of pitch range #{pr} had an error while querying sound metadata: {e}", permutation.name.as_str()));
                    all_metadata_successfully_queried = false;
                    continue;
                }
            };

            let sample_rate_opt = Some(metadata.sample_rate);
            if first_sample_rate.is_none() {
                first_sample_rate = sample_rate_opt;
            }
            else if first_sample_rate != sample_rate_opt {
                sample_rate_repairable = false;
            }

            let channel_count_opt = Some(metadata.channel_count);
            if first_channel_count.is_none() {
                first_channel_count = channel_count_opt;
            }
            else if first_channel_count != channel_count_opt {
                channel_count_repairable = false;
            }

            if sound.channel_count != metadata.channel_count {
                result.errors.push(format!("Permutation #{pe} (`{}`) of pitch range #{pr} has a mismatched channel count (permutation is {}, where sound tag is {})", permutation.name.as_str(), metadata.channel_count, sound.channel_count));
            }

            if sound.sample_rate != metadata.sample_rate {
                result.errors.push(format!("Permutation #{pe} (`{}`) of pitch range #{pr} has a mismatched sample rate (permutation is {} Hz, where sound tag is {} Hz)", permutation.name.as_str(), sample_rate_to_u32(metadata.sample_rate), sample_rate_to_u32(sound.sample_rate)));
            }

            if permutation.buffer_size != metadata.buffer_size {
                result.errors.push(format!("Permutation #{pe} (`{}`) of pitch range #{pr} has a mismatched buffer size (permutation is {}, where sound tag is {})", permutation.name.as_str(), metadata.buffer_size, permutation.buffer_size));
            }
        }
    }

    let issues_after = result.errors.len();

    if issues_after == issues_before {
        None
    }
    else {
        Some(all_metadata_successfully_queried && channel_count_repairable && sample_rate_repairable)
    }
}

/// Return `true` if the sound tag can be repaired, and `None` if no issues were found.
pub(crate) fn verify_sound_formats(sound: &Sound, result: &mut VerifyResult) -> Option<bool> {
    let mut first_format = None;
    let mut format_repairable = true;
    let mut issues_found = false;

    let expected = sound.format;
    for (pr, pitch_range) in ziperator!(sound.pitch_ranges) {
        for (pe, permutation) in ziperator!(pitch_range.permutations) {
            let actual = permutation.format;

            let actual_opt = Some(actual);
            if first_format.is_none() {
                first_format = actual_opt;
            }
            else if actual_opt != first_format {
                format_repairable = false;
            }

            if expected != actual {
                result.errors.push(format!("Permutation #{pe} (`{}`) of pitch range #{pr} has a mismatched sound format (expected {expected}, got {actual} instead)", permutation.name.as_str()));
                issues_found = true;
            }
        }
    }

    if !issues_found {
        None
    }
    else {
        Some(format_repairable)
    }
}

fn verify_sound_permutation_indices(sound: &Sound, result: &mut VerifyResult) {
    let split_permutations = sound_tag_actually_contains_split_permutations(sound);

    'l: for pitch_range in &sound.pitch_ranges {
        for permutation in &pitch_range.permutations {
            if permutation.next_permutation_index == Some(0) {
                result.errors.push("Sound tag contains permutations with next_permutation_index set to 0. This is invalid.".to_owned());
                break 'l;
            }
        }
    }

    if split_permutations {
        for (pr, pitch_range) in ziperator!(sound.pitch_ranges) {
            let subpermutation_count = pitch_range.permutations.len();
            let actual_permutation_count = find_actual_permutation_count(sound, pitch_range) as usize;
            let actual_permutations = &pitch_range.permutations.items[..actual_permutation_count];
            let mut used: Vec<bool> = vec![false; subpermutation_count];

            'permutation_loop:
            for (ap, permutation) in (0..actual_permutations.len()).zip(actual_permutations.iter()) {
                let mut traversed = 0usize;

                let mut permutation_index = ap;
                loop {
                    if permutation_index >= subpermutation_count {
                        result.errors.push(format!("Permutation #{ap} (`{}`) of pitch range #{pr} has out-of-bounds permutations", permutation.name.as_str()));
                        continue 'permutation_loop;
                    }

                    traversed += 1;
                    if traversed > subpermutation_count {
                        result.errors.push(format!("Permutation #{ap} (`{}`) of pitch range #{pr} infinitely loops", permutation.name.as_str()));
                        continue 'permutation_loop;
                    }

                    used[permutation_index] = true;
                    permutation_index = match next_subpermutation_index(&pitch_range.permutations.items[permutation_index]) {
                        Some(n) => n as usize,
                        None => break
                    };
                }
            }

            let unused_permutations_total = used.into_iter().filter(|&p| p == false).count();
            if unused_permutations_total > 0 {
                result.warnings.push(format!("Pitch range #{pr} contains {unused_permutations_total} unused subpermutation(s)"));
            }
        }
    }
}
