use std::collections::{HashMap, HashSet};
use definitions::get_all_referenceable_tag_groups_for_group;
use primitives::dynamic::DynamicTagData;
use primitives::error::RinghopperResult;
use primitives::primitive::{TagPath, TagReference};
use tag::tree::{iterate_through_all_tags, TagTree};

/// Get all dependencies for a block of tag data or a tag itself.
pub fn get_tag_dependencies_for_block<T: DynamicTagData + ?Sized>(data: &T) -> HashSet<TagPath> {
    let mut result = HashSet::new();

    fn iterate_dependencies_recursively<T: DynamicTagData + ?Sized>(data: &T, result: &mut HashSet<TagPath>) {
        for field in data.fields() {
            let field = data.get_field(field).unwrap();
            if let Some(TagReference::Set(p)) = field.as_any().downcast_ref::<TagReference>() {
                result.insert(p.clone());
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
pub fn recursively_get_dependencies_for_tag<T: TagTree>(tag: &TagPath, tag_tree: &T) -> RinghopperResult<HashSet<TagPath>> {
    let mut result: HashMap<TagPath, HashSet<TagPath>> = HashMap::new();
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

/// Get all tags that depend on a tag.
pub fn get_reverse_dependencies_for_tag<T: TagTree>(tag: &TagPath, tag_tree: &T) -> RinghopperResult<HashSet<TagPath>> {
    let mut result = HashSet::new();

    for i in iterate_through_all_tags(tag_tree, None) {
        if !get_all_referenceable_tag_groups_for_group(i.group()).contains(&tag.group()) {
            continue
        }

        let t = tag_tree.open_tag_shared(&i).unwrap();
        let t = t.lock().unwrap();
        if get_tag_dependencies_for_block(t.as_ref()).contains(tag) {
            result.insert(i);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod test;
