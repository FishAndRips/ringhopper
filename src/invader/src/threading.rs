use std::collections::VecDeque;
use std::io::{stderr, stdout, Write};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use cli::CommandLineArgs;
use ringhopper::error::RinghopperResult;
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

pub type ProcessFunction<T, U> = fn(&mut ThreadingContext<T>, &TagPath, &mut U) -> RinghopperResult<bool>;

#[derive(PartialEq, Copy, Clone)]
pub enum DisplayMode {
    /// Show all tags successfully processed
    ShowProcessed,

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
    function: ProcessFunction<T, U>,
) -> Result<(), String> {
    let mut context = ThreadingContext {
        tags_directory,
        args,
    };

    let success = Arc::new(AtomicU64::new(0));
    let total = Arc::new(AtomicU64::new(0));
    let failure = Arc::new(AtomicU64::new(0));

    if !TagFilter::is_filter(user_filter) {
        let tag_path = match group {
            Some(group) => TagPath::new(user_filter, group),
            None => TagPath::from_path(user_filter)
        }.map_err(|_| format!("Invalid tag path `{user_filter}`"))?;
        process_tags(&mut context, &success, &failure, &total, &tag_path, &mut user_data, display_mode, function);
    }
    else {
        let filter = TagFilter::new(user_filter, group);
        let tags: VecDeque<TagPath> = context.tags_directory.get_all_tags_with_filter(Some(&filter)).into();
        let thread_count = std::thread::available_parallelism().map(|t| t.get().max(1)).unwrap_or(1);
        match thread_count {
            1 => {
                for path in tags {
                    process_tags(&mut context, &success, &failure, &total, &path, &mut user_data, display_mode, function);
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
                    let mut user_data = user_data.clone();
                    threads.push(std::thread::spawn(move || {
                        loop {
                            let next = match tags.lock().unwrap().pop_front() {
                                Some(n) => n,
                                None => break
                            };
                            process_tags(&mut context, &success, &failure, &total, &next, &mut user_data, display_mode, function);
                        }
                    }))
                }
                for t in threads {
                    t.join().unwrap();
                }
            }
        }
    }

    let total = Arc::into_inner(total).unwrap().into_inner();
    if total == 0 {
        return Err(format!("No tags matched `{user_filter}`"))
    }

    let failure = Arc::into_inner(failure).unwrap().into_inner();
    let success = Arc::into_inner(success).unwrap().into_inner();

    if total > 1 && display_mode == DisplayMode::ShowProcessed {
        print!("Processed {success} / {total} tags");
        if failure > 0 {
            print!(", with {failure} error{s}", s = if failure == 1 { "" } else { "s" });
        }
        println!();
    }
    else if display_mode == DisplayMode::Silent && failure > 0 {
        println!("Failed to parse {failure} tag{s}", s = if failure == 1 { "" } else { "s" });
    }

    Ok(())
}

fn process_tags<T: TagTree + Send + Clone, U: Clone + Send + 'static>(
    context: &mut ThreadingContext<T>,
    success: &Arc<AtomicU64>,
    failure: &Arc<AtomicU64>,
    total: &Arc<AtomicU64>,
    path: &TagPath,
    user_data: &mut U,
    display_mode: DisplayMode,
    function: ProcessFunction<T, U>
) {
    total.fetch_add(1, Ordering::Relaxed);
    match function(context, &path, user_data) {
        Ok(true) => {
            success.fetch_add(1, Ordering::Relaxed);
            if display_mode == DisplayMode::ShowProcessed {
                writeln!(stdout().lock(), "Processed {path}").unwrap()
            }
        },
        Ok(false) => (),
        Err(e) => {
            failure.fetch_add(1, Ordering::Relaxed);
            writeln!(stderr().lock(), "Failed to process {path}: {e}").unwrap()
        }
    }
}
