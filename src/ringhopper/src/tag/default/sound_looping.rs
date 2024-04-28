use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::SoundLooping;

pub fn set_defaults_for_sound_looping(tag: &mut dyn PrimaryTagStructDyn) {
    let sound: &mut SoundLooping = tag.as_any_mut().downcast_mut().unwrap();
    if sound.zero_detail_sound_period == 0.0 && sound.one_detail_sound_period == 0.0 {
        sound.zero_detail_sound_period = 1.0;
        sound.one_detail_sound_period = 1.0;
    }
}

pub fn unset_defaults_for_sound_looping(tag: &mut dyn PrimaryTagStructDyn) {
    let sound: &mut SoundLooping = tag.as_any_mut().downcast_mut().unwrap();
    if sound.zero_detail_sound_period == 1.0 && sound.one_detail_sound_period == 1.0 {
        sound.zero_detail_sound_period = 0.0;
        sound.one_detail_sound_period = 0.0;
    }
}
