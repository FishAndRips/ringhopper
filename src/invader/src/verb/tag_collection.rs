use std::env::Args;
use cli::CommandLineParser;
use ringhopper::definitions::{TagCollection, UIWidgetCollection};
use ringhopper::error::Error;
use ringhopper::primitives::primitive::TagGroup;
use ringhopper::tag::tag_collection::TagCollectionFunctions;
use ringhopper::tag::tree::TagTree;
use threading::*;
use util::read_file;

macro_rules! make_tag_collection_fn {
    ($name:tt, $tag_struct:tt) => {
        pub fn $name(args: Args, description: &'static str) -> Result<(), String> {
            let parser = CommandLineParser::new(description, "<tag> [args]")
                .add_tags(false)
                .add_data()
                .add_help()
                .add_cow_tags()
                .set_required_extra_parameters(1)
                .parse(args)?;

            let tag = parser.get_extra()[0].clone();
            do_with_threads(parser.get_virtual_tags_directory(), parser, &tag, Some(TagGroup::$tag_struct), (), DisplayMode::ShowProcessed, |context, path, _| {
                let mut full_data_path = context.args.get_data().join(path.to_native_path());
                full_data_path.set_extension("txt");
                let text_file = read_file(full_data_path)?;
                let tag = $tag_struct::from_text_data(text_file.as_slice())
                    .map_err(|e| Error::Other(e.to_string()))?;
                context.tags_directory.write_tag(path, &tag)
            })
        }
    };
}

make_tag_collection_fn!(tag_collection, TagCollection);
make_tag_collection_fn!(ui_widget_collection, UIWidgetCollection);
