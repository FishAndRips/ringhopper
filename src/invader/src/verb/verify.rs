use std::{env::Args, sync::Arc};
use cli::CommandLineParser;
use ringhopper::tag::verify::VerifyContext;
use ringhopper::tag::tree::{CachingTagTree, CachingTagTreeWriteStrategy};
use ringhopper::primitives::primitive::TagGroup;
use ringhopper_engines::Engine;
use threading::{DisplayMode, do_with_threads, ProcessSuccessType};
use util::make_stdout_logger;
use crate::util::StdoutLogger;

#[derive(Clone)]
struct UserData {
    engine: &'static Engine,
    logger: Arc<StdoutLogger>
}

pub fn verify(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<scenario*> [args]")
        .add_tags(false)
        .add_help()
        .add_cow_tags()
        .add_engine()
        .add_jobs()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();

    if !parser.get_engine().build_target {
        return Err(format!("Engine `{}` is not a valid build target", parser.get_engine().name))
    }

    let data = UserData {
        engine: parser.get_engine(),
        logger: make_stdout_logger()
    };

    let logger = data.logger.clone();
    let tree = Arc::new(CachingTagTree::new(parser.get_virtual_tags_directory(), CachingTagTreeWriteStrategy::Instant));

    do_with_threads(tree, parser, &tag, Some(TagGroup::Scenario), data, DisplayMode::Silent, logger, |context, scenario_path, user_data, _| {
        let everything = VerifyContext::verify(scenario_path, &context.tags_directory, user_data.engine)?;

        let locked = user_data.logger.lock();

        let total_issues = everything
            .iter()
            .map(|a| a.1.errors.len() + a.1.warnings.len() + a.1.pedantic_warnings.len())
            .reduce(|a,b| a + b)
            .unwrap_or_default();

        match total_issues {
            0 => locked.success_fmt_ln(format_args!("Verified {scenario_path} and found no issues")),
            1 => locked.warning_fmt_ln(format_args!("Verified {scenario_path} and found one issue:")),
            other => locked.warning_fmt_ln(format_args!("Verified {scenario_path} and found {other} issues:"))
        }

        // First pass: warnings
        for (path, vr) in &everything {
            for i in &vr.pedantic_warnings {
                locked.minor_warning_fmt_ln(format_args!("- {path} warning (minor): {i}"))
            }

            for i in &vr.warnings {
                locked.warning_fmt_ln(format_args!("- {path} warning: {i}"))
            }
        }

        // Second pass: errors
        for (path, vr) in &everything {
            for i in &vr.errors {
                locked.error_fmt_ln(format_args!("- {path} error: {i}"))
            }
        }

        Ok(ProcessSuccessType::Success)
    })
}
