use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use definitions::{get_all_referenceable_tag_groups_for_group, Scenario, ScenarioType};
use primitives::dynamic::DynamicTagData;
use primitives::engine::Engine;
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::{HALO_PATH_SEPARATOR_STR, TagGroup, TagPath, TagReference};
use primitives::tag::{for_each_field, for_each_field_mut};
use crate::tag::result::TagResult;
use crate::tag::tree::{CachingTagTree, CachingTagTreeWriteStrategy, iterate_through_all_tags, TagFilter, TagTree, VirtualTagsDirectory};

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
pub fn recursively_get_dependencies_for_tag<T: TagTree>(tag: &TagPath, tag_tree: &T, allow_broken: bool) -> RinghopperResult<HashMap<TagPath, HashSet<TagPath>>> {
    let mut result: HashMap<TagPath, HashSet<TagPath>> = HashMap::new();
    let mut pending: Vec<(TagPath, Option<TagPath>)> = Vec::from([(tag.to_owned(), None)]);

    while let Some(p) = pending.pop() {
        // Do not open tags that cannot possibly contain dependencies
        if get_all_referenceable_tag_groups_for_group(p.0.group()).is_empty() {
            if !tag_tree.contains(&p.0) {
                if allow_broken {
                    continue;
                }
                if let Some(depender) = p.1 {
                    return Err(Error::BrokenDependency(depender, p.0))
                }
                else {
                    return Err(Error::TagNotFound(p.0))
                }
            }
            result.insert(p.0, HashSet::new());
            continue
        }

        // Try to open it now, converting the TagNotFound error to one that has the tag that depends on it
        let tag = match tag_tree.open_tag_shared(&p.0) {
            Ok(n) => Ok(n),
            Err(Error::TagNotFound(a)) if p.1.is_some() => Err(Error::BrokenDependency(p.1.unwrap(), a)),
            Err(e) => Err(e)
        };
        let tag = match tag {
            Ok(n) => n,
            Err(e) => {
                if allow_broken {
                    continue;
                }
                return Err(e)
            }
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

    Ok(result)
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

    all_dependencies.extend(recursively_get_dependencies_for_tag(scenario, tag_tree, false)?.into_values().flatten());

    for i in [engine.required_tags.all, other].into_iter().flatten() {
        let tag = TagPath::from_path(i).unwrap();
        all_dependencies.extend(recursively_get_dependencies_for_tag(&tag, tag_tree, false)?.into_values().flatten());
        all_dependencies.insert(tag);
    }

    Ok(all_dependencies)
}

pub enum ReplaceType {
    Start,
    All
}

pub fn refactor_paths_for_tag_tree(
    from: &str,
    to: &str,
    dir: &VirtualTagsDirectory,
    threads: NonZeroUsize,
    replace_type: ReplaceType,
    no_move: bool,
    filter: &TagFilter
) -> Result<(Vec<(TagPath, TagPath)>, HashMap<TagPath, TagResult>), (Error, HashMap<TagPath, TagResult>)> {
    let from = from
        .replace("/", HALO_PATH_SEPARATOR_STR)
        .replace(std::path::MAIN_SEPARATOR_STR, HALO_PATH_SEPARATOR_STR);
    let to = to
        .replace("/", HALO_PATH_SEPARATOR_STR)
        .replace(std::path::MAIN_SEPARATOR_STR, HALO_PATH_SEPARATOR_STR);

    if from == to {
        return Ok((Vec::new(), HashMap::new()))
    }

    let all_tags = dir.get_all_tags_with_filter(None);
    let mut tags_to_rename = Vec::<(TagPath, TagPath)>::new();

    for i in &all_tags {
        if !filter.passes(i) {
            continue;
        }
        let path_component = i.path();
        if path_component.contains(&from) {
            let replacement = match replace_type {
                ReplaceType::All => path_component.replace(&from, &to),
                ReplaceType::Start => {
                    if path_component.starts_with(&from) {
                        path_component.replacen(&from, &to, 1)
                    }
                    else {
                        continue
                    }
                }
            };
            let replacement = TagPath::new(&replacement, i.group()).map_err(|e| (e, HashMap::default()))?;
            if no_move && !all_tags.contains(&replacement) {
                return Err((Error::TagNotFound(replacement), HashMap::default()));
            }
            if !no_move {
                if all_tags.contains(&replacement) {
                    return Err((Error::Other(format!("Can't rename {i} to {replacement} because the latter already exists")), HashMap::default()));
                }
                for j in &tags_to_rename {
                    if j.1 == replacement {
                        return Err((Error::Other(format!("Can't rename {i} to {replacement} another tag will be renamed to this...")), HashMap::default()));
                    }
                }
            }
            tags_to_rename.push((i.to_owned(), replacement));
        }
    }

    if tags_to_rename.is_empty() {
        return Err((Error::Other(format!("No tags match `{from}`")), HashMap::default()));
    }

    let mut undo_history = Vec::new();
    let mut results = HashMap::new();

    if !no_move {
        for (old_path, new_path) in &tags_to_rename {
            let mut result = TagResult::default();

            let Some((index, actual_path)) = dir.path_for_tag(old_path) else {
                result.errors.push(format!("Couldn't get the fs path to {old_path}. Aborting..."));
                results.insert(old_path.to_owned(), result);
                break;
            };
            let new_actual_path = dir.get_directory(index).unwrap().join(new_path.to_native_path());
            let Some(parent) = new_actual_path.parent() else {
                result.errors.push(format!("Couldn't find the parent to {}. Aborting...", new_actual_path.display()));
                results.insert(old_path.to_owned(), result);
                break;
            };
            if let Err(e) = std::fs::create_dir_all(parent) {
                result.errors.push(format!("Couldn't create dirs {}: {e:?}; Aborting...", parent.display()));
                results.insert(old_path.to_owned(), result);
                break;
            };
            if let Err(e) = std::fs::rename(&actual_path, &new_actual_path) {
                result.errors.push(format!("Couldn't create dirs {}: {e:?}; Aborting...", parent.display()));
                results.insert(old_path.to_owned(), result);
                break;
            };

            // remove_dir only removes empty directories
            let mut p = actual_path.as_path();
            while let Some(parent) = p.parent() {
                if let Err(_) = std::fs::remove_dir(parent) {
                    break;
                }
                p = parent;
            }

            undo_history.push((actual_path, new_actual_path.to_path_buf()));
            results.insert(old_path.to_owned(), result);
        }

        if undo_history.len() != tags_to_rename.len() {
            for i in undo_history {
                std::fs::rename(&i.1, &i.0).unwrap();
            }
            return Err((Error::Other("Filesystem errors occurred".to_owned()), results));
        }
    }

    let mut all_tags_post_rename = all_tags.clone();
    if !no_move {
        for i in &mut all_tags_post_rename {
            for j in &tags_to_rename {
                if i == &j.0 {
                    *i = j.1.clone();
                }
            }
        }
    }

    struct Context {
        tags_to_rename: Vec<(TagPath, TagPath)>,
        results: Mutex<HashMap<TagPath, TagResult>>,
        cached_dir: CachingTagTree<VirtualTagsDirectory>,
        all_tags_post_rename: Vec<TagPath>,
        tags_to_commit: Mutex<Vec<TagPath>>,
        progress: AtomicUsize,
    }

    let context = Arc::new(Context {
        tags_to_rename,
        results: Mutex::new(results),
        cached_dir: CachingTagTree::new(dir.to_owned(), CachingTagTreeWriteStrategy::Manual),
        all_tags_post_rename,
        tags_to_commit: Mutex::default(),
        progress: AtomicUsize::new(0)
    });

    fn thread_some_shit(context: Arc<Context>) {
        loop {
            let tag = context.progress.fetch_add(1, Ordering::Relaxed);
            if tag >= context.all_tags_post_rename.len() {
                context.progress.fetch_sub(1, Ordering::Relaxed);
                break;
            }

            let tag = &context.all_tags_post_rename[tag];
            let referenceable = get_all_referenceable_tag_groups_for_group(tag.group());
            let mut allowed = false;
            for i in &context.tags_to_rename {
                if referenceable.contains(&i.1.group()) {
                    allowed = true;
                }
            }

            if !allowed {
                continue
            }

            let tag_to_mutate = match context.cached_dir.open_tag_shared(tag) {
                Ok(n) => n,
                Err(e) => {
                    let mut l = context.results.lock().unwrap();
                    l.get_mut(tag).unwrap().errors.push(format!("Can't open {tag}: {e}"));
                    continue;
                }
            };
            let mut tag_to_mutate = tag_to_mutate.lock().unwrap();

            let mut changes_made = false;
            for_each_dependency_mut(tag_to_mutate.as_mut_dynamic(), true, |_, r| {
                for (old_path, new_path) in &context.tags_to_rename {
                    if r.path() == Some(old_path) {
                        *r = TagReference::Set(new_path.to_owned());
                        changes_made = true;
                    }
                }
            });

            if changes_made {
                let mut tags_to_commit = context.tags_to_commit.lock().unwrap();
                tags_to_commit.push(tag.clone());
            }
        }
    }

    let mut t = Vec::new();
    for _ in 0..threads.get() {
        let c = context.clone();
        t.push(std::thread::spawn(move || thread_some_shit(c)));
    }

    for thread in t {
        thread.join().unwrap();
    }

    let mut context = Arc::into_inner(context).unwrap();
    let tags_to_commit = context.tags_to_commit.lock().unwrap();
    let tags_to_commit: &[TagPath] = tags_to_commit.as_slice();

    let mut results = context.results.into_inner().unwrap();

    for i in tags_to_commit {
        if let Err(e) = context.cached_dir.commit(i) {
            let r = match results.get_mut(i) {
                Some(n) => n,
                None => {
                    results.insert(i.to_owned(), TagResult::default());
                    results.get_mut(i).unwrap()
                }
            };
            r.errors.push(format!("Could not fix dependencies in {i}: {e} (you will need to fix the dependencies yourself)"));
        }
    }

    Ok((context.tags_to_rename, results))
}


#[cfg(test)]
mod test;
