use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::Particle;

pub fn unset_defaults_for_particle(tag: &mut dyn PrimaryTagStructDyn) {
    let particle: &mut Particle = tag.as_any_mut().downcast_mut().unwrap();

    if particle.radius_animation.lower == 0.0 {
        particle.radius_animation.lower = 1.0;
    }
    if particle.radius_animation.upper == 0.0 {
        particle.radius_animation.upper = 1.0;
    }

    if (particle.fade_start_size == 5.0 || particle.fade_start_size == 0.0) && (particle.fade_end_size == 4.0 || particle.fade_end_size == 0.0) {
        particle.fade_start_size = 0.0;
        particle.fade_end_size = 0.0;
    }
}

pub fn set_defaults_for_particle(tag: &mut dyn PrimaryTagStructDyn) {
    let particle: &mut Particle = tag.as_any_mut().downcast_mut().unwrap();

    if particle.radius_animation.lower == 1.0 {
        particle.radius_animation.lower = 0.0;
    }
    if particle.radius_animation.upper == 1.0 {
        particle.radius_animation.upper = 0.0;
    }

    // This is a weird behavior with tool.exe, since it can unset values you might've wanted, so the verification step
    // will warn here.
    if particle.fade_start_size == 0.0 || particle.fade_end_size == 0.0 {
        particle.fade_start_size = 5.0;
        particle.fade_end_size = 4.0;
    }
}
