use std::num::NonZeroUsize;
use std::{env::Args, sync::Arc};
use cli::CommandLineParser;
use ringhopper::tag::tree::{CachingTagTree, CachingTagTreeWriteStrategy};
use ringhopper::primitives::primitive::TagGroup;
use ringhopper::tag::verify::verify;
use ringhopper_engines::Engine;
use threading::{DisplayMode, do_with_threads, ProcessSuccessType};
use util::make_stdout_logger;
use verb::print_tag_results;
use crate::util::StdoutLogger;

#[derive(Clone)]
struct UserData {
    engine: &'static Engine,
    logger: Arc<StdoutLogger>
}

pub fn verify_scenario(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<scenario*> [args]")
        .add_tags(true)
        .add_help()
        .add_engine()
        .add_jobs()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();
    let data = UserData {
        engine: parser.get_engine(),
        logger: make_stdout_logger()
    };

    let logger = data.logger.clone();
    let tree = Arc::new(CachingTagTree::new(parser.get_virtual_tags_directory(), CachingTagTreeWriteStrategy::Manual));

    do_with_threads(tree, parser, &tag, Some(TagGroup::Scenario), data, DisplayMode::Silent, logger, |context, scenario_path, user_data, _| {
        let threads = unsafe { NonZeroUsize::new_unchecked(2) }; // TODO: add subjobs later?
        let everything = verify(scenario_path, context.tags_directory.clone(), user_data.engine, threads)?;
        let locked = user_data.logger.lock();
        print_tag_results(&locked, &everything, format_args!("Verified {scenario_path}"));
        Ok(ProcessSuccessType::Success)
    })
}
