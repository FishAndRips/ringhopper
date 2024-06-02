use definitions::{GBXModel, Model, ModelGeometryPart};
use primitives::tag::PrimaryTagStructDyn;

// If BOTH next filthy part index are 0, then it should be defaulted to 255.
//
// And if both are 255, it should be undefaulted to 0.
//
// The HEK tool.exe is bugged and defaults them individually.

pub fn set_defaults_for_model_geometry_part(part: &mut ModelGeometryPart) {
    if part.next_filthy_part_index == 0 && part.prev_filthy_part_index == 0 {
        part.next_filthy_part_index = 255;
        part.prev_filthy_part_index = 255;
    }
}

pub fn unset_defaults_for_model_geometry_part(part: &mut ModelGeometryPart) {
    if part.next_filthy_part_index == 255 && part.prev_filthy_part_index == 255 {
        part.next_filthy_part_index = 0;
        part.prev_filthy_part_index = 0;
    }
}

pub fn set_defaults_for_model(tag: &mut dyn PrimaryTagStructDyn) {
    let model: &mut Model = tag.as_any_mut().downcast_mut().unwrap();

    for geo in &mut model.geometries {
        for part in &mut geo.parts {
            set_defaults_for_model_geometry_part(part);
        }
    }
}

pub fn unset_defaults_for_model(tag: &mut dyn PrimaryTagStructDyn) {
    let model: &mut Model = tag.as_any_mut().downcast_mut().unwrap();

    for geo in &mut model.geometries {
        for part in &mut geo.parts {
            unset_defaults_for_model_geometry_part(part);
        }
    }
}

pub fn set_defaults_for_gbxmodel(tag: &mut dyn PrimaryTagStructDyn) {
    let model: &mut GBXModel = tag.as_any_mut().downcast_mut().unwrap();

    for geo in &mut model.geometries {
        for part in &mut geo.parts {
            set_defaults_for_model_geometry_part(&mut part.model_geometry_part);
        }
    }
}

pub fn unset_defaults_for_gbxmodel(tag: &mut dyn PrimaryTagStructDyn) {
    let model: &mut GBXModel = tag.as_any_mut().downcast_mut().unwrap();

    for geo in &mut model.geometries {
        for part in &mut geo.parts {
            unset_defaults_for_model_geometry_part(&mut part.model_geometry_part);
        }
    }
}
