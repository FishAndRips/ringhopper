use std::env::Args;
use std::path::PathBuf;
use std::sync::Arc;
use cli::{CommandLineParser, CommandLineValue, CommandLineValueType, Parameter};
use ringhopper::error::Error;
use ringhopper::primitives::engine::Engine;
use ringhopper::primitives::primitive::TagGroup;
use ringhopper::tag::archive::{archive_map_to_zip, ZstandardCompressionLevel};
use ringhopper::tag::tree::{CachingTagTree, CachingTagTreeWriteStrategy, VirtualTagsDirectory};
use threading::{DisplayMode, do_with_threads, ProcessSuccessType};
use util::make_stdout_logger;

#[derive(Clone)]
struct UserData {
    cache: Arc<CachingTagTree<VirtualTagsDirectory>>,
    engine: &'static Engine,
    overwrite: bool,
    level: ZstandardCompressionLevel
}

pub fn archive_map(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag*> [args]")
        .add_tags(false)
        .add_help()
        .add_engine()
        .add_jobs()
        .add_overwrite()
        .add_custom_parameter(Parameter::new("level", 'l', "Zstandard compression level. Must be between 0 and 22. Default: 3", "<lvl>", Some(CommandLineValueType::Integer), 1, Some(vec![CommandLineValue::Integer(3)]), false, false))
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();

    let cache = Arc::new(CachingTagTree::new(parser.get_virtual_tags_directory(), CachingTagTreeWriteStrategy::Manual));
    let engine = parser.get_engine();
    let overwrite = parser.get_overwrite();
    let level = ZstandardCompressionLevel::new(parser.get_custom("level").unwrap()[0].integer())
        .map_err(|e| format!("{e:?}"))?;

    let user_data = UserData { cache, engine, overwrite, level };

    do_with_threads(parser.get_virtual_tags_directory(), parser, &tag, Some(TagGroup::Scenario), user_data, DisplayMode::ShowAll, make_stdout_logger(), |_context, path, user_data, _| {
        let zip_path = PathBuf::from(format!("{}.zip", path.base_name()));
        if !user_data.overwrite && zip_path.exists() {
            return Ok(ProcessSuccessType::Skipped("zip already exists"))
        }

        let map = archive_map_to_zip(&path, &user_data.cache, user_data.engine, user_data.level)?;
        std::fs::write(&zip_path, map).map_err(|e| Error::FailedToWriteFile(zip_path, e))?;

        Ok(ProcessSuccessType::Success)
    })
}
