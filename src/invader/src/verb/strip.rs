use std::env::Args;
use crate::cli::CommandLineParser;
use ringhopper::tag::tree::TagTree;
use crate::threading::{DisplayMode, do_with_threads, ProcessSuccessType};
use crate::util::make_stdout_logger;

pub fn strip(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag.group*> [args]")
        .add_tags(false)
        .add_help()
        .add_cow_tags()
        .add_jobs()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();
    do_with_threads(parser.get_virtual_tags_directory(), parser, &tag, None, (), DisplayMode::ShowAll, make_stdout_logger(), |context, path, _, _| {
        let tag = context.tags_directory.open_tag_copy(&path)?;
        ProcessSuccessType::wrap_write_result(context.tags_directory.write_tag(path, tag.as_ref()))
    })
}
