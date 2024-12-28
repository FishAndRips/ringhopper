use std::env::Args;
use std::path::Path;
use ringhopper::definitions::ScenarioType;
use ringhopper::map::{load_map_from_filesystem, MapTagTree};
use ringhopper::primitives::dynamic::DynamicTagDataArray;
use ringhopper::primitives::tag::ParseStrictness;
use crate::cli::{CommandLineParser, CommandLineValue, CommandLineValueType, Parameter};
use crate::util::bytes_to_mib;

pub fn info(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<map> [args]")
        .add_help()
        .add_custom_parameter(
            Parameter::new(
                "type",
                'T',
                "Type of info to get. Must be one of: general (default = general).",
                "<type>",
                Some(CommandLineValueType::String),
                1,
                Some(vec![CommandLineValue::String("general".to_string())]),
                false,
                false
            ))
        .set_required_extra_parameters(1)
        .parse(args)?;
    let map_path = Path::new(&parser.get_extra()[0]);
    let map = load_map_from_filesystem(map_path, ParseStrictness::Relaxed).map_err(|e| format!("Cannot load {map_path:?} as a cache file: {e}"))?;
    let info_type = parser.get_custom("type").expect("where is --type")[0].string();

    match info_type.to_ascii_lowercase().as_str() {
        "general" => general_info(map.as_ref()),
        info_type => {
            return Err(format!("Unknown info type {info_type}"))
        }
    }

    Ok(())
}

fn general_info(map: &dyn MapTagTree) {
    macro_rules! print_pair {
        ($name:expr, $value:expr) => {{
            println!(
                "{name:19}{value}",
                name=format!("{name}{colon}", name=$name, colon=if $name.is_empty() { "" } else { ":" }),
                value=$value)
            ;
        }};
    }

    let scenario_type = map.get_scenario_type();
    let engine = map.get_engine();
    print_pair!("Scenario name", map.get_name());
    print_pair!("Engine", engine.display_name);
    print_pair!("", format_args!("({})", engine.name));
    print_pair!("Build string", map.get_build_string());
    if let Some((header, actual)) = map.get_crc32() {
        if header == actual {
            let be_bytes: [u8; 4] = actual.to_be_bytes();
            let str: String = be_bytes
                .iter()
                .filter_map(|b| char::from_u32(*b as u32))
                .filter(|c| c.is_ascii() && !c.is_ascii_control())
                .collect();

            if str.len() == be_bytes.len() {
                print_pair!("CRC32", format_args!("0x{actual:08X} \"{str}\" (matches header)"));
            }
            else {
                print_pair!("CRC32", format_args!("0x{actual:08X} (matches header)"));
            }
        }
        else {
            print_pair!("CRC32", format_args!("0x{actual:08X} (mismatch; header=0x{header:08X})"));
        }
    }
    print_pair!("Map type", scenario_type);
    print_pair!("Tags", map.get_all_tags().len());

    if let Some(uncompressed) = map.get_uncompressed_size() {
        let limit = match scenario_type {
            ScenarioType::Multiplayer => engine.max_cache_file_size.multiplayer,
            ScenarioType::Singleplayer => engine.max_cache_file_size.singleplayer,
            ScenarioType::UserInterface => engine.max_cache_file_size.user_interface,
        };

        print_pair!("Uncompressed size", format_args!("{} / {}", bytes_to_mib(uncompressed), bytes_to_mib(limit as usize)));
    }

    if let Some(tag_space) = map.get_used_tag_space() {
        let limit = engine.max_tag_space as usize;
        print_pair!("Tag space used", format_args!("{} / {}", bytes_to_mib(tag_space), bytes_to_mib(limit)));

        if let Some(estimated) = map.get_estimated_max_tag_space() {
            if estimated != limit {
                print_pair!("Actual tag space", format_args!("{} (0x{estimated:08X})", bytes_to_mib(estimated)));
                if estimated > limit {
                    print_pair!("", "The map's tag space is HIGHER than expected!");
                }
                else if estimated == limit {
                    print_pair!("", "The map's tag space is exactly as expected!");
                }
                else {
                    print_pair!("", "The map's tag space is LOWER than expected!");
                }
            }
        }
        else if !engine.external_bsps {
            print_pair!("Actual tag space", "Cannot determine");
        }
    }
}
