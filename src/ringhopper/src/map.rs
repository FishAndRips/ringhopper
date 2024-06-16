use std::fs::File;
use std::io::Read;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use flate2::FlushDecompress;

use definitions::{read_any_tag_from_map, Scenario, ScenarioType};
use primitives::engine::{Engine, EngineCacheParser, EngineCompressionType};
use primitives::error::{Error, RinghopperResult};
use primitives::map::Map;
use primitives::primitive::{TagGroup, TagPath};
use primitives::tag::{ParseStrictness, PrimaryTagStructDyn};

use crate::map::extract::*;
use crate::map::gearbox::GearboxCacheFile;
use crate::map::header::ParsedCacheFileHeader;
use crate::map::xbox::XboxCacheFile;
use crate::tag::object::downcast_base_object_mut;
use crate::tag::tree::{TagFilter, TagTree, TagTreeItem, TreeType};

mod extract;
pub mod resource;

pub mod header;
pub mod gearbox;
pub mod xbox;
mod util;

type SizeRange = Range<usize>;

#[derive(Clone)]
struct BSPDomain {
    path: Option<TagPath>,
    range: SizeRange,
    base_address: usize,
    tag_address: usize
}

fn extract_tag_from_map<M: Map>(
    map: &M,
    path: &TagPath,
    scenario_tag: &Scenario
) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
    let group = path.group();
    let mut tag = read_any_tag_from_map(path, map)?;

    match group {
        TagGroup::ActorVariant => fix_actor_variant_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::Bitmap => fix_bitmap_tag(tag.as_any_mut().downcast_mut().unwrap(), map)?,
        TagGroup::ContinuousDamageEffect => fix_continuous_damage_effect_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::DamageEffect => fix_damage_effect_tag(tag.as_any_mut().downcast_mut().unwrap(), path, &scenario_tag),
        TagGroup::GBXModel => fix_gbxmodel_tag(tag.as_any_mut().downcast_mut().unwrap(), map)?,
        TagGroup::Light => fix_light_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::ModelAnimations => fix_model_animations_tag(tag.as_any_mut().downcast_mut().unwrap())?,
        TagGroup::Model => fix_model_tag(tag.as_any_mut().downcast_mut().unwrap(), map)?,
        TagGroup::PointPhysics => fix_point_physics_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::Projectile => fix_projectile_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::Scenario => fix_scenario_tag(tag.as_any_mut().downcast_mut().unwrap(), path.base_name())?,
        TagGroup::ScenarioStructureBSP => fix_scenario_structure_bsp_tag(tag.as_any_mut().downcast_mut().unwrap(), scenario_tag, map)?,
        TagGroup::Sound => fix_sound_tag(tag.as_any_mut().downcast_mut().unwrap())?,
        TagGroup::UnicodeStringList => fix_unicode_string_list_tag(tag.as_any_mut().downcast_mut().unwrap(), map)?,
        TagGroup::Weapon => fix_weapon_tag(tag.as_any_mut().downcast_mut().unwrap(), path, &scenario_tag),
        _ => ()
    };

    if let Some(n) = downcast_base_object_mut(tag.as_mut()) {
        fix_object_tag(n)?;
    }

    Ok(tag)
}

macro_rules! tag_tree_impl {
    () => {
        fn open_tag_copy(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
            self.extract_tag(path)
        }

        fn files_in_path(&self, _path: &str) -> Option<Vec<TagTreeItem>> {
            todo!("files_in_path not yet implemented for MapTagTree")
        }

        fn write_tag(&mut self, _path: &TagPath, _tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<bool> {
            unimplemented!("write_tag not implemented for MapTagTree")
        }

        fn is_read_only(&self) -> bool {
            true
        }

        fn contains(&self, path: &TagPath) -> bool {
            self.get_tag(path).is_some()
        }

        fn root(&self) -> TagTreeItem {
            // TagTreeItem::new(TagTreeItemType::Directory, Cow::default(), None, self)
            todo!()
        }

        fn get_all_tags_with_filter(&self, filter: Option<&TagFilter>) -> Vec<TagPath> {
            let all_tags = self.get_all_tags();
            if let Some(n) = filter {
                all_tags.into_iter().filter(|t| n.passes(t)).collect()
            }
            else {
                all_tags
            }
        }

        fn tree_type(&self) -> TreeType {
            TreeType::CacheFile
        }
    };
}

pub trait MapTagTree: Map {
    /// Get the scenario type for the map.
    fn get_scenario_type(&self) -> ScenarioType;
}
impl<M: MapTagTree> TagTree for M {
    tag_tree_impl!();
}

impl TagTree for Arc<dyn MapTagTree + Send + Sync> {
    tag_tree_impl!();
}

const CACHE_FILE_HEADER_LEN: usize = 0x800;

macro_rules! make_map_load_fn {
    ($name:tt, $principal_type:tt, $doc:tt) => {
        #[doc=$doc]
        pub fn $name<P: AsRef<Path>>(path: P, strictness: ParseStrictness) -> RinghopperResult<Arc<dyn $principal_type + Send + Sync>> {
            let io = |e: std::io::Error| -> Error {
                Error::FailedToReadFile(path.as_ref().to_path_buf(), e)
            };

            let mut file = File::open(&path).map_err(io)?;
            let mut map = vec![0u8; CACHE_FILE_HEADER_LEN];
            file.read_exact(&mut map).map_err(io)?;

            let header = ParsedCacheFileHeader::read_from_map_data(&map)?;
            let engine = header.match_engine().ok_or_else(|| Error::MapParseFailure(format!("Can't parse {} as a cache file due to an unknown engine", path.as_ref().to_string_lossy())))?;

            file.read_to_end(&mut map).map_err(io)?;
            drop(file);

            let map = decompress_map_data(map, &header, engine)?;

            match engine.cache_parser {
                EngineCacheParser::PC => {
                    let resource_data_needed = engine.resource_maps.unwrap();

                    let bitmaps;
                    let sounds;
                    let loc;

                    if let Some(parent) = path.as_ref().parent() {
                        bitmaps = std::fs::read(parent.join("bitmaps.map")).unwrap_or(Vec::new());
                        sounds = std::fs::read(parent.join("sounds.map")).unwrap_or(Vec::new());
                        loc = if resource_data_needed.loc { std::fs::read(parent.join("loc.map")).unwrap_or(Vec::new()) } else { Vec::new() };
                    }
                    else {
                        bitmaps = Vec::new();
                        sounds = Vec::new();
                        loc = Vec::new();
                    }

                    Ok(Arc::new(GearboxCacheFile::new(map, bitmaps, sounds, loc, strictness)?))
                },
                EngineCacheParser::Xbox => Ok(Arc::new(XboxCacheFile::new(map, strictness)?))
            }
        }

    };
}

make_map_load_fn!(load_map_from_filesystem, MapTagTree, "Load the map from the filesystem as a map.");
make_map_load_fn!(load_map_from_filesystem_as_tag_tree, TagTree, "Load the map from the filesystem as a tag tree.");

fn decompress_map_data(data: Vec<u8>, header: &ParsedCacheFileHeader, engine: &Engine) -> RinghopperResult<Vec<u8>> {
    if engine.compression_type == EngineCompressionType::Uncompressed {
        return Ok(data)
    }

    let compressed_start = CACHE_FILE_HEADER_LEN;
    let compressed_end = data.len()
        .checked_sub(header.compression_padding)
        .ok_or_else(|| Error::MapParseFailure("decompression failed: bad padding size in header - larger than cache file".to_owned()))?;
    if compressed_end < compressed_start {
        return Err(Error::MapParseFailure("decompression failed: bad padding size in header - larger than uncompressed data".to_owned()))
    }

    let mut result = Vec::with_capacity(header.decompressed_size);
    result.extend_from_slice(&data[..compressed_start]);
    result.resize(header.decompressed_size, 0);

    let compressed_data = &data[compressed_start..compressed_end];
    match engine.compression_type {
        EngineCompressionType::Uncompressed => unreachable!(),
        EngineCompressionType::Deflate => {
            let mut decompressor = flate2::Decompress::new(true);
            decompressor
                .decompress(compressed_data, &mut result[compressed_start..], FlushDecompress::Finish)
                .map_err(|e| Error::MapParseFailure(format!("decompression failed: flate2 error: {e}")))?;
            Ok(result)
        }
    }
}
