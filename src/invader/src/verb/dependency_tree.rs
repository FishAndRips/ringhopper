use std::collections::{HashMap, HashSet};
use std::env::Args;
use cli::CommandLineParser;
use ringhopper::primitives::primitive::TagPath;
use ringhopper::tag::dependency::*;
use ringhopper::tag::tree::{CachingTagTree, CachingTagTreeWriteStrategy, TagFilter, TagTree};
use util::{make_stdout_logger, StdoutLogger};

pub fn dependency_tree(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag*> [args]")
        .add_tags(true)
        .add_help()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tags = CachingTagTree::new(parser.get_virtual_tags_directory(), CachingTagTreeWriteStrategy::Manual);
    let input_tag_path = &parser.get_extra()[0];
    let all_tags = tags.get_all_tags_with_filter(Some(&TagFilter::new(input_tag_path, None)));
    let mut result: HashMap<TagPath, HashSet<TagPath>> = HashMap::new();
    for i in &all_tags {
        let new_result = str_unwrap!(recursively_get_dependencies_for_tag(&i, &tags, true), "Failed to recursively get dependencies for {i}: {error}");
        for i in new_result {
            result.insert(i.0, i.1);
        }
    }

    let result_sorted = result.into_iter().map(|(path, set)| {
        let mut sorted: Vec<TagPath> = set.into_iter().collect();
        sorted.sort();
        (path, sorted)
    });

    let mut result = HashMap::new();
    for (path, tags) in result_sorted {
        result.insert(path, tags);
    }

    let mut already_printed: HashSet<TagPath> = HashSet::new();

    fn print_tags(
        tag_path: &TagPath,
        result: &HashMap<TagPath, Vec<TagPath>>,
        already_printed: &mut HashSet<TagPath>,
        depth: usize,
        next_same_level: bool,
        output: &StdoutLogger,
        next_on_depths: &mut Vec<bool>
    ) {
        if depth > 0 {
            let bars = depth.saturating_sub(1);
            for n in 0..bars {
                if next_on_depths[n] {
                    output.neutral(" │ ");
                }
                else {
                    output.neutral("   ");
                }
            }
            if next_same_level {
                output.neutral(" ├");
            }
            else {
                output.neutral(" └");
            }
            output.neutral("─");
        }

        let Some(d) = result.get(tag_path) else {
            output.warning_fmt_ln(format_args!("{tag_path} (BROKEN)"));
            return;
        };

        let is_already_printed = already_printed.contains(tag_path);

        if is_already_printed && !d.is_empty() {
            output.neutral_fmt_ln(format_args!("{tag_path} (minimized)"));
        }
        else {
            output.neutral_fmt_ln(format_args!("{tag_path}"));
        }

        if is_already_printed {
            return;
        }

        already_printed.insert(tag_path.to_owned());

        let mut dependencies = d.iter().peekable();
        while let Some(d) = dependencies.next() {
            let inner_depth = depth + 1;
            let next_same_level = dependencies.peek().is_some();
            next_on_depths.push(next_same_level);
            print_tags(d, result, already_printed, inner_depth, next_same_level, output, next_on_depths);
            next_on_depths.pop();
        }
    }

    let mut next_on_depths = Vec::new();

    let output = make_stdout_logger();

    let root_tags = all_tags.iter().filter(|&t| {
        for r in &result {
            if r.1.contains(t) {
                return false
            }
        }
        true
    });

    for i in root_tags {
        print_tags(i, &result, &mut already_printed, 0, false, output.as_ref(), &mut next_on_depths);
    }
    Ok(())
}
