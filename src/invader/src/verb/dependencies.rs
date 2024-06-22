use std::env::Args;
use cli::{CommandLineParser, Parameter};
use ringhopper::primitives::primitive::{TagGroup, TagPath};
use ringhopper::tag::dependency::*;
use ringhopper::tag::tree::TagTree;

use crate::util::make_stdout_logger;

pub fn dependencies(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_tags(true)
        .add_custom_parameter(Parameter::single(
            "recursive",
            'r',
            "List all tags depended by this tag and its descendents.",
            "",
            None
        ))
        .add_custom_parameter(Parameter::single(
            "reverse",
            'R',
            "List all tags that depend on this tag. Cannot be used with --recursive.",
            "",
            None
        ))
        .add_help()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tags = parser.get_virtual_tags_directory();
    let tag_path = str_unwrap!(TagPath::from_path(&parser.get_extra()[0]), "Invalid tag path: {error}");
    let recursive = parser.get_custom("recursive").is_some();
    let reverse = parser.get_custom("reverse").is_some();

    if recursive && reverse {
        return Err("--reverse and --recursive are not supported together".to_owned());
    }

    let result = if reverse {
        str_unwrap!(get_reverse_dependencies_for_tag(&tag_path, &tags), "Failed to get reverse dependencies: {error}")
    }
    else {
        if recursive {
            let result = str_unwrap!(recursively_get_dependencies_for_tag(&tag_path, &tags, false), "Failed to recursively get dependencies: {error}");
            result.into_values().flatten().collect()
        }
        else {
            let tag = str_unwrap!(tags.open_tag_copy(&tag_path), "Failed to open tag: {error}");
            get_tag_dependencies_for_block(tag.as_ref().as_dynamic())
        }
    };

    let mut dependencies_sorted = Vec::with_capacity(result.len());
    dependencies_sorted.extend(result);
    dependencies_sorted.sort();

    let logger = make_stdout_logger();
    for i in dependencies_sorted {
        logger.neutral_fmt_ln(format_args!("{i}"))
    }

    Ok(())
}


pub fn list_scenario_tags(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> <args>")
        .add_tags(true)
        .add_engine()
        .add_help()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tags = parser.get_virtual_tags_directory();
    let tag_path = str_unwrap!(TagPath::new(&parser.get_extra()[0], TagGroup::Scenario), "Invalid tag path: {error}");
    let dependencies = str_unwrap!(recursively_get_dependencies_for_map(&tag_path, &tags, parser.get_engine()), "Failed to get reverse dependencies: {error}");

    let mut dependencies_sorted: Vec<TagPath> = Vec::with_capacity(dependencies.len());
    dependencies_sorted.extend(dependencies);
    dependencies_sorted.sort();

    let logger = make_stdout_logger();
    for i in dependencies_sorted {
        logger.neutral_fmt_ln(format_args!("{i}"))
    }

    Ok(())
}
