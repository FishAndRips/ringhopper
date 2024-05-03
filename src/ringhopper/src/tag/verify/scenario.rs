use primitives::{primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::Scenario;

use crate::tag::tree::TagTree;

use super::{VerifyContext, VerifyResult};

pub fn verify_scenario<T: TagTree + Send + Sync + 'static>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &VerifyContext<T>, result: &mut VerifyResult) {
    let scenario: &Scenario = tag.as_any().downcast_ref().unwrap();
    if scenario_missing_source_data(scenario) {
        result.errors.push("No source data, but scripts/globals detected".to_owned())
    }
}

pub fn scenario_missing_source_data(tag: &Scenario) -> bool {
    tag.source_files.items.is_empty() && !(tag.scripts.items.is_empty() && tag.globals.items.is_empty())
}
