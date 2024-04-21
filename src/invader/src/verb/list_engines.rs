use std::env::Args;
use ringhopper::primitives::engine::Engine;
use ringhopper_engines::{EngineCompressionType, ALL_SUPPORTED_ENGINES};

pub fn list_engines(mut args: Args, _description: &'static str) -> Result<(), String> {
    if let Some(n) = args.next() {
        match ALL_SUPPORTED_ENGINES.binary_search_by(|e| e.name.cmp(n.as_str())) {
            Ok(n) => {
                print_info_for_engine(&ALL_SUPPORTED_ENGINES[n]);
                return Ok(())
            },
            Err(_) => return Err(format!("{n} is not a valid engine target"))
        }
    }

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
        println!("    {shorthand:20} {full_name}", shorthand = engine.name, full_name = engine.display_name)
    };

    println!("Engine targets:");
    for i in buildable_engines {
        print_engine(i);
    }
    println!();
    println!("Read-only engines (cannot be used with --engine):");
    for i in read_only_engines {
        print_engine(i);
    }
    println!();
    println!("Use list-engines <engine> for technical information about an engine.");

    Ok(())
}

fn print_info_for_engine(engine: &Engine) {
    let bytes_to_mib = |bytes| -> f64 {
        bytes as f64 / 1024.0 / 1024.0
    };

    println!("Info for engine `{}`", engine.name);
    println!();
    println!("Target name:         {}", engine.name);
    println!("Full name:           {}", engine.display_name);
    println!();
    println!("Engine version:      {}", engine.version.unwrap_or("<none>"));
    println!("Cache version:       {}", engine.cache_file_version);
    println!("Build version:       {}", engine.build.map(|e| e.string).unwrap_or("<none>"));
    println!();
    println!("Tag data address:    0x{:08X}{}", engine.base_memory_address.address, if engine.base_memory_address.inferred { " (inferred)" } else { "" });
    println!("Max tag space:       0x{:08X} ({} MiB)", engine.max_tag_space, bytes_to_mib(engine.max_tag_space));
    println!("BSP in tag space:    {}", if engine.external_bsps { "no" } else { "yes" } );
    println!("Models in tag space: {}", if engine.external_models { "no" } else { "yes" } );
    println!();
    println!("Vertex type:         {}", if engine.compressed_models { "compressed" } else { "uncompressed" });
    println!("Max script nodes:    {}", engine.max_script_nodes);
    println!("Texture sizes:       {}", if engine.bitmap_options.texture_dimension_must_modulo_block_size { "must modulo block size" } else { "any" });
    println!();
    println!("Cache compression:   {}", match engine.compression_type {
        EngineCompressionType::Uncompressed => "no",
        EngineCompressionType::Deflate => "yes (deflate)"
    });
    println!("Maximum cache file sizes:");
    println!(" - singleplayer:     0x{:08X} ({} MiB)", engine.max_cache_file_size.singleplayer, bytes_to_mib(engine.max_cache_file_size.singleplayer));
    println!(" - multiplayer:      0x{:08X} ({} MiB)", engine.max_cache_file_size.multiplayer, bytes_to_mib(engine.max_cache_file_size.multiplayer));
    println!(" - user interface    0x{:08X} ({} MiB)", engine.max_cache_file_size.user_interface, bytes_to_mib(engine.max_cache_file_size.user_interface));

    if engine.fallback {
        println!();
        println!("NOTE: This is a fallback engine and may not represent all engines like this.");
    }
    else if !engine.build_target {
        println!();
        println!("NOTE: This is a read-only engine kept for cache file compatibility and may not be 100% accurate.");
    }
}
