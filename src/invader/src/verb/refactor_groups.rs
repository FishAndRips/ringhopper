use std::env::Args;
use crate::cli::CommandLineParser;
use ringhopper::{definitions::get_all_referenceable_tag_groups_for_group, primitives::primitive::TagGroup, tag::{dependency::refactor_groups_for_block, tree::TagTree}};
use crate::threading::{DisplayMode, do_with_threads, ProcessSuccessType};
use crate::util::make_stdout_logger;

use crate::cli::*;

pub fn refactor_groups(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<group-from> <group-to> [args]")
        .add_tags(false)
        .add_help()
        .add_cow_tags()
        .add_jobs()
        .add_custom_parameter(Parameter::new(
            "filter",
            'f',
            "Filter what tags that can be edited. By default, all tags will be edited if possible.",
            "<tag.group*>",
            Some(CommandLineValueType::String),
            1,
            Some(vec![CommandLineValue::String("*".to_owned())]),
            false,
            false
        ))
        .set_required_extra_parameters(2)
        .parse(args)?;

    let parse_group = |group: &str| -> Result<TagGroup, String> {
        TagGroup::from_str(group).map_err(|_| format!("{group} does not correspond to a valid tag group"))
    };

    #[derive(Clone)]
    struct UserData {
        group_from: TagGroup,
        group_to: TagGroup,
    }

    let group_from = parse_group(&parser.get_extra()[0])?;
    let group_to = parse_group(&parser.get_extra()[1])?;

    let user_data = UserData {
        group_from,
        group_to,
    };

    let tag = parser.get_custom("filter").unwrap()[0].string().to_owned();
    do_with_threads(parser.get_virtual_tags_directory(), parser, &tag, None, user_data, DisplayMode::ShowAll, make_stdout_logger(), |context, path, user_data, _| {
        let all_referenceable = get_all_referenceable_tag_groups_for_group(path.group());

        // Can it reference from OR to?
        if !all_referenceable.contains(&user_data.group_from) || !all_referenceable.contains(&user_data.group_to) {
            return Ok(ProcessSuccessType::Ignored)
        }

        // Actually open and do something.
        let mut tag = context.tags_directory.open_tag_copy(&path)?;
        if !refactor_groups_for_block(tag.as_mut_dynamic(), user_data.group_from, user_data.group_to, &context.tags_directory, true) {
            return Ok(ProcessSuccessType::Skipped("no matching references found"))
        }
        ProcessSuccessType::wrap_write_result(context.tags_directory.write_tag(path, tag.as_ref()))
    })
}
