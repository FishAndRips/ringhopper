use std::collections::HashMap;
use std::path::PathBuf;
use definitions::Bitmap;
use primitives::error::RinghopperResult;
use primitives::primitive::{TagGroup, TagPath};
use primitives::tag::PrimaryTagStructDyn;
use crate::tag::bitmap::extract_compressed_color_plate_data;

pub type RecoverFunction = fn(tag_path: &TagPath, tag_data: &Box<dyn PrimaryTagStructDyn>) -> RinghopperResult<Option<HashMap<PathBuf, Vec<u8>>>>;

pub fn get_recover_function(group: TagGroup) -> Option<RecoverFunction> {
    match group {
        TagGroup::Bitmap => Some(recover_bitmap),
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
