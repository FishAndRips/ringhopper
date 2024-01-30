use std::collections::VecDeque;
use std::io::{stderr, stdout, Write};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use cli::CommandLineArgs;
use ringhopper::error::RinghopperResult;
use ringhopper::primitives::primitive::{TagGroup, TagPath};
use ringhopper::tag::tree::{iterate_through_all_tags, TagFilter, VirtualTagsDirectory};

#[derive(Clone)]
pub struct ThreadingContext {
    pub args: CommandLineArgs,
    pub tags_directory: VirtualTagsDirectory
}

pub type ProcessFunction = fn(&mut ThreadingContext, &TagPath) -> RinghopperResult<()>;

pub fn do_with_threads(
    args: CommandLineArgs,
    user_filter: &str,
    group: Option<TagGroup>,
    function: ProcessFunction
) -> Result<(), String> {
    let mut context = ThreadingContext {
        tags_directory: args.get_virtual_tags_directory(),
        args,
    };

    let mut success = Arc::new(AtomicU64::new(0));
    let mut total = Arc::new(AtomicU64::new(0));

    if !TagFilter::is_filter(user_filter) {
        let tag_path = match group {
            Some(group) => TagPath::new(user_filter, group),
            None => TagPath::from_path(user_filter)
        }.map_err(|_| format!("Invalid tag path `{user_filter}`"))?;
        process_tags(&mut context, &success, &total, &tag_path, function);
    }
    else {
        let filter = TagFilter::new(user_filter, group);
        let tags: VecDeque<TagPath> = iterate_through_all_tags(&context.tags_directory, Some(&filter)).collect();
        let thread_count = std::thread::available_parallelism().map(|t| t.get().max(1)).unwrap_or(1);
        match thread_count {
            1 => {
                for path in tags {
                    process_tags(&mut context, &success, &total, &path, function);
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
                    threads.push(std::thread::spawn(move || {
                        loop {
                            let next = match tags.lock().unwrap().pop_front() {
                                Some(n) => n,
                                None => break
                            };
                            process_tags(&mut context, &success, &total, &next, function);
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

    Ok(())
}

fn process_tags(context: &mut ThreadingContext, success: &Arc<AtomicU64>, total: &Arc<AtomicU64>, path: &TagPath, function: ProcessFunction) {
    total.fetch_add(1, Ordering::Relaxed);
    match function(context, &path) {
        Ok(_) => {
            success.fetch_add(1, Ordering::Relaxed);
            writeln!(stdout().lock(), "Processed {path}").unwrap()
        },
        Err(e) => writeln!(stderr().lock(), "Failed to process {path}: {e}").unwrap()
    }
}
