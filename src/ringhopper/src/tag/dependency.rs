use std::collections::{HashMap, HashSet};
use definitions::{get_all_referenceable_tag_groups_for_group, Scenario, ScenarioType};
use primitives::dynamic::DynamicTagData;
use primitives::engine::Engine;
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::{TagPath, TagReference};
use crate::tag::tree::{iterate_through_all_tags, TagTree};

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
    let mut pending: Vec<(TagPath, Option<TagPath>)> = Vec::from([(tag.to_owned(), None)]);

    while let Some(p) = pending.pop() {
        // Do not open tags that cannot possibly contain dependencies
        if get_all_referenceable_tag_groups_for_group(p.0.group()).is_empty() {
            if !tag_tree.contains(&p.0) {
                if let Some(depender) = p.1 {
                    return Err(Error::BrokenDependency(depender, p.0))
                }
                else {
                    return Err(Error::TagNotFound(p.0))
                }
            }
            continue
        }

        // Try to open it now, converting the TagNotFound error to one that has the tag that depends on it
        let tag = match tag_tree.open_tag_shared(&p.0) {
            Ok(n) => n,
            Err(Error::TagNotFound(a)) if p.1.is_some() => return Err(Error::BrokenDependency(p.1.unwrap(), a)),
            Err(e) => return Err(e)
        };

        let dependencies = get_tag_dependencies_for_block(tag.lock().unwrap().as_ref());
        for i in &dependencies {
            if result.contains_key(i) || pending.iter().find(|p| &p.0 == i).is_some() {
                continue
            }
            pending.push((i.to_owned(), Some(p.0.clone())));
        }
        result.insert(p.0, dependencies);
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

pub fn recursively_get_dependencies_for_map<T: TagTree>(scenario: &TagPath, tag_tree: &T, engine: &Engine) -> RinghopperResult<HashSet<TagPath>> {
    let mut all_dependencies = HashSet::new();

    all_dependencies.insert(scenario.to_owned());

    let mutex = tag_tree.open_tag_shared(scenario)?;
    let lock = mutex.lock().unwrap();
    let scenario_tag: &Scenario = lock.as_any().downcast_ref().unwrap();

    let other: &'static [&'static str] = match scenario_tag._type {
        ScenarioType::Singleplayer => engine.required_tags.singleplayer,
        ScenarioType::Multiplayer => engine.required_tags.multiplayer,
        ScenarioType::UserInterface => engine.required_tags.user_interface,
    };

    // prevent deadlocking for caching tag trees
    drop(lock);

    all_dependencies.extend(recursively_get_dependencies_for_tag(scenario, tag_tree)?);

    for i in [engine.required_tags.all, other].into_iter().flatten() {
        let tag = TagPath::from_path(i).unwrap();
        all_dependencies.extend(recursively_get_dependencies_for_tag(&tag, tag_tree)?);
        all_dependencies.insert(tag);
    }

    Ok(all_dependencies)
}

#[cfg(test)]
mod test;
