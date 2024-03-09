use std::env::Args;
use cli::CommandLineParser;
use ringhopper::tag::unicode_string_list::*;
use ringhopper::definitions::UnicodeStringList;
use ringhopper::error::Error;
use ringhopper::primitives::primitive::TagGroup;
use ringhopper::tag::tree::TagTree;
use threading::{DisplayMode, do_with_threads, ProcessSuccessType};
use util::read_file;

pub fn unicode_strings(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_tags(false)
        .add_data()
        .add_help()
        .add_cow_tags()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();
    do_with_threads(parser.get_virtual_tags_directory(), parser, &tag, Some(TagGroup::UnicodeStringList), (), DisplayMode::ShowAll, |context, path, _| {
        let mut full_data_path = context.args.get_data().join(path.to_native_path());
        full_data_path.set_extension("txt");
        let text_file = read_file(full_data_path)?;
        let tag = UnicodeStringList::from_text_data(text_file.as_slice())
            .map_err(|e| Error::Other(e.to_string()))?;
        ProcessSuccessType::wrap_write_result(context.tags_directory.write_tag(path, &tag))
    })
}
