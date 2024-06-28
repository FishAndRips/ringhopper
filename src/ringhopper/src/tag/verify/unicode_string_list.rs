use primitives::{primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::UnicodeStringList;
use crate::tag::tree::TagTree;
use super::{ScenarioContext, ScenarioTreeTagResult};

pub fn verify_unicode_string_list<T: TagTree + Send + Sync>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &ScenarioContext<T>, result: &mut ScenarioTreeTagResult) {
    let list: &UnicodeStringList = tag.as_any().downcast_ref().unwrap();

    for (i, string) in ziperator!(list.strings) {
        if string.string.get_string_lossy().is_err() {
            result.errors.push(format!("String #{i} is corrupted"));
        }
    }
}
