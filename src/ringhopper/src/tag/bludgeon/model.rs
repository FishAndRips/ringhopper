use primitives::{primitive::TagGroup, tag::PrimaryTagStructDyn};
use ringhopper_structs::{GBXModel, Model};

use crate::tag::model::ModelFunctions;
use super::BludgeonResult;

pub fn repair_model(tag: &mut dyn PrimaryTagStructDyn) -> BludgeonResult {
    let model_fns: &mut dyn ModelFunctions = match tag.group() {
        TagGroup::Model => tag.as_any_mut().downcast_mut::<Model>().unwrap(),
        TagGroup::GBXModel => tag.as_any_mut().downcast_mut::<GBXModel>().unwrap(),
        g => unreachable!("can't repair non-model group {g}")
    };

    if model_fns.fix_runtime_markers().is_err() {
        return BludgeonResult::CannotRepair
    }

    model_fns.fix_compressed_vertices();
    model_fns.fix_uncompressed_vertices();

    BludgeonResult::Done
}
