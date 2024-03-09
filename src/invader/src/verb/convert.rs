use std::env::Args;
use cli::CommandLineParser;
use ringhopper::error::Error;
use ringhopper::primitives::primitive::TagGroup;
use ringhopper::tag::convert::get_tag_conversion_fn;
use ringhopper::tag::tree::{TagTree};
use threading::{DisplayMode, do_with_threads, ProcessSuccessType};

pub fn convert(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> <group> [args]")
        .add_tags(false)
        .add_overwrite()
        .add_help()
        .set_required_extra_parameters(2)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();
    let group = TagGroup::from_str(&parser.get_extra()[1]).map_err(|_| format!("{} does not correspond to a tag group", parser.get_extra()[1]))?;

    do_with_threads(parser.get_virtual_tags_directory(), parser, &tag, None, group, DisplayMode::ShowAll, |context, path, to_group| {
        let to_group = *to_group;
        let from_group = path.group();

        let function = match get_tag_conversion_fn(from_group, to_group) {
            Some(n) => n,
            None => return Err(Error::Other(format!("{from_group} cannot convert into {to_group}")))
        };

        let mut new_path = path.clone();
        new_path.set_group(to_group);
        if !context.args.get_overwrite() && context.tags_directory.contains(&new_path) {
            return Ok(ProcessSuccessType::Skipped("file already exists"));
        }

        let tag = context.tags_directory.open_tag_copy(&path)?;
        let new_tag = function(tag.as_ref())?;

        ProcessSuccessType::wrap_write_result(context.tags_directory.write_tag(&new_path, new_tag.as_ref()))
    })
}
