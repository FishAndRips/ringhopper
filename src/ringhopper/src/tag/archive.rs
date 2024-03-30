use std::io::{Cursor, Write};
use zip::CompressionMethod;
use zip::write::FileOptions;
use primitives::engine::Engine;
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::TagPath;
use crate::tag::dependency::{recursively_get_dependencies_for_map, recursively_get_dependencies_for_tag};
use crate::tag::tree::TagTree;

#[derive(Copy, Clone)]
pub struct ZstandardCompressionLevel {
    level: i32
}

impl ZstandardCompressionLevel {
    pub fn new(level: i32) -> RinghopperResult<ZstandardCompressionLevel> {
        if (0..=22).contains(&level) {
            Ok(Self { level })
        }
        else {
            Err(Error::Other(format!("invalid zstandard level {level} - must be between 0 and 22")))
        }
    }

    pub fn level(self) -> i32 {
        self.level
    }
}

pub fn archive_tag_set_to_zip<T: TagTree, I: IntoIterator<Item = TagPath>>(tag_set: I, tag_tree: &T, zstandard_compression_level: ZstandardCompressionLevel) -> RinghopperResult<Vec<u8>> {
    macro_rules! wrap_zip_result {
        ($res: expr) => {
            ($res).map_err(|e| Error::Other(format!("zip error: {e:?}")))
        };
    }

    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Zstd)
        .compression_level(Some(zstandard_compression_level.level()))
        .large_file(true);

    for i in tag_set.into_iter() {
        wrap_zip_result!(archive.start_file(i.to_zip_path(), options))?;

        let tag = tag_tree.open_tag_shared(&i)?;
        let lock = tag.lock().unwrap();

        wrap_zip_result!(archive.write(lock.to_tag_file()?.as_slice()))?;
    }

    Ok(wrap_zip_result!(archive.finish())?.into_inner())
}

pub fn archive_map_to_zip<T: TagTree>(scenario: &TagPath, tag_tree: &T, engine: &Engine, zstandard_compression_level: ZstandardCompressionLevel) -> RinghopperResult<Vec<u8>> {
    let mut all_dependencies = recursively_get_dependencies_for_map(scenario, tag_tree, engine)?;
    all_dependencies.insert(scenario.to_owned());
    archive_tag_set_to_zip(all_dependencies, tag_tree, zstandard_compression_level)
}

pub fn archive_tag_to_zip<T: TagTree>(tag: &TagPath, tag_tree: &T, zstandard_compression_level: ZstandardCompressionLevel) -> RinghopperResult<Vec<u8>> {
    let mut all_dependencies = recursively_get_dependencies_for_tag(tag, tag_tree)?;
    all_dependencies.insert(tag.to_owned());
    archive_tag_set_to_zip(all_dependencies, tag_tree, zstandard_compression_level)
}
