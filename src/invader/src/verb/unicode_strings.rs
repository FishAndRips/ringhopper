use std::env::Args;
use cli::CommandLineParser;
use ringhopper::tag::unicode_string_list::*;
use ringhopper::definitions::UnicodeStringList;
use ringhopper::primitives::primitive::{TagGroup, TagPath};
use ringhopper::tag::tree::TagTree;
use util::read_file;

pub fn unicode_strings(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_tags(false)
        .add_data()
        .add_help()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag_path = str_unwrap!(TagPath::new(&parser.get_extra()[0], TagGroup::UnicodeStringList), "Invalid tag path: {error}");

    let text_file_name = parser.get_extra()[0].to_owned() + ".txt";
    let text_file = parser.get_data().join(text_file_name);

    let file = read_file(&text_file)?;
    let tag = str_unwrap!(UnicodeStringList::from_text_data(file.as_slice()), "Failed to parse {}: {error}", text_file.display());

    str_unwrap!(parser.get_virtual_tags_directory()
        .write_tag(&tag_path, &tag),
        "Failed to write tag: {error}");

    println!("Saved {tag_path} successfully");
    Ok(())
}
