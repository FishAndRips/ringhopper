use primitives::{parse::TagDataDefaults, primitive::TagGroup, tag::PrimaryTagStructDyn};
use ringhopper_structs::group_has_default_in_definitions;

mod object;
mod sound;
mod light;
mod particle;
mod light_volume;
mod shader;
mod lens_flare;

use self::{
    light::*,
    object::*,
    sound::*,
    particle::*,
    light_volume::*,
    shader::*,
    lens_flare::*
};

use super::object::is_object;

type DefaultFn = fn(&mut dyn PrimaryTagStructDyn);

#[derive(Copy, Clone)]
struct DefaultFnHolder {
    default: DefaultFn,
    undefault: DefaultFn
}

fn get_default_fns(group: TagGroup) -> [Option<DefaultFnHolder>; 3] {
    let mut fns: [Option<DefaultFnHolder>; 3] = [None; 3];
    let mut offset = 0usize;

    let mut add_defaulting_fn = |function: DefaultFnHolder| {
        fns[offset] = Some(function);
        offset += 1;
    };

    if group_has_default_in_definitions(group) {
        add_defaulting_fn(DefaultFnHolder { default: TagDataDefaults::set_defaults, undefault: TagDataDefaults::unset_defaults } );
    }

    let default_undefault: Option<(DefaultFn, DefaultFn)> = match group {
        TagGroup::Sound => Some((set_defaults_for_sound, unset_defaults_for_sound)),
        TagGroup::Light => Some((set_defaults_for_light, unset_defaults_for_light)),
        TagGroup::Particle => Some((set_defaults_for_particle, unset_defaults_for_particle)),
        TagGroup::LightVolume => Some((set_defaults_for_light_volume, unset_defaults_for_light_volume)),
        TagGroup::ShaderModel => Some((set_defaults_for_shader_model, unset_defaults_for_shader_model)),
        TagGroup::ShaderTransparentChicago => Some((set_defaults_for_shader_transparent_chicago, unset_defaults_for_shader_transparent_chicago)),
        TagGroup::ShaderTransparentChicagoExtended => Some((set_defaults_for_shader_transparent_chicago_extended, unset_defaults_for_shader_transparent_chicago_extended)),
        TagGroup::ShaderTransparentGeneric => Some((set_defaults_for_shader_transparent_generic, unset_defaults_for_shader_transparent_generic)),
        TagGroup::LensFlare => Some((set_defaults_for_lens_flare, unset_defaults_for_lens_flare)),
        _ => None
    };

    if let Some((default, undefault)) = default_undefault {
        add_defaulting_fn(DefaultFnHolder { default, undefault } );
    }

    if is_object(group) {
        add_defaulting_fn(DefaultFnHolder { default: set_defaults_for_object, undefault: unset_defaults_for_object });
    }

    fns
}

pub fn set_all_defaults_for_tag(tag: &mut dyn PrimaryTagStructDyn) {
    for i in get_default_fns(tag.group()) {
        if let Some(n) = i {
            (n.default)(tag)
        }
    }
}

pub fn unset_all_defaults_for_tag(tag: &mut dyn PrimaryTagStructDyn) {
    for i in get_default_fns(tag.group()) {
        if let Some(n) = i {
            (n.undefault)(tag)
        }
    }
}

pub fn group_has_defaults(group: TagGroup) -> bool {
    get_default_fns(group)[0].is_some()
}
