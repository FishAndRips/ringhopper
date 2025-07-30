use primitives::{primitive::Reflexive, tag::PrimaryTagStructDyn};
use ringhopper_structs::*;

pub fn unset_defaults_for_shader_model(tag: &mut dyn PrimaryTagStructDyn) {
    fix_shader_model(tag, true);
}

pub fn set_defaults_for_shader_model(tag: &mut dyn PrimaryTagStructDyn) {
    fix_shader_model(tag, false);
}

pub fn unset_defaults_for_shader_transparent_generic(tag: &mut dyn PrimaryTagStructDyn) {
    fix_shader_transparent_generic(tag, true);
}

pub fn set_defaults_for_shader_transparent_generic(tag: &mut dyn PrimaryTagStructDyn) {
    fix_shader_transparent_generic(tag, false);
}

pub fn unset_defaults_for_shader_transparent_chicago(tag: &mut dyn PrimaryTagStructDyn) {
    fix_shader_transparent_chicago(tag, true);
}

pub fn set_defaults_for_shader_transparent_chicago(tag: &mut dyn PrimaryTagStructDyn) {
    fix_shader_transparent_chicago(tag, false);
}

pub fn unset_defaults_for_shader_transparent_chicago_extended(tag: &mut dyn PrimaryTagStructDyn) {
    fix_shader_transparent_chicago_extended(tag, true);
}

pub fn set_defaults_for_shader_transparent_chicago_extended(tag: &mut dyn PrimaryTagStructDyn) {
    fix_shader_transparent_chicago_extended(tag, false);
}

fn fix_shader_model(tag: &mut dyn PrimaryTagStructDyn, undefault: bool) {
    let shader: &mut ShaderModel = tag.as_any_mut().downcast_mut().unwrap();
    fix_map_scale(&mut shader.maps.map_u_scale, &mut shader.maps.map_v_scale, undefault);
}

fn fix_shader_transparent_generic(tag: &mut dyn PrimaryTagStructDyn, undefault: bool) {
    let shader: &mut ShaderTransparentGeneric = tag.as_any_mut().downcast_mut().unwrap();
    fix_generic_map_reflexive(&mut shader.maps, undefault);
}

fn fix_shader_transparent_chicago(tag: &mut dyn PrimaryTagStructDyn, undefault: bool) {
    let shader: &mut ShaderTransparentChicago = tag.as_any_mut().downcast_mut().unwrap();
    fix_chicago_map_reflexive(&mut shader.maps, undefault);
}

fn fix_shader_transparent_chicago_extended(tag: &mut dyn PrimaryTagStructDyn, undefault: bool) {
    let shader: &mut ShaderTransparentChicagoExtended = tag.as_any_mut().downcast_mut().unwrap();
    fix_chicago_map_reflexive(&mut shader._2_stage_maps, undefault);
    fix_chicago_map_reflexive(&mut shader._4_stage_maps, undefault);
}

fn fix_map_scale(u: &mut f32, v: &mut f32, undefault: bool) {
    if undefault {
        if (*u == 1.0 || *u == 0.0) && (*v == 1.0 || *v == 0.0) {
            *u = 0.0;
            *v = 0.0;
        }
    }
    else if *u == 0.0 && *v == 0.0 {
        *u = 1.0;
        *v = 1.0;
    }
    else {
        if *u == 0.0 {
            *u = *v;
        }
        else if *v == 0.0 {
            *v = *u;
        }
    }
}

fn fix_chicago_map_reflexive(reflexive: &mut Reflexive<ShaderTransparentChicagoMap>, undefault: bool) {
    for i in reflexive {
        fix_map_scale(&mut i.parameters.map_u_scale, &mut i.parameters.map_v_scale, undefault);
    }
}

fn fix_generic_map_reflexive(reflexive: &mut Reflexive<ShaderTransparentGenericMap>, undefault: bool) {
    for i in reflexive {
        fix_map_scale(&mut i.parameters.map_u_scale, &mut i.parameters.map_v_scale, undefault);
    }
}
