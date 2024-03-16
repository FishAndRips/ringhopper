extern crate ringhopper_definitions;

use ringhopper_definitions::{CacheParser, Engine, load_all_definitions};
use std::fmt::Write;
use proc_macro::TokenStream;

#[proc_macro]
pub fn generate_ringhopper_engines(_: TokenStream) -> TokenStream {
    let definitions = load_all_definitions();
    let mut engine_code = "pub const ALL_SUPPORTED_ENGINES: &'static [Engine] = &[".to_string();

    let mut sorted: Vec<&Engine> = Vec::with_capacity(definitions.engines.len());
    for (_name, engine) in &definitions.engines {
        let result = sorted.binary_search_by(|e| e.name.cmp(&engine.name)).unwrap_err();
        sorted.insert(result, engine);
    }

    for engine in &sorted {
        let unwrap_str = |str: &Option<String>| match str { Some(n) => format!("Some(\"{n}\")"), None => "None".to_owned() };
        let make_string_list = |str: &[String]| {
            let mut value = String::new();
            for i in str {
                value += &format!("\"{val}\",", val=i.replace("\\", "\\\\").replace("\"", "\\\""));
            }
            format!("&[{value}]")
        };

        let name = &engine.name;
        let display_name = &engine.display_name;
        let version = unwrap_str(&engine.version);
        let build = match &engine.build {
            Some(n) => format!("Some(EngineBuild {{ string: \"{}\", enforced: {}, fallback: {} }})", n.string, n.enforced, make_string_list(&n.fallback).as_str()),
            None => "None".to_string()
        };
        let build_target = engine.build_target;
        let cache_file_version = engine.cache_file_version;
        let max_script_nodes = engine.max_script_nodes;
        let max_tag_space = engine.max_tag_space;
        let external_bsps = engine.external_bsps;
        let resource_maps = if let Some(n) = &engine.resource_maps {
            let externally_indexed_tags = n.externally_indexed_tags;
            let loc = n.loc;
            format!("Some(EngineSupportedResourceMaps {{ loc: {loc}, externally_indexed_tags: {externally_indexed_tags} }})")
        }
        else {
            "None".to_owned()
        };
        let max_cache_file_size = format!("EngineCacheFileSize {{
            user_interface: {user_interface},
            singleplayer: {singleplayer},
            multiplayer: {multiplayer},
        }}", user_interface=engine.max_cache_file_size.user_interface, singleplayer=engine.max_cache_file_size.singleplayer, multiplayer=engine.max_cache_file_size.multiplayer);
        let base_memory_address = format!("EngineBaseMemoryAddress {{
            address: {address},
            inferred: {inferred}
        }}", address=engine.base_memory_address.address, inferred=engine.base_memory_address.inferred);
        let required_tags = format!("EngineRequiredTags {{
            all: {all},
            user_interface: {user_interface},
            singleplayer: {singleplayer},
            multiplayer: {multiplayer}
        }}",
                                    all=make_string_list(engine.required_tags.all.as_slice()),
                                    user_interface=make_string_list(engine.required_tags.user_interface.as_slice()),
                                    singleplayer=make_string_list(engine.required_tags.singleplayer.as_slice()),
                                    multiplayer=make_string_list(engine.required_tags.multiplayer.as_slice())
        );
        let cache_default = engine.cache_default;
        let cache_parser = match engine.cache_parser {
            CacheParser::PC => "PC",
            CacheParser::Xbox => "Xbox"
        };

        write!(&mut engine_code, "Engine {{
            name: \"{name}\",
            display_name: \"{display_name}\",
            version: {version},
            build: {build},
            build_target: {build_target},
            cache_default: {cache_default},
            cache_file_version: {cache_file_version},
            external_bsps: {external_bsps},
            max_script_nodes: {max_script_nodes},
            max_tag_space: {max_tag_space},
            max_cache_file_size: {max_cache_file_size},
            base_memory_address: {base_memory_address},
            resource_maps: {resource_maps},
            cache_parser: EngineCacheParser::{cache_parser},
            required_tags: {required_tags},
        }},").unwrap();
    }

    writeln!(&mut engine_code, "];").unwrap();

    engine_code.parse().unwrap()
}
