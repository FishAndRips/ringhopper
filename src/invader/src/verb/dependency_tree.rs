use std::collections::{HashMap, HashSet};
use std::env::Args;
use cli::{CommandLineParser, CommandLineValue, CommandLineValueType, Parameter};
use ringhopper::primitives::primitive::TagPath;
use ringhopper::tag::dependency::*;
use ringhopper::tag::tree::{CachingTagTree, CachingTagTreeWriteStrategy, TagFilter, TagTree};
use util::{make_stdout_logger, StdoutLogger};

pub fn dependency_tree(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag*> [args]")
        .add_tags(true)
        .add_help()
        .add_custom_parameter(Parameter::new("broken", 'b', "Only show broken dependencies (and their dependents)", "", None, 0, None, false, false))
        .add_custom_parameter(Parameter::new("depth", 'd', "Maximum depth (0 = only show root tags; by default, max depth is 2^32-1)", "", Some(CommandLineValueType::UInteger), 1, None, false, false))
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

    let max_depth = parser.get_custom("depth").unwrap_or(&[CommandLineValue::UInteger(u32::MAX)])[0].uinteger();

    let result_sorted = result.into_iter().map(|(path, set)| {
        let mut sorted: Vec<TagPath> = set.into_iter().collect();
        sorted.sort();
        (path, sorted)
    });

    let mut result = HashMap::new();
    for (path, tags) in result_sorted {
        result.insert(path, tags);
    }

    fn contains_broken_dependency_somewhere<'a>(
        tag_path: &'a TagPath,
        result: &'a HashMap<TagPath, Vec<TagPath>>,
        stack: &mut Vec<&'a TagPath>
    ) -> bool {
        let Some(it) = result.get(tag_path) else {
            return true;
        };
        if stack.contains(&tag_path) {
            return false;
        }
        stack.push(tag_path);
        let mut broken = false;
        for i in it {
            if contains_broken_dependency_somewhere(i, result, stack) {
                broken = true;
                break;
            }
        }
        let _popped = stack.pop();
        debug_assert_eq!(_popped, Some(tag_path));
        broken
    }

    // Find all tags we can show here.
    let only_show_broken = parser.get_custom("broken").is_some();
    let tags_with_broken_dependencies = if only_show_broken {
        let mut allowed_tags = Vec::new();
        let mut stack = Vec::new();
        for i in result.keys() {
            if contains_broken_dependency_somewhere(i, &result, &mut stack) {
                allowed_tags.push(i);
            }
        }
        Some(allowed_tags)
    }
    else {
        None
    };
    let tags_with_broken_dependencies = tags_with_broken_dependencies.as_ref().map(|t| t.as_slice());

    fn print_tags<'a>(
        tag_path: &'a TagPath,
        result: &'a HashMap<TagPath, Vec<TagPath>>,
        depth: u32,
        max_depth: u32,
        next_same_level: bool,
        tags_with_broken_dependencies: &Option<&[&TagPath]>,
        output: &StdoutLogger,
        next_on_depths: &mut Vec<bool>,
        stack: &mut Vec<&'a TagPath>
    ) {
        if depth > 0 {
            let bars = depth.saturating_sub(1);
            for n in 0..bars {
                if next_on_depths[n as usize] {
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

        let Some(dependencies) = result.get(tag_path) else {
            output.warning_fmt_ln(format_args!("{tag_path} (BROKEN)"));
            return;
        };

        if stack.iter().any(|t| *t == tag_path) {
            output.neutral_fmt_ln(format_args!("{tag_path} (circular)"));
            return;
        }
        else if depth == max_depth && !dependencies.is_empty() {
            output.neutral_fmt_ln(format_args!("{tag_path} (minimized)"));
            return;
        }
        else {
            output.neutral_fmt_ln(format_args!("{tag_path}"));
        }

        stack.push(tag_path);

        fn print_dependencies<'a, I: Iterator<Item = &'a TagPath>>(
            dependencies: I,
            depth: u32,
            max_depth: u32,
            tags_with_broken_dependencies: &Option<&[&TagPath]>,
            next_on_depths: &mut Vec<bool>,
            result: &'a HashMap<TagPath, Vec<TagPath>>,
            stack: &mut Vec<&'a TagPath>,
            output: &StdoutLogger,
        ) {
            let mut dependencies = dependencies.peekable();
            while let Some(d) = dependencies.next() {
                let inner_depth = depth + 1;
                let next_same_level = dependencies.peek().is_some();
                next_on_depths.push(next_same_level);
                print_tags(d, result, inner_depth, max_depth, next_same_level, &tags_with_broken_dependencies, output, next_on_depths, stack);
                next_on_depths.pop();
            }
        }

        if let Some(n) = tags_with_broken_dependencies {
            let dependencies = dependencies.iter().filter(|t| n.contains(t) || !result.contains_key(t));
            print_dependencies(dependencies, depth, max_depth, tags_with_broken_dependencies, next_on_depths, result, stack, output);
        }
        else {
            let dependencies = dependencies.iter();
            print_dependencies(dependencies, depth, max_depth, tags_with_broken_dependencies, next_on_depths, result, stack, output);
        }

        let _popped = stack.pop();
        debug_assert_eq!(_popped, Some(tag_path));
    }

    // Sort out all root tags.
    let mut next_on_depths = Vec::new();
    let output = make_stdout_logger();
    let root_tags = all_tags.iter().filter(|&t| {
        for r in &result {
            if r.1.contains(t) {
                return false
            }
            if tags_with_broken_dependencies.is_some_and(|t| !t.contains(&r.0)) {
                continue;
            }
        }
        true
    });

    let mut stack = Vec::new();
    for i in root_tags {
        print_tags(i, &result, 0, max_depth, false, &tags_with_broken_dependencies, output.as_ref(), &mut next_on_depths, &mut stack);
    }
    Ok(())
}
