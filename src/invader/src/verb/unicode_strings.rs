use std::env::Args;
use cli::CommandLineParser;
use ringhopper::tag::unicode_string_list::*;
use ringhopper::definitions::UnicodeStringList;
use ringhopper::primitives::tag::PrimaryTagStructDyn;

pub fn unicode_strings(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_tags(false)
        .add_data()
        .add_help()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let text_file_name = parser.get_extra()[0].to_owned() + ".txt";
    let text_file = parser.get_data().join(text_file_name);

    let file = std::fs::read(&text_file)
        .map_err(|e| format!("Failed to read {}: {e}", text_file.display()))?;

    let tag = UnicodeStringList::from_text_data(file.as_slice())
        .map_err(|e| format!("Failed to parse {}: {e}", text_file.display()))?;

    let tag_file_name = parser.get_extra()[0].to_owned() + ".unicode_string_list";
    let tag_file = parser.get_tags()[0].join(tag_file_name);

    std::fs::create_dir_all(tag_file.parent().unwrap())
        .map_err(|e| format!("Failed to write {}: {e}", tag_file.display()))?;

    std::fs::write(&tag_file, tag.to_tag_file().expect("should be ok"))
        .map_err(|e| format!("Failed to write {}: {e}", tag_file.display()))
}
