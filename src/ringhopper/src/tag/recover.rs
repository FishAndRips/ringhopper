use std::collections::HashMap;
use std::path::PathBuf;
use definitions::*;
use primitives::error::Error::InvalidTagData;
use primitives::error::RinghopperResult;
use primitives::primitive::{TagGroup, TagPath};
use primitives::tag::PrimaryTagStructDyn;
use crate::tag::bitmap::extract_compressed_color_plate_data;
use crate::tag::unicode_string_list::UnicodeStringListFunctions;

pub type RecoverFunction = fn(tag_path: &TagPath, tag_data: &Box<dyn PrimaryTagStructDyn>) -> RinghopperResult<Option<HashMap<PathBuf, Vec<u8>>>>;

pub fn get_recover_function(group: TagGroup) -> Option<RecoverFunction> {
    match group {
        TagGroup::Bitmap => Some(recover_bitmap),
        TagGroup::Scenario => Some(recover_scenario_scripts),
        TagGroup::UnicodeStringList => Some(recover_unicode_string_lists),
        _ => None
    }
}

fn recover_bitmap(tag_path: &TagPath, tag_data: &Box<dyn PrimaryTagStructDyn>) -> RinghopperResult<Option<HashMap<PathBuf, Vec<u8>>>> {
    let bitmap = tag_data.as_any().downcast_ref::<Bitmap>().unwrap();
    let color_plate_data = match extract_compressed_color_plate_data(bitmap)? {
        Some(n) => n,
        None => return Ok(None)
    };

    let result = PathBuf::from(tag_path.to_native_path()).with_extension("tif");
    let mut fs = HashMap::new();
    fs.insert(result, color_plate_data.to_tiff());

    Ok(Some(fs))
}

fn recover_unicode_string_lists(tag_path: &TagPath, tag_data: &Box<dyn PrimaryTagStructDyn>) -> RinghopperResult<Option<HashMap<PathBuf, Vec<u8>>>> {
    let unicode_string_list: &UnicodeStringList = tag_data.as_any().downcast_ref().unwrap();
    let data = unicode_string_list.as_text_data().map_err(|e| InvalidTagData(format!("{e:?}")))?;
    let result = PathBuf::from(tag_path.to_native_path()).with_extension("txt");
    let mut fs = HashMap::new();
    fs.insert(result, data);
    Ok(Some(fs))
}

fn recover_scenario_scripts(tag_path: &TagPath, tag_data: &Box<dyn PrimaryTagStructDyn>) -> RinghopperResult<Option<HashMap<PathBuf, Vec<u8>>>> {
    let scenario: &Scenario = tag_data.as_any().downcast_ref().unwrap();
    if scenario.source_files.items.is_empty() {
        return Ok(None)
    }

    // TODO: Verify that there are no duplicate script source files, and that all source files have lowercase names.

    let base_scripts_dir = PathBuf::from(tag_path.to_native_path()).parent().unwrap().join("scripts");
    let mut fs = HashMap::new();
    for i in &scenario.source_files.items {
        let path = match i.name.as_str() {
            "global_scripts" => "global_scripts.hsc".into(),
            name => base_scripts_dir.clone().join(format!("{name}.hsc"))
        };

        // Trim to null terminator if present
        let data = &i.source.bytes;
        let end = data.iter().position(|p| *p == 0).unwrap_or(data.len());

        // Finally add the script
        let data = data[..end].to_owned();
        fs.insert(path, data);
    }

    Ok(Some(fs))
}
