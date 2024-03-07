use std::env::Args;
use std::path::Path;
use std::sync::{Arc, Mutex};
use cli::CommandLineParser;
use ringhopper::map::GearboxCacheFile;
use ringhopper::tag::tree::{AtomicTagTree, TagTree};
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
    let loc_path = map_path.parent().unwrap().join("loc.map");

    let map_data = read_file(map_path).unwrap();
    let bitmaps = read_file(bitmaps_path).unwrap_or(Vec::new());
    let sounds = read_file(sounds_path).unwrap_or(Vec::new());
    let loc = read_file(loc_path).unwrap_or(Vec::new());

    let map = GearboxCacheFile::new(map_data, bitmaps, sounds, loc).unwrap();
    let tag = parser.get_extra()[1].clone();

    let output_tags_dir = AtomicTagTree::new(parser.get_virtual_tags_directory());

    do_with_threads(Arc::new(map), parser, &tag, None, output_tags_dir, DisplayMode::ShowProcessed, |context, path, output_tags_dir| {
        if !context.args.get_overwrite() && output_tags_dir.contains(path) {
            return Ok(false)
        }
        let tag = context.tags_directory.open_tag_copy(path)?;
        output_tags_dir.write_tag(path, tag.as_ref()).map(|_| true)
    })?;

    Ok(())
}
