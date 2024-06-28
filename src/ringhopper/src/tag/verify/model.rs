use primitives::{primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::{GBXModel, Model};
use crate::tag::tree::TagTree;
use crate::tag::model::ModelFunctions;
use crate::primitives::primitive::Vector;
use super::{ScenarioContext, ScenarioTreeTagResult};

macro_rules! write_verify_model_fn {
    ($name:tt, $group:tt) => {
        pub fn $name<T: TagTree + Send + Sync>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &ScenarioContext<T>, result: &mut ScenarioTreeTagResult) {
            let model: &$group = tag.as_any().downcast_ref().unwrap();

            if let Err(e) = model.check_indices() {
                result.errors.push(format!("Error occurred while checking vertex indices: {e}"));
            }

            if !model.runtime_markers.items.is_empty() {
                result.errors.push("Runtime markers are non-empty. This tag is invalid and needs fixed! This can be automatically repaired with the bludgeon command.".to_string());
            }

            let mut contains_non_normal_vectors = false;
            for node in model.nodes() {
                if !node.default_rotation.is_unit_vector() {
                    contains_non_normal_vectors = true;
                }
            }

            'outer: for region in model.regions() {
                for permutation in &region.permutations {
                    for marker in &permutation.markers {
                        if !marker.rotation.is_unit_vector() {
                            contains_non_normal_vectors = true;
                            break 'outer;
                        }
                    }
                }
            }

            if contains_non_normal_vectors {
                result.errors.push("Non-normal rotation vectors detected. This can be automatically repaired with the bludgeon command.".to_owned());
            }
        }
    };
}

write_verify_model_fn!(verify_model, Model);
write_verify_model_fn!(verify_gbxmodel, GBXModel);
