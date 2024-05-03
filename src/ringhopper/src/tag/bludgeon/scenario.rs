use primitives::{primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::Scenario;

use crate::tag::{scenario::decompile_scripts, verify::scenario::scenario_missing_source_data};
use super::BludgeonResult;

pub fn repair_scenario(tag: &mut dyn PrimaryTagStructDyn, path: &TagPath) -> BludgeonResult {
    let scenario: &mut Scenario = tag.as_any_mut().downcast_mut().unwrap();

    if scenario_missing_source_data(scenario) {
        if decompile_scripts(scenario, path.base_name()).is_err() {
            return BludgeonResult::CannotRepair
        }
    }

    BludgeonResult::Done
}
