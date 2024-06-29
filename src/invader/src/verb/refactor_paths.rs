use std::env::Args;
use std::num::NonZeroUsize;
use std::time::Instant;
use cli::CommandLineParser;
use ringhopper::tag::tree::TagFilter;
use ringhopper::tag::dependency::{refactor_paths_for_tag_tree, ReplaceType};
use util::make_stdout_logger;
use verb::print_tag_results;

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

    let logger = make_stdout_logger();
    let logger = logger.lock();
    logger.neutral_ln("Refactoring... this might take a while (do not cancel this or you might break something)");
    logger.flush();

    let start = Instant::now();

    let tags_to_rename = match refactor_paths_for_tag_tree(
        parser.get_extra()[0].as_str(),
        parser.get_extra()[1].as_str(),
        &parser.get_virtual_tags_directory(),
        NonZeroUsize::new(parser.get_jobs()).unwrap(),
        match parser.get_custom("replace-type").unwrap()[0].string() {
            "start" => ReplaceType::Start,
            "all" => ReplaceType::All,
            n => return Err(format!("Unknown replace-type argument {n}"))
        },
        parser.get_custom("no-move").is_some(),
        &TagFilter::new(parser.get_custom("filter").unwrap()[0].string(), None)
    ) {
        Ok(n) => {
            for (from, to) in &n.0 {
                logger.success_fmt_ln(format_args!("Renamed {from} to {to}"))
            }
            print_tag_results(&logger, &n.1, format_args!("Renamed tags"));
            n.0
        },
        Err(e) => {
            print_tag_results(&logger, &e.1, format_args!("Failed to rename tags"));
            return Err(format!("{}", e.0))
        }
    };

    let time_elapsed = (Instant::now() - start).as_millis();
    logger.success_fmt_ln(format_args!("Renamed {} tags in {time_elapsed} ms", tags_to_rename.len()));

    Ok(())
}
