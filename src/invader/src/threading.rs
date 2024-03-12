use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use cli::CommandLineArgs;
use ringhopper::error::RinghopperResult;
use ringhopper::logger::Logger;
use ringhopper::primitives::primitive::{TagGroup, TagPath};
use ringhopper::tag::tree::{TagFilter, TagTree};

pub struct ThreadingContext<T: TagTree + Send + Clone> {
    pub args: CommandLineArgs,
    pub tags_directory: T
}

impl<T: TagTree + Send + Clone> Clone for ThreadingContext<T> {
    fn clone(&self) -> Self {
        Self {
            args: self.args.clone(),
            tags_directory: self.tags_directory.clone()
        }
    }
}

pub type ProcessFunction<T, U> = fn(&mut ThreadingContext<T>, &TagPath, &mut U, logger: &Arc<dyn Logger>) -> RinghopperResult<ProcessSuccessType>;

pub enum ProcessSuccessType {
    /// Used if successful; printed unless silent
    Success,

    /// Used if successful; printed unless silent
    Skipped(&'static str),

    /// Used if incompatible with the tool (e.g. nudging a tag that cannot be nudged); does not usually get printed
    Ignored
}

impl ProcessSuccessType {
    /// Wrap a result for VirtualTagsDirectory write tag result
    pub fn wrap_write_result(result: RinghopperResult<bool>) -> RinghopperResult<ProcessSuccessType> {
        result.map(|r| if r { ProcessSuccessType::Success } else { ProcessSuccessType::Skipped("file on disk matches tag") })
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum DisplayMode {
    /// Show all tags successfully processed or skipped
    ShowAll,

    /// Only show errors
    Silent
}

pub fn do_with_threads<T: TagTree + Send + 'static + Clone, U: Clone + Send + 'static>(
    tags_directory: T,
    args: CommandLineArgs,
    user_filter: &str,
    group: Option<TagGroup>,
    mut user_data: U,
    display_mode: DisplayMode,
    logger: Arc<dyn Logger>,
    function: ProcessFunction<T, U>,
) -> Result<(), String> {
    let start = std::time::Instant::now();

    let mut context = ThreadingContext {
        tags_directory,
        args,
    };

    let success = Arc::new(AtomicU64::new(0));
    let ignored = Arc::new(AtomicU64::new(0));
    let total = Arc::new(AtomicU64::new(0));
    let failure = Arc::new(AtomicU64::new(0));

    if !TagFilter::is_filter(user_filter) {
        let tag_path = match group {
            Some(group) => TagPath::new(user_filter, group),
            None => TagPath::from_path(user_filter)
        }.map_err(|_| format!("Invalid tag path `{user_filter}`"))?;
        process_tags(&mut context, &success, &failure, &ignored, &total, &tag_path, &mut user_data, display_mode, &logger, function);
    }
    else {
        let filter = TagFilter::new(user_filter, group);
        let tags: VecDeque<TagPath> = context.tags_directory.get_all_tags_with_filter(Some(&filter)).into();
        let thread_count = std::thread::available_parallelism().map(|t| t.get().max(1)).unwrap_or(1);
        match thread_count {
            1 => {
                for path in tags {
                    process_tags(&mut context, &success, &failure, &ignored, &total, &path, &mut user_data, display_mode, &logger, function);
                }
            },
            n => {
                let mut threads = Vec::with_capacity(n);
                let tags = Arc::new(Mutex::new(tags));
                for _ in 0..n {
                    let tags = tags.clone();
                    let mut context = context.clone();
                    let success = success.clone();
                    let total = total.clone();
                    let failure = failure.clone();
                    let ignored = ignored.clone();
                    let mut user_data = user_data.clone();
                    let logger = logger.clone();
                    threads.push(std::thread::spawn(move || {
                        loop {
                            let next = match tags.lock().unwrap().pop_front() {
                                Some(n) => n,
                                None => break
                            };
                            process_tags(&mut context, &success, &failure, &ignored, &total, &next, &mut user_data, display_mode, &logger, function);
                        }
                    }))
                }
                for t in threads {
                    t.join().unwrap();
                }
            }
        }
    }

    let ignored = Arc::into_inner(ignored).unwrap().into_inner();
    let total = Arc::into_inner(total).unwrap().into_inner() - ignored;
    if total == 0 {
        return Err(format!("No viable tags matched `{user_filter}`"))
    }

    let failure = Arc::into_inner(failure).unwrap().into_inner();
    let success = Arc::into_inner(success).unwrap().into_inner();

    let milliseconds_taken = (std::time::Instant::now() - start).as_millis();

    if display_mode == DisplayMode::Silent {
        if failure > 0 {
            logger.error_fmt_ln(format_args!("Failed to parse {failure} tag{s}", s = if failure == 1 { "" } else { "s" }));
        }
    }
    else if total > 1 {
        let processed_amt = format!("Saved {success} / {total} tags in {milliseconds_taken} ms");
        if failure > 0 {
            logger.warning_fmt_ln(format_args!("{processed_amt}, with {failure} error{s}", s = if failure == 1 { "" } else { "s" }));
        }
        else {
            logger.success_ln(&processed_amt);
        }
    }

    Ok(())
}

fn process_tags<T: TagTree + Send + Clone, U: Clone + Send + 'static>(
    context: &mut ThreadingContext<T>,
    success: &Arc<AtomicU64>,
    failure: &Arc<AtomicU64>,
    ignored: &Arc<AtomicU64>,
    total: &Arc<AtomicU64>,
    path: &TagPath,
    user_data: &mut U,
    display_mode: DisplayMode,
    logger: &Arc<dyn Logger>,
    function: ProcessFunction<T, U>
) {
    total.fetch_add(1, Ordering::Relaxed);
    match function(context, &path, user_data, logger) {
        Ok(ProcessSuccessType::Success) => {
            success.fetch_add(1, Ordering::Relaxed);
            if display_mode == DisplayMode::ShowAll {
                logger.success_fmt_ln(format_args!("Saved {path}"));
                logger.flush();
            }
        },
        Ok(ProcessSuccessType::Skipped(reason)) => if display_mode == DisplayMode::ShowAll {
            logger.neutral_fmt_ln(format_args!("Skipped {path}: {reason}"));
            logger.flush();
        },
        Ok(ProcessSuccessType::Ignored) => {
            ignored.fetch_add(1, Ordering::Relaxed);
        }
        Err(e) => {
            failure.fetch_add(1, Ordering::Relaxed);
            logger.error_fmt_ln(format_args!("Failed to process {path}: {e}"));
            logger.flush();
        }
    }
}
