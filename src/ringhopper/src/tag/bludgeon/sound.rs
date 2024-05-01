use primitives::{dynamic::DynamicTagDataArray, tag::PrimaryTagStructDyn};
use ringhopper_structs::Sound;

use crate::tag::{sound::SoundPermutationMetadata, verify::sound::{find_actual_permutation_count, sound_tag_actually_contains_split_permutations, sound_tag_is_fubar}};

use super::BludgeonResult;

pub fn repair_sound(tag: &mut dyn PrimaryTagStructDyn) -> BludgeonResult {
    let sound: &mut Sound = tag.as_any_mut().downcast_mut().unwrap();

    if sound_tag_is_fubar(sound) {
        return BludgeonResult::CannotRepair;
    }

    sound.flags.split_long_sound_into_permutations |= sound_tag_actually_contains_split_permutations(sound);

    for pr in 0..sound.pitch_ranges.len() {
        sound.pitch_ranges.items[pr].actual_permutation_count = find_actual_permutation_count(sound, &sound.pitch_ranges.items[pr]);

        for pe in 0..sound.pitch_ranges.items[pr].permutations.items.len() {
            let metadata = SoundPermutationMetadata::read_from_sound_permutation(sound, &sound.pitch_ranges.items[pr].permutations.items[pe])
                .expect("should be able to get metadata if sound tag is not fubar");

            let permutation = &mut sound.pitch_ranges.items[pr].permutations.items[pe];
            permutation.buffer_size = metadata.buffer_size;
            sound.format = permutation.format;
            sound.sample_rate = metadata.sample_rate;
            sound.channel_count = metadata.channel_count;

            if permutation.next_permutation_index == Some(0) {
                permutation.next_permutation_index = None;
            }
        }
    }

    BludgeonResult::Done
}
