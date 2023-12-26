//! General-use definitions parsing library.
//!
//! This can be used for writing parsers for tags.

extern crate serde_json;

mod types;
pub use types::*;

/// Load all built-in definitions.
pub fn load_all_definitions() -> ParsedDefinitions {
    let values = get_all_definitions();
    let mut parsed = ParsedDefinitions::default();
    parsed.load_from_json(&values);
    parsed.assert_valid();
    parsed.resolve_parent_class_references();

    parsed
}
