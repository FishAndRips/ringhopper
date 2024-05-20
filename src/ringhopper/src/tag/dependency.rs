use std::collections::{HashMap, HashSet};
use definitions::{get_all_referenceable_tag_groups_for_group, Scenario, ScenarioType};
use primitives::dynamic::DynamicTagData;
use primitives::engine::Engine;
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::{TagGroup, TagPath, TagReference};
use primitives::tag::{for_each_field, for_each_field_mut};
use crate::tag::tree::{iterate_through_all_tags, TagTree};

/// Iterate through each [`TagReference`] of a block.
pub fn for_each_dependency<P: FnMut(&TagReference)>(data: &dyn DynamicTagData, mut predicate: P) {
    for_each_field(data, |_, b| {
        let reference_maybe: Option<&TagReference> = b.as_any().downcast_ref();
        if let Some(n) = reference_maybe {
            predicate(n)
        }
    });
}

/// Mutably iterate through each [`TagReference`] of a block.
pub fn for_each_dependency_mut<P: FnMut(&'static [TagGroup], &mut TagReference)>(data: &mut dyn DynamicTagData, access_read_only_fields: bool, mut predicate: P) {
    for_each_field_mut(data, access_read_only_fields, |a, b| {
        let reference_maybe: Option<&mut TagReference> = b.as_any_mut().downcast_mut();
        if let Some(n) = reference_maybe {
            predicate(a.unwrap().allowed_references.unwrap(), n)
        }
    });
}

/// Get all dependencies for a block of tag data or a tag itself.
pub fn get_tag_dependencies_for_block(data: &dyn DynamicTagData) -> HashSet<TagPath> {
    let mut result = HashSet::new();
    for_each_dependency(data, |dependency| {
        if let TagReference::Set(p) = dependency {
            result.insert(p.clone());
        }
    });
    result
}

pub fn refactor_groups_for_block<T: TagTree>(data: &mut dyn DynamicTagData, from: TagGroup, to: TagGroup, tag_tree: &T, access_read_only_fields: bool) -> bool {
    let mut anything_done = false;

    for_each_dependency_mut(data, access_read_only_fields, |allowed, dependency| {
        let allowed = allowed.contains(&to);

        if !allowed || dependency.group() != from {
            return;
        }

        if let TagReference::Set(p) = dependency {
            let mut new_path = p.clone();
            new_path.set_group(to);
            if !tag_tree.contains(&new_path) {
                return;
            }
        }

        dependency.set_group(to);
        anything_done = true;
    });

    anything_done
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

        let dependencies = get_tag_dependencies_for_block(tag.lock().unwrap().as_ref().as_dynamic());
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
        if get_tag_dependencies_for_block(t.as_ref().as_dynamic()).contains(tag) {
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
