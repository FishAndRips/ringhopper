use std::env::Args;
use cli::CommandLineParser;
use ringhopper::tag::nudge::{is_nudgeable, nudge_tag};
use ringhopper::tag::tree::{TagTree};
use threading::do_with_threads;

pub fn nudge(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_tags(false)
        .add_help()
        .add_cow_tags()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();
    do_with_threads(parser.get_virtual_tags_directory(), parser, &tag, None, (), |context, path, _| {
        if !is_nudgeable(path.group()) {
            return Ok(false)
        }
        let mut tag = context.tags_directory.open_tag_copy(&path)?;
        if nudge_tag(tag.as_mut()) {
            context.tags_directory.write_tag(path, tag.as_ref())
        }
        else {
            Ok(false)
        }
    })
}
