use definitions::{GBXModel, Model};
use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::LightVolume;

pub fn set_defaults_for_model(tag: &mut dyn PrimaryTagStructDyn) {
    let model: &mut Model = tag.as_any_mut().downcast_mut().unwrap();


}

pub fn unset_defaults_for_gbxmodel(tag: &mut dyn PrimaryTagStructDyn) {
    let model: &mut GBXModel = tag.as_any_mut().downcast_mut().unwrap();

    for geo in &mut model.geometries {
        for part in &mut geo.parts {
            //part.model_geometry_part
        }
    }
}
