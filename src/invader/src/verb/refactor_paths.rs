use std::env::Args;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use cli::CommandLineParser;
use ringhopper::definitions::get_all_referenceable_tag_groups_for_group;
use ringhopper::tag::tree::{CachingTagTree, CachingTagTreeWriteStrategy, TagFilter, TagTree, VirtualTagsDirectory};
use ringhopper::primitives::primitive::{HALO_PATH_SEPARATOR_STR, TagPath, TagReference};
use ringhopper::tag::dependency::for_each_dependency_mut;
use util::{make_stdout_logger, StdoutLogger};

use crate::cli::*;

pub fn refactor_paths(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<find> <replace> [args]")
        .add_tags(true)
        .add_help()
        .add_jobs()
        .add_custom_parameter(Parameter::new(
            "no-move",
            'N',
            "Do not move tags (the destination tag(s) must exist)",
            "",
            None,
            0,
            None,
            false,
            false
        ))
        .add_custom_parameter(Parameter::new(
            "replace-type",
            'r',
            "Replace type (can be start, all; default = start)",
            "<type>",
            Some(CommandLineValueType::String),
            1,
            Some(vec![CommandLineValue::String("start".to_owned())]),
            false,
            false
        ))
        .add_custom_parameter(Parameter::new(
            "filter",
            'f',
            "Filter what tags that can be refactored. By default, all tags will be refactored if possible.",
            "<tag.group*>",
            Some(CommandLineValueType::String),
            1,
            Some(vec![CommandLineValue::String("*".to_owned())]),
            false,
            false
        ))
        .set_required_extra_parameters(2)
        .parse(args)?;

    let start = Instant::now();

    let from = parser.get_extra()[0]
        .replace("/", HALO_PATH_SEPARATOR_STR)
        .replace(std::path::MAIN_SEPARATOR_STR, HALO_PATH_SEPARATOR_STR);
    let to = parser.get_extra()[1]
        .replace("/", HALO_PATH_SEPARATOR_STR)
        .replace(std::path::MAIN_SEPARATOR_STR, HALO_PATH_SEPARATOR_STR);

    if from == to {
        return Err("from and to are the same".to_owned())
    }

    enum ReplaceType {
        Start,
        All
    }
    let replace_type = match parser.get_custom("replace-type").unwrap()[0].string() {
        "all" => ReplaceType::All,
        "start" => ReplaceType::Start,
        n => return Err(format!("Unknown replace_type {n}; expected `all`, `first`, or `last`"))
    };

    let no_move = parser.get_custom("no-move").is_some();
    let dir = parser.get_virtual_tags_directory();

    let all_tags = dir.get_all_tags_with_filter(None);
    let mut tags_to_rename = Vec::<(TagPath, TagPath)>::new();

    let filter = TagFilter::new(parser.get_custom("filter").unwrap()[0].string(), None);

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
            let Ok(replacement) = TagPath::new(&replacement, i.group()) else {
                return Err(format!("Can't rename {i} to {replacement}.{} - invalid path", i.group()))
            };
            if no_move && !all_tags.contains(&replacement) {
                return Err(format!("Can't rename {i} to {replacement} because the replacement doesn't exist (--no-move used)"))
            }
            if !no_move && all_tags.contains(&replacement) {
                return Err(format!("Can't rename {i} to {replacement} because the latter already exists (use --no-move to prevent this)"))
            }
            if !no_move {
                for j in &tags_to_rename {
                    if j.1 == replacement {
                        return Err(format!("Can't rename {i} to {replacement} another tag will be renamed to this..."))
                    }
                }
            }
            tags_to_rename.push((i.to_owned(), replacement));
        }
    }

    if tags_to_rename.is_empty() {
        return Err(format!("No tags match `{from}`"))
    }

    let logger = make_stdout_logger();
    let mut undo_history = Vec::new();

    if !no_move {
        for (old_path, new_path) in &tags_to_rename {
            let Some((index, actual_path)) = dir.path_for_tag(old_path) else {
                logger.error_fmt(format_args!("Couldn't get the fs path to {old_path}. Aborting..."));
                break;
            };
            let new_actual_path = dir.get_directory(index).unwrap().join(new_path.to_native_path());
            let Some(parent) = new_actual_path.parent() else {
                logger.error_fmt(format_args!("Couldn't find the parent to {}. Aborting...", new_actual_path.display()));
                break;
            };
            if let Err(e) = std::fs::create_dir_all(parent) {
                logger.error_fmt(format_args!("Couldn't create dirs {}: {e:?}; Aborting...", parent.display()));
                break;
            };
            if let Err(e) = std::fs::rename(&actual_path, &new_actual_path) {
                logger.error_fmt(format_args!("Couldn't create dirs {}: {e:?}; Aborting...", parent.display()));
                break;
            };
            logger.success_fmt_ln(format_args!("Renamed {} to {}", actual_path.display(), new_actual_path.display()));
            undo_history.push((actual_path, new_actual_path.to_path_buf()));
        }

        if undo_history.len() != tags_to_rename.len() {
            for i in undo_history {
                std::fs::rename(&i.1, &i.0).unwrap();
            }
            return Err("An error occurred while moving tags. No tags were modified.".to_owned());
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

    logger.neutral_ln("Updating references (this may take a while)...");
    logger.flush();

    struct Context {
        logger: Arc<StdoutLogger>,
        tags_to_rename: Vec<(TagPath, TagPath)>,
        cached_dir: CachingTagTree<VirtualTagsDirectory>,
        all_tags_post_rename: Vec<TagPath>,
        tags_to_commit: Mutex<Vec<TagPath>>,
        progress: AtomicUsize,
    }

    let context = Arc::new(Context {
        logger: logger.clone(),
        tags_to_rename,
        cached_dir: CachingTagTree::new(dir, CachingTagTreeWriteStrategy::Manual),
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
                    context.logger.error_fmt_ln(format_args!("Can't open {tag}: {e}"));
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
    for _ in 0..parser.get_jobs() {
        let c = context.clone();
        t.push(std::thread::spawn(move || thread_some_shit(c)));
    }

    for thread in t {
        thread.join().unwrap();
    }

    let mut context = Arc::into_inner(context).unwrap();
    let tags_to_commit = context.tags_to_commit.lock().unwrap();
    let tags_to_commit: &[TagPath] = tags_to_commit.as_slice();

    for i in tags_to_commit {
        if let Err(e) = context.cached_dir.commit(i) {
            logger.error_fmt_ln(format_args!("Could not fix dependencies in {i}: {e} (you will need to fix the dependencies yourself)"));
        }
    }

    let time_elapsed = (Instant::now() - start).as_millis();
    logger.success_fmt_ln(format_args!("Renamed {} tags and changed references in {} tag(s) in {time_elapsed} ms", context.tags_to_rename.len(), tags_to_commit.len()));

    Ok(())
}
