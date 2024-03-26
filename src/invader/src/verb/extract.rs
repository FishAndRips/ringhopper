use std::env::Args;
use std::path::Path;
use std::sync::Arc;
use cli::{CommandLineParser, CommandLineValue, CommandLineValueType, Parameter};
use ringhopper::definitions::ScenarioType;
use ringhopper::map::load_map_from_filesystem;
use ringhopper::primitives::primitive::TagGroup;
use ringhopper::primitives::tag::ParseStrictness;
use ringhopper::tag::tree::{TagTree, VirtualTagsDirectory};
use threading::{DisplayMode, do_with_threads, ProcessSuccessType};
use util::make_stdout_logger;

pub fn extract(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<map>")
        .add_tags(false)
        .add_overwrite()
        .add_help()
        .add_custom_parameter(Parameter::new(
            "filter",
            'f',
            "Filter tags to extract. By default, all tags are extracted.",
            "<param>",
            Some(CommandLineValueType::String),
            1,
            Some(vec![CommandLineValue::String("*".to_owned())]),
            false,
            false
        ))
        .add_jobs()
        .add_custom_parameter(Parameter::new(
            "non-mp-globals",
            'n',
            "Allow non-multiplayer globals tags to be extracted. Note that such globals tags may only safely be used with scenarios of the same type or lower.",
            "",
            None,
            0,
            None,
            false,
            false
        ))
        .set_required_extra_parameters(1)
        .parse(args)?;

    let map_path = Path::new(&parser.get_extra()[0]);

    let map = load_map_from_filesystem(map_path, ParseStrictness::Strict).map_err(|e| format!("Cannot load {map_path:?} as a cache file: {e:?}"))?;
    let tag = parser.get_custom("filter").unwrap()[0].string().to_owned();

    #[derive(Clone)]
    struct UserData {
        allow_globals: bool,
        output_tags_dir: Arc<VirtualTagsDirectory>
    }

    let allow_globals = parser.get_custom("non-mp-globals").is_some() || map.get_scenario_type() == ScenarioType::Multiplayer;
    let output_tags_dir = Arc::new(parser.get_virtual_tags_directory());

    let user_data = UserData {
        allow_globals, output_tags_dir
    };

    do_with_threads(map, parser, &tag, None, user_data, DisplayMode::ShowAll, make_stdout_logger(), |context, path, user_data, _| {
        if !context.args.get_overwrite() && user_data.output_tags_dir.contains(path) {
            return Ok(ProcessSuccessType::Skipped("file already exists"))
        }
        if !user_data.allow_globals && path.group() == TagGroup::Globals {
            return Ok(ProcessSuccessType::Skipped("refusing to extract a non-multiplayer globals tag without --non-mp-globals"))
        }
        let tag = context.tags_directory.open_tag_copy(path)?;
        let tag = tag.as_ref();
        user_data.output_tags_dir.write_tag_to_directory(path, tag, 0).map(|r|
            if r {
                ProcessSuccessType::Success
            }
            else {
                ProcessSuccessType::Skipped("file on disk matches tag")
            }
        )
    })?;

    Ok(())
}
