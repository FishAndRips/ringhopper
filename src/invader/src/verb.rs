use std::env::Args;

mod version;
mod unicode_strings;

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
    Verb::new("unicode-strings", "Generate unicode_string_list tags from data", unicode_strings::unicode_strings),
    Verb::new("version", "View the version/license of Invader", version::version)
];

pub fn get_verb(what: &str) -> Option<&'static Verb> {
    ALL_VERBS.binary_search_by(|c| c.name.cmp(what)).map(|i| &ALL_VERBS[i]).ok()
}
