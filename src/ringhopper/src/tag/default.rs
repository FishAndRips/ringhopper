use primitives::{primitive::TagGroup, tag::PrimaryTagStructDyn};

mod object;
mod sound;
mod light;
mod particle;
mod light_volume;
mod shader;

use self::{
    light::*,
    object::*,
    sound::*,
    particle::*,
    light_volume::*,
    shader::*
};

use super::object::is_object;

pub fn set_all_defaults_for_tag(tag: &mut dyn PrimaryTagStructDyn) {
    tag.set_defaults();

    match tag.group() {
        TagGroup::Sound => set_defaults_for_sound(tag),
        TagGroup::Light => set_defaults_for_light(tag),
        TagGroup::Particle => set_defaults_for_particle(tag),
        TagGroup::LightVolume => set_defaults_for_light_volume(tag),
        TagGroup::ShaderModel => set_defaults_for_shader_model(tag),
        TagGroup::ShaderTransparentChicago => set_defaults_for_shader_transparent_chicago(tag),
        TagGroup::ShaderTransparentChicagoExtended => set_defaults_for_shader_transparent_chicago_extended(tag),
        TagGroup::ShaderTransparentGeneric => set_defaults_for_shader_transparent_generic(tag),
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
        TagGroup::LightVolume => unset_defaults_for_light_volume(tag),
        TagGroup::ShaderModel => unset_defaults_for_shader_model(tag),
        TagGroup::ShaderTransparentChicago => unset_defaults_for_shader_transparent_chicago(tag),
        TagGroup::ShaderTransparentChicagoExtended => unset_defaults_for_shader_transparent_chicago_extended(tag),
        TagGroup::ShaderTransparentGeneric => unset_defaults_for_shader_transparent_generic(tag),
        _ => ()
    }

    if is_object(tag.group()) {
        unset_defaults_for_object(tag);
    }
}
