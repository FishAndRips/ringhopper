#![allow(mismatched_lifetime_syntaxes)]

#[cfg(target_os = "windows")]
extern crate windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
extern crate libc;

use std::process::ExitCode;
use util::make_stdout_logger;

mod cli;
mod verb;
mod util;
mod threading;

extern crate ringhopper;
extern crate ringhopper_engines;

fn main() -> ExitCode {
    let mut args= std::env::args();
    let application_path = args.next().expect("should have application path");

    args.next()
        .and_then(|mut verb_name| {
            verb_name.make_ascii_lowercase();
            let found_verb = verb::get_verb(&verb_name);
            if found_verb.is_none() {
                make_stdout_logger().error_fmt_ln(format_args!("Error: No such verb `{verb_name}`!"));
            }
            found_verb
        })
        .map(|v| match (v.function)(args, v.description) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                make_stdout_logger().error_fmt_ln(format_args!("Error executing {}: {e}", v.name));
                ExitCode::FAILURE
            }
        })
        .unwrap_or_else(|| show_arguments(&application_path))
}

fn show_arguments(application_path: &str) -> ExitCode {
    println!("Usage: {application_path} <verb> [arguments...]");
    println!();
    println!("Available verbs:");
    for c in verb::ALL_VERBS {
        println!("    {:20} {}", c.name, c.description);
    }
    println!();
    println!("Use {application_path} --help to view help information for a verb.");

    ExitCode::FAILURE
}
