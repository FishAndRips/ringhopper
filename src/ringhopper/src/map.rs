use std::collections::HashMap;
use std::ops::Range;
use definitions::{Bitmap, CacheFileHeaderPCDemo, CacheFileTag, CacheFileTagDataHeaderPC, Model, read_any_tag_from_map, Scenario, ScenarioStructureBSPCompiledHeader};
use crate::map::extract::*;
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::map::{DomainType, Map, ResourceMapType, Tag};
use primitives::primitive::{ID, TagGroup, TagPath};
use primitives::tag::PrimaryTagStructDyn;
use primitives::parse::TagData;
use crate::tag::object::downcast_base_object_mut;
use crate::tag::tree::{TagFilter, TagTree, TagTreeItem};

mod extract;

type SizeRange = Range<usize>;

#[derive(Clone)]
pub struct GearboxCacheFile {
    name: String,
    data: Vec<u8>,
    tag_data: SizeRange,
    vertex_data: SizeRange,
    triangle_data: SizeRange,
    bsp_data: Vec<BSPDomain>,
    base_memory_address: usize,
    tags: Vec<Tag>,
    ids: HashMap<TagPath, ID>,
    scenario_tag_data: Scenario,
    scenario_tag: ID,
    bitmaps: Vec<u8>,
    sounds: Vec<u8>
}

#[derive(Clone)]
struct BSPDomain {
    range: SizeRange,
    base_address: usize
}

impl GearboxCacheFile {
    pub fn new(data: Vec<u8>, bitmaps: Vec<u8>, sounds: Vec<u8>) -> RinghopperResult<Self> {
        let mut map = Self {
            name: String::new(),
            data,
            vertex_data: Default::default(),
            triangle_data: Default::default(),
            tag_data: Default::default(),
            bsp_data: Default::default(),
            base_memory_address: 0x4BF10000, // TODO: use engine definitions for this
            tags: Default::default(),
            scenario_tag: ID::null(),
            scenario_tag_data: Scenario::default(),
            ids: Default::default(),
            bitmaps,
            sounds
        };

        let header = CacheFileHeaderPCDemo::read_from_map(&map, 0, &DomainType::MapData)?;
        let tag_data_start = header.tag_data_offset as usize;
        let tag_data_end = tag_data_start.add_overflow_checked(header.tag_data_size as usize)?;
        let tag_data_range = tag_data_start..tag_data_end;
        if map.data.get(tag_data_range.clone()).is_none() {
            return Err(
                Error::MapParseFailure(format!("tag data region out of bounds 0x{tag_data_start:08X} - 0x{tag_data_start:08X}"))
            )
        }
        map.tag_data = tag_data_range;
        map.name = header.name.to_string();

        let tag_data_header = CacheFileTagDataHeaderPC::read_from_map(&map, map.base_memory_address, &DomainType::TagData)?;
        let model_data_start = tag_data_header.model_data_file_offset as usize;
        let model_data_size = tag_data_header.model_data_size as usize;
        let model_data_end = model_data_start.add_overflow_checked(model_data_size)?;
        let model_triangle_offset = tag_data_header.model_triangle_offset as usize;

        let model_data_range = model_data_start..model_data_end;
        if map.data.get(model_data_range.clone()).is_none() {
            return Err(
                Error::MapParseFailure(format!("model data region is out of bounds 0x{model_data_start:08X} - 0x{model_data_end:08X}"))
            )
        }

        if model_triangle_offset > model_data_size {
            return Err(
                Error::MapParseFailure(format!("model data triangle offset is out of bounds 0x{model_triangle_offset:08X} > 0x{model_data_size:08X}"))
            )
        }

        let vertex_end = model_data_start + model_triangle_offset;
        map.vertex_data = model_data_start..vertex_end;
        map.triangle_data = vertex_end..model_data_end;
        map.scenario_tag = tag_data_header.cache_file_tag_data_header.scenario_tag;

        let mut tag_address = tag_data_header.cache_file_tag_data_header.tag_array_address.address as usize;

        let tag_count = tag_data_header.cache_file_tag_data_header.tag_count as usize;
        if tag_count > u16::MAX as usize {
            return Err(
                Error::MapParseFailure(format!("maximum tag count exceeded (map claims to have {tag_count} tags"))
            )
        }

        let mut tags = Vec::with_capacity(tag_data_header.cache_file_tag_data_header.tag_count as usize);
        for t in 0..tag_count {
            let cached_tag = CacheFileTag::read_from_map(&map, tag_address, &DomainType::TagData)?;

            let expected_index = cached_tag.id.index();
            if expected_index != Some(t as u16) {
                return Err(Error::MapParseFailure(format!("tag #{t} has an invalid tag ID")))
            }

            let tag_path_address = cached_tag.path.address as usize;
            let path = map
                .get_c_string_at_address(tag_path_address, &DomainType::TagData)
                .ok_or_else(|| Error::MapParseFailure(format!("unable to get the tag path for tag #{t} due to a bad address 0x{tag_path_address:08X}")))?;

            let tag_path = TagPath::new(path, cached_tag.tag_group)
                .map_err(|e| Error::MapParseFailure(format!("unable to get the tag path for tag #{t} due to a parse failure: {e}")))?;

            if cached_tag.external != 0 {
                todo!("handle external indexed tags")
            }

            if tag_path.group() != TagGroup::_Unset {
                if map.ids.get(&tag_path).is_some() {
                    return Err(Error::MapParseFailure(format!("multiple instances of tag {tag_path} detected")))
                }
                map.ids.insert(tag_path.clone(), cached_tag.id);
            }

            let tag = Tag {
                tag_path,
                address: cached_tag.data.address as usize,
                domain: DomainType::TagData
            };

            tag_address = tag_address.add_overflow_checked(CacheFileTag::size())?;
            tags.push(tag);
        }
        map.tags = tags;

        // Get the scenario tag to get the BSP data
        let scenario = match map.get_tag_by_id(map.scenario_tag) {
            Some(n) => n,
            None => return Err(Error::MapParseFailure(format!("unable to get the scenario tag due to an invalid tag ID {}", map.scenario_tag)))
        };
        if scenario.tag_path.group() != TagGroup::Scenario {
            return Err(Error::MapParseFailure(format!("scenario tag is marked as a {} tag; likely protected/corrupted map", scenario.tag_path.group())))
        }

        // Now get BSPs from the scenario tag
        let scenario_tag = Scenario::read_from_map(&map, scenario.address, &scenario.domain)?;
        for bsp_index in 0..scenario_tag.structure_bsps.items.len() {
            let bsp = &scenario_tag.structure_bsps.items[bsp_index];
            let path = &bsp.structure_bsp;
            let start = bsp.bsp_start as usize;
            let range = start..start.add_overflow_checked(bsp.bsp_size as usize)?;

            if map.data.get(range.clone()).is_none() {
                return Err(Error::MapParseFailure(format!("BSP tag {path} has an invalid range")))
            }

            let bsp_base_address = bsp.bsp_address as usize;
            map.bsp_data.push(BSPDomain {
                range,
                base_address: bsp_base_address
            });

            let header = ScenarioStructureBSPCompiledHeader::read_from_map(
                &map,
                bsp_base_address,
                &DomainType::BSP(bsp_index)
            )?;
            let bsp_tag_base_address = header.pointer.address as usize;

            if let Some(n) = bsp.structure_bsp.path() {
                for t in &mut map.tags {
                    if &t.tag_path == n {
                        if matches!(t.domain, DomainType::BSP(_)) {
                            return Err(Error::MapParseFailure(format!("BSP tag {path} has ambiguous data")))
                        }

                        t.domain = DomainType::BSP(bsp_index);
                        t.address = bsp_tag_base_address;
                        break
                    }
                }
            }
        }

        for i in 0..tag_count {
            let t= &map.tags[i];
            match t.tag_path.group() {
                TagGroup::ScenarioStructureBSP => if !matches!(t.domain, DomainType::BSP(_)) {
                    return Err(Error::MapParseFailure(format!("BSP tag {} has no corresponding data in the scenario tag", t.tag_path)))
                }
                TagGroup::Scenario => if i != map.scenario_tag.index().unwrap() as usize {
                    return Err(Error::MapParseFailure(format!("Extraneous scenario tag {} in the map (map likely protected/corrupted)", t.tag_path)))
                }
                _ => ()
            }
        }

        // Cache the scenario tag for later usages
        map.scenario_tag_data = scenario_tag;

        // Done one more time to make sure everything's good
        debug_assert!(map.data.get(map.tag_data.clone()).is_some());
        debug_assert!(map.data.get(map.vertex_data.clone()).is_some());
        debug_assert!(map.data.get(map.triangle_data.clone()).is_some());
        debug_assert!(map.bsp_data.iter().all(|f| map.data.get(f.range.clone()).is_some()));

        Ok(map)
    }
}

fn extract_tag_from_map<M: Map>(
    map: &M,
    path: &TagPath,
    scenario_tag: &Scenario,

    bitmap_extraction_fn: fn(tag: &mut Bitmap, map: &M) -> RinghopperResult<()>,
    model_extraction_fn: fn(tag: &mut Model, map: &M) -> RinghopperResult<()>,


) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
    let mut tag = read_any_tag_from_map(path, map)?;

    match path.group() {
        TagGroup::ActorVariant => fix_actor_variant_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::Bitmap => bitmap_extraction_fn(tag.as_any_mut().downcast_mut().unwrap(), map)?,
        TagGroup::ContinuousDamageEffect => fix_continuous_damage_effect_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::DamageEffect => fix_damage_effect_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::GBXModel => fix_gbxmodel_tag(tag.as_any_mut().downcast_mut().unwrap(), map)?,
        TagGroup::Light => fix_light_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::ModelAnimations => fix_model_animations_tag(tag.as_any_mut().downcast_mut().unwrap())?,
        TagGroup::Model => { model_extraction_fn(tag.as_any_mut().downcast_mut().unwrap(), map)?; },
        TagGroup::PointPhysics => fix_point_physics_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::Projectile => fix_projectile_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::Scenario => fix_scenario_tag(tag.as_any_mut().downcast_mut().unwrap(), path.base_name())?,
        TagGroup::Sound => fix_sound_tag(tag.as_any_mut().downcast_mut().unwrap())?,
        TagGroup::Weapon => fix_weapon_tag(tag.as_any_mut().downcast_mut().unwrap(), path, &scenario_tag),
        _ => ()
    };

    if let Some(n) = downcast_base_object_mut(tag.as_mut()) {
        fix_object_tag(n)?;
    }

    Ok(tag)
}

impl Map for GearboxCacheFile {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn extract_tag(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        extract_tag_from_map(self, path, &self.scenario_tag_data, fix_bitmap_tag_normal, fix_model_tag_uncompressed)
    }

    fn get_domain(&self, domain: &DomainType) -> Option<(&[u8], usize)> {
        match domain {
            DomainType::MapData => Some((self.data.as_slice(), 0)),

            // OK because these are checked on load
            DomainType::TagData => Some((unsafe { self.data.get_unchecked(self.tag_data.clone()) }, self.base_memory_address)),
            DomainType::ModelVertexData => Some((unsafe { self.data.get_unchecked(self.vertex_data.clone()) }, 0)),
            DomainType::ModelTriangleData => Some((unsafe { self.data.get_unchecked(self.triangle_data.clone()) }, 0)),
            DomainType::BSP(b) => {
                let bsp = self.bsp_data.get(*b)?;
                Some((unsafe { self.data.get_unchecked(bsp.range.clone()) }, bsp.base_address))
            }

            DomainType::ResourceMapFile(ResourceMapType::Bitmaps) => Some((self.bitmaps.as_slice(), 0)),
            DomainType::ResourceMapFile(ResourceMapType::Sounds) => Some((self.sounds.as_slice(), 0)),

            _ => None
        }
    }

    fn get_tag_by_id(&self, id: ID) -> Option<&Tag> {
        self.tags.get(id.index()? as usize)
    }

    fn get_tag(&self, path: &TagPath) -> Option<&Tag> {
        self.get_tag_by_id(*self.ids.get(path)?)
    }

    fn get_scenario_tag(&self) -> &Tag {
        self.get_tag_by_id(self.scenario_tag).unwrap()
    }

    fn get_all_tags(&self) -> Vec<TagPath> {
        self.ids.keys().map(|key| key.to_owned()).collect()
    }
}

impl<M: Map> TagTree for M {
    fn open_tag_copy(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        self.extract_tag(path)
    }

    fn files_in_path(&self, _path: &str) -> Option<Vec<TagTreeItem>> {
        unimplemented!()
    }

    fn write_tag(&mut self, _path: &TagPath, _tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<bool> {
        unimplemented!()
    }

    fn contains(&self, path: &TagPath) -> bool {
        self.get_tag(path).is_some()
    }

    fn get_all_tags_with_filter(&self, filter: Option<&TagFilter>) -> Vec<TagPath> where Self: Sized {
        let all_tags = self.get_all_tags();
        if let Some(n) = filter {
            all_tags.into_iter().filter(|t| n.passes(t)).collect()
        }
        else {
            all_tags
        }
    }
}
