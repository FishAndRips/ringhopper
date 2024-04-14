use std::collections::{HashMap, VecDeque};
use std::env::Args;
use std::io::Cursor;
use std::path::Path;
use std::sync::{Arc, Mutex};
use cli::{CommandLineParser, CommandLineValue, CommandLineValueType, Parameter};
use ringhopper::error::Error;
use ringhopper::map::load_map_from_filesystem_as_tag_tree;
use ringhopper::primitives::byteorder::WriteBytesExt;
use ringhopper::primitives::primitive::TagPath;
use ringhopper::primitives::tag::ParseStrictness;
use ringhopper::tag::compare::{compare_tags, TagComparisonDifference};
use ringhopper::tag::tree::{TagTree, VirtualTagsDirectory};
use threading::{DisplayMode, do_with_threads, ProcessSuccessType};
use util::make_stdout_logger;

use crate::util::StdoutLogger;

#[derive(PartialEq)]
enum Show {
    All,
    Mismatched,
    Matched
}

#[derive(Clone)]
struct UserData {
    tags: Arc<dyn TagTree + Send + Sync>,
    differences: Arc<Mutex<HashMap<TagPath, Vec<TagComparisonDifference>>>>,
    should_strip: [bool; 2],
    raw: bool
}

pub fn compare(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<source1> <source2> [args]")
        .add_help()
        .add_custom_parameter(Parameter::new(
            "verbose",
            'v',
            "Display detailed output.",
            "",
            None,
            0,
            None,
            false,
            false
        ))
        .add_custom_parameter(Parameter::new(
            "show",
            's',
            "Set whether to display `matched`, `mismatched`, or `all`. Default: `all`",
            "<param>",
            Some(CommandLineValueType::String),
            1,
            Some(vec![CommandLineValue::String("all".to_owned())]),
            false,
            false
        ))
        .add_custom_parameter(Parameter::new(
            "filter",
            'f',
            "Filter tags to be compared. By default, all tags are compared.",
            "<tag*>",
            Some(CommandLineValueType::String),
            1,
            Some(vec![CommandLineValue::String("*".to_owned())]),
            false,
            false
        ))
        .add_jobs()
        .add_custom_parameter(Parameter::single("raw", 'r', "Do not strip cache-only fields from cache file tags when comparing.", "", None))
        .set_required_extra_parameters(2)
        .parse(args)?;

    let display_mode = match parser.get_custom("show").unwrap()[0].string() {
        "all" => Show::All,
        "mismatched" => Show::Mismatched,
        "matched" => Show::Matched,
        n => return Err(format!("Invalid --show parameter {n}"))
    };

    let verbose = parser.get_custom("verbose").is_some();
    let raw = parser.get_custom("raw").is_some();

    let mut source: VecDeque<Arc<dyn TagTree + Send + Sync>> = VecDeque::new();
    let mut should_strip = Cursor::new([0u8; 2]);
    for i in parser.get_extra() {
        let path: &Path = i.as_ref();
        if !path.exists() {
            return Err(format!("Source `{i}` does not exist"))
        }
        else if path.is_file() && path.extension() == Some("map".as_ref()) {
            should_strip.write_u8(if raw { 0 } else { 1 }).unwrap();
            let map = load_map_from_filesystem_as_tag_tree(path, ParseStrictness::Strict)
                .map_err(|e| format!("Cannot load {path:?} as a cache file: {e:?}"))?;
            source.push_back(map);
        }
        else if path.is_dir() {
            should_strip.write_u8(0).unwrap();
            let dir = match VirtualTagsDirectory::new(&[path], None) {
                Ok(n) => n,
                Err(e) => return Err(format!("Error with tags directory {}: {e}", path.display()))
            };
            source.push_back(Arc::new(dir));
        }
        else {
            return Err(format!("Source `{i}` does not refer to a directory"))
        }
    }

    let should_strip = should_strip.into_inner();
    let should_strip = [should_strip[0] != 0, should_strip[1] != 0];

    let primary = source.pop_front().unwrap();
    let secondary = source.pop_front().unwrap();
    let tag = parser.get_custom("filter").unwrap()[0].string().to_owned();

    let user_data = UserData {
        tags: secondary,
        differences: Arc::new(Mutex::new(HashMap::new())),
        should_strip,
        raw
    };

    let logger = make_stdout_logger();

    do_with_threads(primary, parser, &tag, None, user_data.clone(), DisplayMode::Silent, logger.clone(), |context, path, user_data, _| {
        let mut secondary = match user_data.tags.open_tag_copy(path) {
            Ok(n) => n,
            Err(Error::TagNotFound(_)) => return Ok(ProcessSuccessType::Skipped("not in directory")),
            n => n?
        };

        let mut primary = context.tags_directory.open_tag_copy(path)?;

        // Strip any cache-only fields
        if user_data.should_strip[0] {
            primary = ringhopper::definitions::read_any_tag_from_file_buffer(&primary.to_tag_file()?, ParseStrictness::Relaxed)?;
        }
        if user_data.should_strip[1] {
            secondary = ringhopper::definitions::read_any_tag_from_file_buffer(&secondary.to_tag_file()?, ParseStrictness::Relaxed)?;
        }

        // Unset defaults if raw was not requested
        if !user_data.raw {
            primary.unset_defaults();
            secondary.unset_defaults();
        }

        let differences = compare_tags(primary.as_ref(), secondary.as_ref());
        user_data.differences.lock().unwrap().insert(path.to_owned(), differences);

        Ok(ProcessSuccessType::Success)
    })?;

    display_result(display_mode, verbose, user_data, &logger);

    Ok(())
}

fn display_result(display_mode: Show, verbose: bool, user_data: UserData, logger: &Arc<StdoutLogger>) {
    let mut matched = 0usize;
    let all_differences = Arc::into_inner(user_data.differences).unwrap().into_inner().unwrap();
    let mut keys: Vec<TagPath> = all_differences.keys().map(|c| c.to_owned()).collect();
    keys.sort();

    for k in &keys {
        let differences = &all_differences[k];
        if !differences.is_empty() {
            if display_mode == Show::All || display_mode == Show::Mismatched {
                logger.warning_fmt_ln(format_args!("Mismatched: {k}"));
                if verbose {
                    display_diff(differences, &logger);
                }
            }
        }
        else {
            if display_mode == Show::All || display_mode == Show::Matched {
                logger.success_fmt_ln(format_args!("Matched: {k}"));
            }
            matched += 1;
        }
    }

    logger.neutral_fmt_ln(format_args!("Matched {matched} / {} tag{s}", all_differences.len(), s = if all_differences.len() == 1 { "" } else { "s" }));
}

#[derive(Default)]
struct DifferenceMap {
    field: String,
    difference: Option<TagComparisonDifference>,
    children: Vec<DifferenceMap>
}

impl DifferenceMap {
    fn access_mut(&mut self, path: &str) -> &mut DifferenceMap {
        match path.find('.') {
            Some(n) => {
                let (field, remaining) = path.split_at(n);
                self.access_mut(field)
                    .access_mut(&remaining[1..])
            },
            None => {
                self.create_if_not_exists(path)
            }
        }
    }

    fn create_if_not_exists(&mut self, field: &str) -> &mut DifferenceMap {
        let index = self.get(field);
        &mut self.children[index]
    }

    fn get(&mut self, field: &str) -> usize {
        let len = self.children.len();
        for i in 0..self.children.len() {
            if self.children[i].field == field {
                return i
            }
        }
        self.children.push(DifferenceMap {
            field: field.to_owned(),
            difference: None,
            children: Vec::new()
        });
        len
    }
}

fn display_item(what: &DifferenceMap, depth: usize, io: &Arc<StdoutLogger>) {
    if depth > 0 {
        for _ in 0..depth {
            io.neutral("    ");
        }
        io.warning(&what.field);
        if let Some(n) = &what.difference {
            io.warning_fmt(format_args!(": {}", n.difference));
        }
        io.neutral_ln("");
    }
    for i in &what.children {
        display_item(i, depth + 1, io);
    }
}

fn display_diff(diff: &Vec<TagComparisonDifference>, io: &Arc<StdoutLogger>) {
    let mut map = DifferenceMap::default();
    for i in diff {
        map.access_mut(&i.path).difference = Some(i.clone());
    }
    display_item(&map, 0, io);
}
