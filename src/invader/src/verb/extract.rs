use std::env::Args;
use std::path::Path;
use std::sync::{Arc, Mutex};
use cli::CommandLineParser;
use ringhopper::map::GearboxCacheFile;
use ringhopper::tag::tree::TagTree;
use threading::{DisplayMode, do_with_threads};
use util::read_file;

pub fn extract(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<map> <tag>")
        .add_tags(false)
        .add_overwrite()
        .add_help()
        .set_required_extra_parameters(2)
        .parse(args)?;

    let map_path = Path::new(&parser.get_extra()[0]);

    let bitmaps_path = map_path.parent().unwrap().join("bitmaps.map");
    let sounds_path = map_path.parent().unwrap().join("sounds.map");

    // todo: read the header, then determine what parser to use, and then use it

    let map_data = read_file(map_path).unwrap();
    let bitmaps = read_file(bitmaps_path).unwrap();
    let sounds = read_file(sounds_path).unwrap();

    let map = GearboxCacheFile::new(map_data, bitmaps, sounds).unwrap();
    let tag = parser.get_extra()[1].clone();

    let output_tags_dir = Arc::new(Mutex::new(parser.get_virtual_tags_directory()));

    do_with_threads(map, parser, &tag, None, output_tags_dir, DisplayMode::ShowProcessed, |context, path, output_tags_dir| {
        let output = || output_tags_dir.lock().unwrap();

        if !context.args.get_overwrite() && output().contains(path) {
            return Ok(false)
        }

        let tag = context.tags_directory.open_tag_copy(path)?;
        output().write_tag(path, tag.as_ref()).map(|_| true)
    })?;

    Ok(())
}
