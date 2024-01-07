use std::env::Args;

pub fn version(_: Args, _: &'static str) -> Result<(), String> {
    println!();
    println!("================================================================================");
    println!();
    println!("  Invader & Ringhopper version: {}", env!("CARGO_PKG_VERSION"));
    println!("  Copyright (C) 2024 Snowy Mouse");
    println!();
    println!("  This software is licensed under version 3 of the GNU General Public License");
    println!("  as published by the Free Software Foundation.");
    println!();
    println!("  For more information, see https://www.gnu.org/licenses/gpl-3.0.en.html");
    println!();
    println!("================================================================================");
    println!();
    Ok(())
}
