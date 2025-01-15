use std::env::Args;
use std::path::PathBuf;
use std::sync::Arc;
use crate::cli::{CommandLineParser, CommandLineValue, CommandLineValueType, Parameter};
use ringhopper::primitives::primitive::{TagGroup, TagPath};
use ringhopper::tag::archive::*;
use ringhopper::tag::tree::{CachingTagTree, CachingTagTreeWriteStrategy};
use crate::util::make_stdout_logger;

fn archive_command(args: Args, description: &'static str, full_scenario: bool) -> Result<(), String> {
    let usage = if full_scenario { "<tag> [args]" } else { "<tag.group> [args]" };

    let parser = CommandLineParser::new(description, usage)
        .add_tags(true)
        .add_help()
        .add_overwrite()
        .add_custom_parameter(Parameter::new("output", 'O', "Output filename. Default: <scenario_basename>.7z", "<file>", Some(CommandLineValueType::Path), 1, None, false, false))
        .add_custom_parameter(Parameter::new("level", 'l', "LZMA compression level. Must be between 0 and 9. Default: 7", "<lvl>", Some(CommandLineValueType::UInteger), 1, Some(vec![CommandLineValue::UInteger(7)]), false, false))
        .set_required_extra_parameters(1);

    let parser = if full_scenario {
        parser.add_engine()
    }
    else {
        parser
    }.parse(args)?;

    let logger = make_stdout_logger();
    let path = if full_scenario {
        TagPath::new(&parser.get_extra()[0], TagGroup::Scenario)
    }
    else {
        TagPath::from_path(&parser.get_extra()[0])
    }.map_err(|e| format!("{e}"))?;

    let cache = Arc::new(CachingTagTree::new(parser.get_virtual_tags_directory(), CachingTagTreeWriteStrategy::Manual));
    let overwrite = parser.get_overwrite();
    let level = LZMACompressionLevel::new(parser.get_custom("level").unwrap()[0].uinteger())
        .map_err(|e| format!("{e}"))?;

    let zip_path = parser.get_custom("output").map_or_else(|| PathBuf::from(format!("{}.7z", path.base_name())), |o| o[0].path().to_owned());
    if !overwrite && zip_path.exists() {
        logger.warning_fmt_ln(format_args!("{zip_path:?} already exists; skipping"));
        return Ok(())
    }

    let tag = if full_scenario {
        archive_map_to_7zip(&path, &cache, parser.get_engine(), level)
    }
    else {
        archive_tag_to_7zip(&path, &cache, level)
    }.map_err(|e| format!("{e}"))?;

    std::fs::write(&zip_path, tag).map_err(|e| format!("{e}"))?;

    logger.success_fmt_ln(format_args!("Wrote {zip_path:?}"));

    Ok(())
}

pub fn archive_tag(args: Args, description: &'static str) -> Result<(), String> {
    archive_command(args, description, false)
}

pub fn archive_scenario(args: Args, description: &'static str) -> Result<(), String> {
    archive_command(args, description, true)
}
