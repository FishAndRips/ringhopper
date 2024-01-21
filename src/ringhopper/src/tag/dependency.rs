use std::collections::{HashMap};
use primitives::dynamic::DynamicTagData;
use primitives::error::RinghopperResult;
use primitives::primitive::{TagPath, TagReference};
use tag::tree::TagTree;

/// Get all dependencies for a block of tag data or a tag itself.
pub fn get_tag_dependencies_for_block<T: DynamicTagData + ?Sized>(data: &T) -> Vec<TagPath> {
    let mut result = Vec::new();

    fn iterate_dependencies_recursively<T: DynamicTagData + ?Sized>(data: &T, result: &mut Vec<TagPath>) {
        for field in data.fields() {
            let field = data.get_field(field).unwrap();
            if let Some(TagReference::Set(p)) = field.as_any().downcast_ref::<TagReference>() {
                result.push(p.clone());
                continue;
            }
            if let Some(arr) = field.as_array() {
                for i in 0..arr.len() {
                    let item = arr.get_at_index(i).unwrap();
                    iterate_dependencies_recursively(item, result);
                }
            }
            iterate_dependencies_recursively(field, result);
        }
    }

    iterate_dependencies_recursively(data, &mut result);
    result
}

/// Get all dependencies for a tag.
///
/// Returns `Err` if a depended tag could not be opened from a tag tree.
pub fn recursively_get_dependencies_for_tag<T: TagTree>(tag: &TagPath, tag_tree: &T) -> RinghopperResult<Vec<TagPath>> {
    let mut result: HashMap<TagPath, Vec<TagPath>> = HashMap::new();
    let mut pending: Vec<TagPath> = Vec::from([tag.to_owned()]);

    while let Some(p) = pending.pop() {
        let dependencies = get_tag_dependencies_for_block(tag_tree.open_tag_shared(&p)?.lock().unwrap().as_ref());
        for i in &dependencies {
            if result.contains_key(i) || pending.contains(i) {
                continue
            }
            pending.push(i.to_owned());
        }
        result.insert(p, dependencies);
    }

    Ok(result.into_values().flatten().collect())
}

#[cfg(test)]
mod test;
