use std::env::Args;
use cli::CommandLineParser;
use ringhopper::tag::unicode_string_list::*;
use ringhopper::definitions::UnicodeStringList;
use ringhopper::error::Error;
use ringhopper::primitives::primitive::TagGroup;
use ringhopper::tag::tree::TagTree;
use threading::do_with_threads;
use util::read_file;

pub fn unicode_strings(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_tags(false)
        .add_data()
        .add_help()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();
    do_with_threads(parser, &tag, Some(TagGroup::UnicodeStringList), |context, path| {
        let mut full_data_path = context.args.get_data().join(path.to_native_path());
        full_data_path.set_extension("txt");
        let text_file = read_file(full_data_path)?;
        let tag = UnicodeStringList::from_text_data(text_file.as_slice())
            .map_err(|e| Error::Other(e.to_string()))?;
        context.tags_directory.write_tag(path, &tag)
    })
}
