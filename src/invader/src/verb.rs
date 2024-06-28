use std::collections::HashMap;
use std::env::Args;
use ringhopper::primitives::primitive::TagPath;
use ringhopper::tag::result::ScenarioTreeTagResult;
use util::LockedStdoutLogger;

macro_rules! str_unwrap {
    ($what:expr, $($fmt:tt)+) => {
        ($what).map_err(|e| format!($($fmt)+, error=e.to_string()))?
    };
}

mod dependency_list;
mod version;
mod unicode_strings;
mod strip;
mod tag_collection;
mod nudge;
mod compare;
mod convert;
mod extract;
mod list_engines;
mod undefault;
mod plate;
mod archive;
mod recover;
mod verify_scenario;
mod refactor_groups;
mod bludgeon;
mod recompress_vertices;
mod dependency_tree;
mod refactor_paths;

pub struct Verb {
    pub name: &'static str,
    pub description: &'static str,
    pub function: fn(Args, &'static str) -> Result<(), String>
}

impl Verb {
    const fn new(name: &'static str, description: &'static str, function: fn(Args, &'static str) -> Result<(), String>) -> Self {
        Self {
            name, description, function
        }
    }
}

pub const ALL_VERBS: &'static [Verb] = &[
    Verb::new("archive-scenario", "Create a .7z of a map's tag structure", archive::archive_scenario),
    Verb::new("archive-tag", "Create a .7z of a tag and its dependencies", archive::archive_tag),
    Verb::new("bludgeon", "Automatically repair common issues with tags", bludgeon::bludgeon),
    Verb::new("compare", "Compare tags between two tag sources", compare::compare),
    Verb::new("convert", "Convert tags to another tag group", convert::convert),
    Verb::new("dependency-list", "View dependencies of tags", dependency_list::dependency_list),
    Verb::new("dependency-tree", "View dependencies of a tag in a recursive tree", dependency_tree::dependency_tree),
    Verb::new("extract", "Extract tags from a map", extract::extract),
    Verb::new("list-engines", "List all available engine targets", list_engines::list_engines),
    Verb::new("list-scenario-tags", "View all tags needed to build a scenario into a map", dependency_list::list_scenario_tags),
    Verb::new("nudge", "Fix floating point precision errors from tag extraction", nudge::nudge),
    Verb::new("plate", "Generate color plates for bitmaps", plate::plate),
    Verb::new("recompress-vertices", "Recompress model vertices", recompress_vertices::recompress_vertices),
    Verb::new("recover", "Recover data from tags", recover::recover),
    Verb::new("refactor-groups", "Batch refactor dependencies by tag group if the new dependency exists", refactor_groups::refactor_groups),
    Verb::new("refactor-paths", "Batch refactor dependencies by tag path (file extensions cannot be changed)", refactor_paths::refactor_paths),
    Verb::new("strip", "Clean tags", strip::strip),
    Verb::new("tag-collection", "Generate tag_collection tags from data", tag_collection::tag_collection),
    Verb::new("ui-widget-collection", "Generate ui_widget_collection tags from data", tag_collection::ui_widget_collection),
    Verb::new("undefault", "Strip default values from tags", undefault::undefault),
    Verb::new("unicode-strings", "Generate unicode_string_list tags from data", unicode_strings::unicode_strings),
    Verb::new("verify-scenario", "Verify that a scenario tree does not contain errors", verify_scenario::verify_scenario),
    Verb::new("version", "View the version/license of Invader", version::version)
];

pub fn get_verb(what: &str) -> Option<&'static Verb> {
    ALL_VERBS.binary_search_by(|c| c.name.cmp(what)).map(|i| &ALL_VERBS[i]).ok()
}

fn print_scenario_tree_tag_result(logger: &LockedStdoutLogger, results: &HashMap<TagPath, ScenarioTreeTagResult>, scenario_path: &TagPath) {
    let total_issues = results
        .iter()
        .map(|a| a.1.errors.len() + a.1.warnings.len() + a.1.pedantic_warnings.len())
        .reduce(|a,b| a + b)
        .unwrap_or_default();

    match total_issues {
        0 => logger.success_fmt_ln(format_args!("Verified {scenario_path} and found no issues")),
        1 => logger.warning_fmt_ln(format_args!("Verified {scenario_path} and found one issue:")),
        other => logger.warning_fmt_ln(format_args!("Verified {scenario_path} and found {other} issues:"))
    }

    // First pass: pedantic warnings
    for (path, vr) in results {
        for i in &vr.pedantic_warnings {
            logger.minor_warning_fmt_ln(format_args!("WARNING (minor) {path}: {i}"))
        }
    }

    // Second pass: warnings
    for (path, vr) in results {
        for i in &vr.warnings {
            logger.warning_fmt_ln(format_args!("WARNING {path}: {i}"))
        }
    }

    // Final pass: errors
    for (path, vr) in results {
        for i in &vr.errors {
            logger.error_fmt_ln(format_args!("ERROR {path}: {i}"))
        }
    }
}
