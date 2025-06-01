use std::collections::HashSet;
use std::io::Cursor;
use primitives::engine::Engine;
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::TagPath;
use sevenz_rust2::lzma::LZMA2Options;
use sevenz_rust2::{SevenZWriter, SevenZArchiveEntry, SevenZMethodConfiguration, MethodOptions, SevenZMethod};
use crate::tag::dependency::{recursively_get_dependencies_for_map, recursively_get_dependencies_for_tag};
use crate::tag::tree::TagTree;

#[derive(Copy, Clone)]
pub struct LZMACompressionLevel {
    level: u32
}

impl LZMACompressionLevel {
    pub fn new(level: u32) -> RinghopperResult<LZMACompressionLevel> {
        if (0..=9).contains(&level) {
            Ok(Self { level })
        }
        else {
            Err(Error::Other(format!("invalid LZMA level {level} - must be between 0 and 9")))
        }
    }

    pub fn level(self) -> u32 {
        self.level
    }
}

pub fn archive_tag_set_to_zip<T: TagTree, I: IntoIterator<Item = TagPath>>(tag_set: I, tag_tree: &T, compression_level: LZMACompressionLevel) -> RinghopperResult<Vec<u8>> {
    macro_rules! unwrap_7z {
        ($res: expr) => {
            ($res).map_err(|e| Error::Other(format!("7zip error: {e}")))?
        };
    }

    let mut archive = unwrap_7z!(SevenZWriter::new(Cursor::new(Vec::new())));
    archive.set_content_methods(vec![SevenZMethodConfiguration { method: SevenZMethod::LZMA2, options: Some(MethodOptions::LZMA2(LZMA2Options::with_preset(compression_level.level))) }]);

    // Keep groups together, and then sort by path.
    let tags_to_filter_paths: Vec<TagPath> = tag_set.into_iter().collect();
    let mut tags_to_filter_indices: Vec<usize> = (0..tags_to_filter_paths.len()).collect();
    tags_to_filter_indices.sort_by(|a, b| {
        let a = &tags_to_filter_paths[*a];
        let b = &tags_to_filter_paths[*b];

        if a.group() != b.group() {
            a.group().cmp(&b.group())
        }
        else {
            a.cmp(b)
        }
    });

    for i in tags_to_filter_indices.into_iter().map(|i| &tags_to_filter_paths[i]) {
        let mut entry = SevenZArchiveEntry::new();
        entry.name = i.to_zip_path();

        let tag = tag_tree.open_tag_shared(&i)?;
        let lock = tag.lock().unwrap();

        unwrap_7z!(archive.push_archive_entry(entry, Some(lock.to_tag_file()?.as_slice())));
    }

    Ok(unwrap_7z!(archive.finish()).into_inner())
}

pub fn archive_map_to_7zip<T: TagTree>(scenario: &TagPath, tag_tree: &T, engine: &Engine, compression_level: LZMACompressionLevel) -> RinghopperResult<Vec<u8>> {
    let all_dependencies = recursively_get_dependencies_for_map(scenario, tag_tree, engine)?;
    archive_tag_set_to_zip(all_dependencies, tag_tree, compression_level)
}

pub fn archive_tag_to_7zip<T: TagTree>(tag: &TagPath, tag_tree: &T, compression_level: LZMACompressionLevel) -> RinghopperResult<Vec<u8>> {
    let mut all_dependencies: HashSet<TagPath> = recursively_get_dependencies_for_tag(tag, tag_tree, false)?.into_values().flatten().collect();
    all_dependencies.insert(tag.to_owned());
    archive_tag_set_to_zip(all_dependencies, tag_tree, compression_level)
}
