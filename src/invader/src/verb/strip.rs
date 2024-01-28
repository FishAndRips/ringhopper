use std::env::Args;
use cli::CommandLineParser;
use ringhopper::tag::tree::{TagTree};
use threading::do_with_threads;

pub fn strip(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_tags(false)
        .add_help()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();
    do_with_threads(parser, &tag, None, |context, path| {
        let tag = context.tags_directory.open_tag_copy(&path)?;
        context.tags_directory.write_tag(path, tag.as_ref())
    })
}
