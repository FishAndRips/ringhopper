use std::env::Args;

extern crate ringhopper_engines;


pub fn list_engines(_args: Args, _description: &'static str) -> Result<(), String> {
    println!("Available engine targets:");
    for i in ringhopper_engines::ALL_SUPPORTED_ENGINES {
        if !i.build_target {
            continue
        }
        println!("    {shorthand:20} {full_name}", shorthand=i.name, full_name=i.display_name)
    }
    Ok(())
}
