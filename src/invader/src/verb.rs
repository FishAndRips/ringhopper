use std::env::Args;

macro_rules! str_unwrap {
    ($what:expr, $($fmt:tt)+) => {
        ($what).map_err(|e| format!($($fmt)+, error=e.to_string()))?
    };
}

mod dependencies;
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
mod verify;
mod refactor_groups;
mod bludgeon;

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
    Verb::new("archive-scenario", "Create a .zip of a map's tag structure", archive::archive_scenario),
    Verb::new("archive-tag", "Create a .zip of a tag and its dependencies", archive::archive_tag),
    Verb::new("bludgeon", "Automatically repair common issues with tags", bludgeon::bludgeon),
    Verb::new("compare", "Compare tags between two tag sources", compare::compare),
    Verb::new("convert", "Convert tags to another tag group", convert::convert),
    Verb::new("dependencies", "View dependencies of tags", dependencies::dependencies),
    Verb::new("extract", "Extract tags from a map", extract::extract),
    Verb::new("list-engines", "List all available engine targets", list_engines::list_engines),
    Verb::new("list-scenario-tags", "View all tags needed to build a scenario into a map", dependencies::list_scenario_tags),
    Verb::new("nudge", "Fix floating point precision errors from tag extraction", nudge::nudge),
    Verb::new("plate", "Generate color plates for bitmaps", plate::plate),
    Verb::new("recover", "Recover data from tags", recover::recover),
    Verb::new("refactor-groups", "Batch refactor dependencies by tag group if the new dependency exists", refactor_groups::refactor_groups),
    Verb::new("strip", "Clean tags", strip::strip),
    Verb::new("tag-collection", "Generate tag_collection tags from data", tag_collection::tag_collection),
    Verb::new("ui-widget-collection", "Generate ui_widget_collection tags from data", tag_collection::ui_widget_collection),
    Verb::new("undefault", "Strip default values from tags", undefault::undefault),
    Verb::new("unicode-strings", "Generate unicode_string_list tags from data", unicode_strings::unicode_strings),
    Verb::new("verify", "Verify that a tag does not contain errors", verify::verify),
    Verb::new("version", "View the version/license of Invader", version::version)
];

pub fn get_verb(what: &str) -> Option<&'static Verb> {
    ALL_VERBS.binary_search_by(|c| c.name.cmp(what)).map(|i| &ALL_VERBS[i]).ok()
}
