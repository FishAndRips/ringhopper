use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::LightVolume;

pub fn set_defaults_for_light_volume(tag: &mut dyn PrimaryTagStructDyn) {
    let light_volume: &mut LightVolume = tag.as_any_mut().downcast_mut().unwrap();

    if light_volume.perpendicular_brightness_scale == 0.0 && light_volume.parallel_brightness_scale == 0.0 {
        light_volume.parallel_brightness_scale = 1.0;
        light_volume.perpendicular_brightness_scale = 1.0;
    }
}

pub fn unset_defaults_for_light_volume(tag: &mut dyn PrimaryTagStructDyn) {
    let light_volume: &mut LightVolume = tag.as_any_mut().downcast_mut().unwrap();

    if light_volume.perpendicular_brightness_scale == 1.0 && light_volume.parallel_brightness_scale == 1.0 {
        light_volume.parallel_brightness_scale = 0.0;
        light_volume.perpendicular_brightness_scale = 0.0;
    }
}
