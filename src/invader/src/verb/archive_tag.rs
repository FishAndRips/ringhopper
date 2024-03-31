use std::env::Args;
use std::path::PathBuf;
use std::sync::Arc;
use cli::{CommandLineParser, CommandLineValue, CommandLineValueType, Parameter};
use ringhopper::primitives::primitive::TagPath;
use ringhopper::tag::archive::{archive_tag_to_zip, ZstandardCompressionLevel};
use ringhopper::tag::tree::{CachingTagTree, CachingTagTreeWriteStrategy};
use util::make_stdout_logger;

pub fn archive_tag(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_tags(false)
        .add_help()
        .add_overwrite()
        .add_custom_parameter(Parameter::new("level", 'l', "Zstandard compression level. Must be between 0 and 22. Default: 6", "<lvl>", Some(CommandLineValueType::Integer), 1, Some(vec![CommandLineValue::Integer(6)]), false, false))
        .set_required_extra_parameters(1)
        .parse(args)?;

    let logger = make_stdout_logger();

    let path = TagPath::from_path(&parser.get_extra()[0]).map_err(|e| format!("{e:?}"))?;

    let cache = Arc::new(CachingTagTree::new(parser.get_virtual_tags_directory(), CachingTagTreeWriteStrategy::Manual));
    let overwrite = parser.get_overwrite();
    let level = ZstandardCompressionLevel::new(parser.get_custom("level").unwrap()[0].integer())
        .map_err(|e| format!("{e:?}"))?;

    let zip_path = PathBuf::from(format!("{}.zip", path.base_name()));
    if !overwrite && zip_path.exists() {
        logger.warning_fmt_ln(format_args!("{zip_path:?} already exists; skipping"));
        return Ok(())
    }

    let tag = archive_tag_to_zip(&path, &cache, level).map_err(|e| format!("{e:?}"))?;
    std::fs::write(&zip_path, tag).map_err(|e| format!("{e:?}"))?;

    logger.success_fmt_ln(format_args!("Wrote {zip_path:?}"));

    Ok(())
}
