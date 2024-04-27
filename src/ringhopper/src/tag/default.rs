use primitives::{primitive::TagGroup, tag::PrimaryTagStructDyn};

mod object;
mod sound;
mod light;
mod particle;

use self::{
    light::*,
    object::*,
    sound::*,
    particle::*
};

use super::object::is_object;

pub fn set_all_defaults_for_tag(tag: &mut dyn PrimaryTagStructDyn) {
    tag.set_defaults();

    match tag.group() {
        TagGroup::Sound => set_defaults_for_sound(tag),
        TagGroup::Light => set_defaults_for_light(tag),
        TagGroup::Particle => set_defaults_for_particle(tag),
        _ => ()
    }

    if is_object(tag.group()) {
        set_defaults_for_object(tag);
    }
}

pub fn unset_all_defaults_for_tag(tag: &mut dyn PrimaryTagStructDyn) {
    tag.unset_defaults();

    match tag.group() {
        TagGroup::Sound => unset_defaults_for_sound(tag),
        TagGroup::Light => unset_defaults_for_light(tag),
        TagGroup::Particle => unset_defaults_for_particle(tag),
        _ => ()
    }

    if is_object(tag.group()) {
        unset_defaults_for_object(tag);
    }
}
