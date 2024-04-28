use std::collections::{HashMap, HashSet};
use definitions::{get_all_referenceable_tag_groups_for_group, Scenario, ScenarioType};
use primitives::dynamic::DynamicTagData;
use primitives::engine::Engine;
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::{TagGroup, TagPath, TagReference};
use crate::tag::tree::{iterate_through_all_tags, TagTree};

/// Iterate through each [`TagReference`] of a block.
pub fn for_each_dependency<T: DynamicTagData + ?Sized, P: FnMut(&TagReference)>(data: &T, mut predicate: P) {
    fn recursion<T: DynamicTagData + ?Sized, P: FnMut(&TagReference)>(data: &T, predicate: &mut P) {
        for field in data.fields() {
            let field = data.get_field(field).unwrap();
            if let Some(p) = field.as_any().downcast_ref::<TagReference>() {
                predicate(p);
                continue;
            }
            if let Some(arr) = field.as_array() {
                for i in 0..arr.len() {
                    let item = arr.get_at_index(i).unwrap();
                    recursion(item, predicate);
                }
            }
            recursion(field, predicate);
        }
    }
    recursion(data, &mut predicate);
}

/// Mutably iterate through each [`TagReference`] of a block.
fn for_each_dependency_mut<P: FnMut(&'static [TagGroup], &mut TagReference)>(data: &mut dyn DynamicTagData, access_read_only_fields: bool, mut predicate: P) {
    fn recursion<P: FnMut(&'static [TagGroup], &mut TagReference)>(data: &mut dyn DynamicTagData, predicate: &mut P, access_read_only_fields: bool) {
        for field_name in data.fields() {
            let metadata = match data.get_metadata_for_field(field_name) {
                Some(n) => n,
                None => continue
            };

            if !access_read_only_fields && metadata.read_only {
                continue;
            }

            let field = data.get_field_mut(field_name).unwrap();
            if let Some(p) = field.as_any_mut().downcast_mut::<TagReference>() {
                predicate(metadata.allowed_references.unwrap(), p);
                continue;
            }
            if let Some(arr) = field.as_array_mut() {
                for i in 0..arr.len() {
                    let item = arr.get_at_index_mut(i).unwrap();
                    recursion(item, predicate, access_read_only_fields);
                }
            }
            recursion(field, predicate, access_read_only_fields);
        }
    }
    recursion(data, &mut predicate, access_read_only_fields);
}

/// Get all dependencies for a block of tag data or a tag itself.
pub fn get_tag_dependencies_for_block<T: DynamicTagData + ?Sized>(data: &T) -> HashSet<TagPath> {
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
