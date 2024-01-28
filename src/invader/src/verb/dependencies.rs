use std::env::Args;
use cli::{CommandLineParser, Parameter};
use ringhopper::primitives::primitive::TagPath;
use ringhopper::tag::dependency::{get_reverse_dependencies_for_tag, get_tag_dependencies_for_block, recursively_get_dependencies_for_tag};
use ringhopper::tag::tree::TagTree;
use util::get_tags_directory;

pub fn dependencies(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_tags(true)
        .add_custom_parameter(Parameter::single(
            "recursive",
            'r',
            "List all tags depended by this tag and its descendents.",
            "",
            None
        ))
        .add_custom_parameter(Parameter::single(
            "reverse",
            'R',
            "List all tags that depend on this tag. Cannot be used with --recursive.",
            "",
            None
        ))
        .add_help()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tags = get_tags_directory(&parser)?;
    let tag_path = str_unwrap!(TagPath::from_path(&parser.get_extra()[0]), "Invalid tag path: {error}");
    let recursive = parser.get_custom("recursive").is_some();
    let reverse = parser.get_custom("reverse").is_some();

    if recursive && reverse {
        return Err("--reverse and --recursive are not yet supported together".to_owned());
    }

    let result = if reverse {
        str_unwrap!(get_reverse_dependencies_for_tag(&tag_path, &tags), "Failed to get reverse dependencies: {error}")
    }
    else {
        if recursive {
            str_unwrap!(recursively_get_dependencies_for_tag(&tag_path, &tags), "Failed to recursively get dependencies: {error}")
        }
        else {
            let tag = str_unwrap!(tags.open_tag_copy(&tag_path), "Failed to open tag: {error}");
            get_tag_dependencies_for_block(tag.as_ref())
        }
    };

    let mut vec = Vec::with_capacity(result.len());
    for r in result {
        vec.push(r);
    }
    vec.sort();

    for i in vec {
        println!("{i}")
    }

    Ok(())
}
