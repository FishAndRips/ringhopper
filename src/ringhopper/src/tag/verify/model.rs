use primitives::{primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::{GBXModel, Model};
use crate::tag::tree::TagTree;
use crate::tag::model::ModelFunctions;
use super::{VerifyContext, VerifyResult};

macro_rules! write_verify_model_fn {
    ($name:tt, $group:tt) => {
        pub fn $name<T: TagTree>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &mut VerifyContext<T>, result: &mut VerifyResult) {
            let model: &$group = tag.as_any().downcast_ref().unwrap();

            if let Err(e) = model.check_indices() {
                result.errors.push(format!("Error occurred while checking vertex indices: {e}"));
            }

            if !model.runtime_markers.items.is_empty() {
                result.errors.push("Runtime markers are non-empty. This tag is invalid and needs fixed!".to_string());
            }
        }
    };
}

write_verify_model_fn!(verify_model, Model);
write_verify_model_fn!(verify_gbxmodel, GBXModel);
