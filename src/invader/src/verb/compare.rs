use std::collections::{HashMap, VecDeque};
use std::env::Args;
use std::sync::{Arc, Mutex};
use cli::{CommandLineParser, CommandLineValue, CommandLineValueType, Parameter};
use ringhopper::error::Error;
use ringhopper::primitives::primitive::TagPath;
use ringhopper::tag::compare::{compare_tags, TagComparisonDifference};
use ringhopper::tag::tree::{AtomicTagTree, TagTree, VirtualTagsDirectory};
use threading::{DisplayMode, do_with_threads};

#[derive(PartialEq)]
enum Show {
    All,
    Mismatched,
    Matched
}

#[derive(Clone)]
struct UserData {
    tags: AtomicTagTree<Box<dyn TagTree + Send>>,
    differences: Arc<Mutex<HashMap<TagPath, Vec<TagComparisonDifference>>>>
}

pub fn compare(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
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
            'S',
            "Set whether to display `matched`, `mismatched`, or `all`. Default: `all`",
            "<param>",
            Some(CommandLineValueType::String),
            1,
            Some(vec![CommandLineValue::String("all".to_owned())]),
            false,
            false
        ))
        .add_custom_parameter(Parameter::new(
            "tags-source",
            'T',
            "Add a tags directory to compare against.",
            "<tags>",
            Some(CommandLineValueType::Path),
            1,
            None,
            true,
            false
        ))
        .add_custom_parameter(Parameter::new(
            "map-source",
            'M',
            "Add a map to compare against.",
            "<map>",
            Some(CommandLineValueType::Path),
            1,
            None,
            true,
            false
        ))
        .set_required_extra_parameters(1)
        .parse(args)?;

    let display_mode = match parser.get_custom("show").unwrap()[0].string() {
        "all" => Show::All,
        "mismatched" => Show::Mismatched,
        "matched" => Show::Matched,
        n => return Err(format!("Invalid --show parameter {n}"))
    };

    let verbose = parser.get_custom("verbose").is_some();

    let mut source: VecDeque<Box<dyn TagTree + Send>> = VecDeque::new();
    if let Some(n) = parser.get_custom("tags-source") {
        for i in n {
            let path = i.path();
            let dir = match VirtualTagsDirectory::new(&[path], None) {
                Ok(n) => n,
                Err(e) => return Err(format!("Error with tags directory {}: {e}", path.display()))
            };
            source.push_back(Box::new(dir));
        }
    }
    if let Some(n) = parser.get_custom("map-source") {
        for _ in n {
            return Err("Map comparison is not yet implemented!".to_string());
        }
    }

    if source.len() != 2 {
        return Err("Only two sources are supported.".to_string());
    }

    let primary = source.pop_front().unwrap();
    let secondary = AtomicTagTree::new(source.pop_front().unwrap());
    let tag = parser.get_extra()[0].clone();

    let user_data = UserData {
        tags: secondary,
        differences: Arc::new(Mutex::new(HashMap::new()))
    };

    do_with_threads(AtomicTagTree::new(primary), parser, &tag, None, user_data.clone(), DisplayMode::Silent, |context, path, user_data| {
        let secondary = user_data.tags.open_tag_copy(path);
        let secondary = match secondary {
            Ok(n) => n,
            Err(Error::CorruptedTag(_, _)) => secondary?,
            _ => return Ok(false)
        };

        let primary = context.tags_directory.open_tag_copy(path)?;
        let differences = compare_tags(primary.as_ref(), secondary.as_ref());
        user_data.differences.lock().unwrap().insert(path.to_owned(), differences);

        Ok(true)
    })?;

    display_result(display_mode, verbose, user_data);

    Ok(())
}

fn display_result(display_mode: Show, verbose: bool, user_data: UserData) {
    let mut matched = 0usize;
    let all_differences = Arc::into_inner(user_data.differences).unwrap().into_inner().unwrap();
    let mut keys: Vec<TagPath> = all_differences.keys().map(|c| c.to_owned()).collect();
    keys.sort();

    for k in &keys {
        let differences = &all_differences[k];
        if !differences.is_empty() {
            if display_mode == Show::All || display_mode == Show::Mismatched {
                println!("Mismatched: {k}");
                if verbose {
                    display_diff(differences);
                }
            }
        }
        else {
            if display_mode == Show::All || display_mode == Show::Matched {
                println!("Matched: {k}");
            }
            matched += 1;
        }
    }

    println!("Matched {matched} / {} tag{s}", keys.len(), s = if keys.len() == 1 { "" } else { "s" });
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

fn display_item(what: &DifferenceMap, depth: usize) {
    if depth > 0 {
        for _ in 0..depth {
            print!("    ");
        }
        print!("{}", what.field);
        if let Some(n) = &what.difference {
            print!(": {}", n.difference);
        }
        println!();
    }
    for i in &what.children {
        display_item(i, depth + 1);
    }
}

fn display_diff(diff: &Vec<TagComparisonDifference>) {
    let mut map = DifferenceMap::default();
    for i in diff {
        map.access_mut(&i.path).difference = Some(i.clone());
    }
    display_item(&map, 0);
}
