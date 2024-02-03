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
    Verb::new("compare", "Compare tags between two tag sources", compare::compare),
    Verb::new("convert", "Convert tags to another tag group", convert::convert),
    Verb::new("dependencies", "View dependencies of tags", dependencies::dependencies),
    Verb::new("nudge", "Fix floating point precision errors from tag extraction", nudge::nudge),
    Verb::new("strip", "Clean tags", strip::strip),
    Verb::new("tag-collection", "Generate tag_collection tags from data", tag_collection::tag_collection),
    Verb::new("ui-widget-collection", "Generate ui_widget_collection tags from data", tag_collection::ui_widget_collection),
    Verb::new("unicode-strings", "Generate unicode_string_list tags from data", unicode_strings::unicode_strings),
    Verb::new("version", "View the version/license of Invader", version::version)
];

pub fn get_verb(what: &str) -> Option<&'static Verb> {
    ALL_VERBS.binary_search_by(|c| c.name.cmp(what)).map(|i| &ALL_VERBS[i]).ok()
}
