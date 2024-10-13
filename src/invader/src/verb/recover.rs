use std::env::Args;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::cli::CommandLineParser;
use ringhopper::error::{Error, RinghopperResult};
use ringhopper::primitives::primitive::TagPath;
use ringhopper::tag::recover::get_recover_function;
use ringhopper::tag::tree::{TagTree, VirtualTagsDirectory};
use crate::threading::{DisplayMode, do_with_threads, ProcessSuccessType, ThreadingContext};
use crate::util::make_stdout_logger;
use crate::util::StdoutLogger;

struct UserData {
    overwrite: bool,
    data: PathBuf,
    logger: Arc<StdoutLogger>,

    success: AtomicUsize,
    total: AtomicUsize,
    error: AtomicUsize
}

pub fn recover(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag.group*> [args]")
        .add_tags(false)
        .add_data()
        .add_help()
        .add_overwrite()
        .add_jobs()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let logger = make_stdout_logger();

    let user_data = UserData {
        overwrite: parser.get_overwrite(),
        logger: logger.clone(),
        data: parser.get_data().to_path_buf(),

        success: AtomicUsize::default(),
        total: AtomicUsize::default(),
        error: AtomicUsize::default(),
    };

    let atomic = Arc::new(user_data);

    let tag = parser.get_extra()[0].clone();
    do_with_threads(parser.get_virtual_tags_directory(), parser, &tag, None, atomic.clone(), DisplayMode::Silent, logger.clone(), |context, path, user_data, _| {
        let result = recover_tag(user_data, path, context);

        match &result {
            Ok(ProcessSuccessType::Success) => { user_data.success.fetch_add(1, Ordering::Relaxed); }
            Ok(ProcessSuccessType::Skipped(_)) => (),
            Ok(ProcessSuccessType::Ignored) => return result,
            Err(e) => { user_data.logger.error_fmt_ln(format_args!("Failed to recover {path}: {e}")); user_data.error.fetch_add(1, Ordering::Relaxed); }
        }

        user_data.total.fetch_add(1, Ordering::Relaxed);
        result
    })?;

    let total = atomic.total.fetch_add(0, Ordering::Relaxed);
    let success = atomic.success.fetch_add(0, Ordering::Relaxed);
    let error = atomic.error.fetch_add(0, Ordering::Relaxed);

    if total > 1 {
        if success >= 1 {
            logger.success_fmt_ln(format_args!("Recovered {success} / {total} tags with {error} error(s)"));
        }
        else if error == 0 {
            logger.warning_fmt_ln(format_args!("Recovered 0 / {total} tags (all skipped)"));
        }
        else {
            logger.error_fmt_ln(format_args!("Recovered 0 / {total} tags with {error} error(s)"));
        }
    }

    Ok(())
}

fn recover_tag(user_data: &UserData, path: &TagPath, context: &ThreadingContext<VirtualTagsDirectory>) -> RinghopperResult<ProcessSuccessType> {
    let func = match get_recover_function(path.group()) {
        Some(n) => n,
        None => return Ok(ProcessSuccessType::Ignored)
    };

    let fs = match func(path, &context.tags_directory.open_tag_copy(path)?)? {
        Some(n) => n,
        None => return Ok(ProcessSuccessType::Skipped("no recoverable data in tag"))
    };

    let mut anything_saved = false;

    for (dpath, ddata) in fs {
        let full_data_path = user_data.data.join(&dpath);
        let path_as_utf8 = dpath.to_str().unwrap();

        if !user_data.overwrite && full_data_path.exists() {
            user_data.logger.warning_fmt_ln(format_args!("Skipping {path_as_utf8}; already exists"));
            continue;
        }

        let parent = full_data_path.parent().unwrap();
        std::fs::create_dir_all(parent).map_err(|e| Error::FailedToWriteFile(parent.to_path_buf(), e))?;
        std::fs::write(&full_data_path, ddata).map_err(|e| Error::FailedToWriteFile(full_data_path.clone(), e))?;
        user_data.logger.success_fmt_ln(format_args!("Recovered {path_as_utf8}"));

        anything_saved = true;
    }

    Ok(if anything_saved {
        ProcessSuccessType::Success
    }
    else {
        ProcessSuccessType::Skipped("no data written")
    })
}
