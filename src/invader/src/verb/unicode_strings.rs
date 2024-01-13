use std::env::Args;
use cli::CommandLineParser;
use ringhopper::tag::unicode_string_list::*;
use ringhopper::definitions::UnicodeStringList;
use ringhopper::primitives::primitive::{TagGroup, TagPath};
use ringhopper::tag::tree::TagTree;

pub fn unicode_strings(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_tags(false)
        .add_data()
        .add_help()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag_path = TagPath::new(&parser.get_extra()[0], TagGroup::UnicodeStringList)
        .map_err(|e| format!("Invalid tag path: {e}"))?;

    let text_file_name = parser.get_extra()[0].to_owned() + ".txt";
    let text_file = parser.get_data().join(text_file_name);

    let file = std::fs::read(&text_file)
        .map_err(|e| format!("Failed to read {}: {e}", text_file.display()))?;

    let tag = UnicodeStringList::from_text_data(file.as_slice())
        .map_err(|e| format!("Failed to parse {}: {e}", text_file.display()))?;

    parser.get_virtual_tags_directory()
        .write_tag(&tag_path, &tag)
        .map_err(|e| format!("Failed to write tag: {e}"))?;

    println!("Saved {tag_path} successfully");
    Ok(())
}
