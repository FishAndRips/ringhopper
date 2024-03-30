use std::env::Args;
use ringhopper::primitives::engine::Engine;
use ringhopper_engines::ALL_SUPPORTED_ENGINES;

pub fn list_engines(_args: Args, _description: &'static str) -> Result<(), String> {
    let mut buildable_engines = Vec::with_capacity(ALL_SUPPORTED_ENGINES.len());
    let mut read_only_engines = Vec::with_capacity(ALL_SUPPORTED_ENGINES.len());

    for i in ALL_SUPPORTED_ENGINES {
        if i.fallback {
            continue
        }

        if i.build_target {
            buildable_engines.push(i)
        }
        else {
            read_only_engines.push(i)
        }
    }

    let print_engine = |engine: &Engine| {
        println!("    {shorthand:20} {full_name}", shorthand= engine.name, full_name= engine.display_name)
    };

    println!("Read-only engines:");
    for i in read_only_engines {
        print_engine(i);
    }
    println!();
    println!("Targetable engines:");
    for i in buildable_engines {
        print_engine(i);
    }


    Ok(())
}
