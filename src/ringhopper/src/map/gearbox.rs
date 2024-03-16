use std::collections::HashMap;
use definitions::{CacheFileTag, CacheFileTagDataHeaderPC, Scenario, ScenarioType, Sound, SoundPitchRange};
use primitives::byteorder::LittleEndian;
use primitives::engine::Engine;
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::map::{DomainType, Map, ResourceMapType, Tag};
use primitives::parse::{SimpleTagData, TagData};
use primitives::primitive::{Address, ID, ReflexiveC, TagGroup, TagPath};
use primitives::tag::PrimaryTagStructDyn;
use crate::map::{BSPDomain, extract_tag_from_map, MapTagTree, SizeRange};
use crate::map::extract::{fix_bitmap_tag_normal, fix_model_tag_uncompressed};
use crate::map::resource::ResourceMap;

pub struct GearboxCacheFile {
    name: String,
    engine: &'static Engine,
    data: Vec<u8>,
    tag_data: SizeRange,
    vertex_data: SizeRange,
    triangle_data: SizeRange,
    bsp_data: Vec<BSPDomain>,
    base_memory_address: usize,
    tags: Vec<Option<Tag>>,
    ids: HashMap<TagPath, ID>,
    scenario_tag: ID,
    merged_sound_resources: HashMap<DomainType, Vec<u8>>, // for Halo Custom Edition
    bitmaps: Option<ResourceMap>,
    sounds: Option<ResourceMap>,
    loc: Option<ResourceMap>,
    scenario_tag_data: Scenario
}

impl GearboxCacheFile {
    pub fn new(data: Vec<u8>, bitmaps: Vec<u8>, sounds: Vec<u8>, loc: Vec<u8>) -> RinghopperResult<Self> {
        let (header, engine, tag_data_range) = super::util::get_tag_data_details(&data)?;

        let mut map = Self {
            name: String::new(),
            data,
            engine,
            vertex_data: Default::default(),
            triangle_data: Default::default(),
            tag_data: Default::default(),
            bsp_data: Default::default(),
            base_memory_address: engine.base_memory_address.address as usize,
            tags: Default::default(),
            scenario_tag: ID::null(),
            merged_sound_resources: HashMap::new(),
            ids: Default::default(),
            bitmaps: if bitmaps.is_empty() { None } else { Some(ResourceMap::from_data(bitmaps)?) },
            sounds: if sounds.is_empty() { None } else { Some(ResourceMap::from_data(sounds)?) },
            loc: if loc.is_empty() { None } else { Some(ResourceMap::from_data(loc)?) },
            scenario_tag_data: Scenario::default()
        };

        map.tag_data = tag_data_range;
        map.name = header.name.to_string();

        let tag_data_header = CacheFileTagDataHeaderPC::read_from_map(&map, map.base_memory_address, &DomainType::TagData)?;
        let tag_address: usize = tag_data_header.cache_file_tag_data_header.tag_array_address.into();
        let tag_count = tag_data_header.cache_file_tag_data_header.tag_count as usize;
        if engine.base_memory_address.inferred {
            map.base_memory_address = tag_address.sub_overflow_checked(CacheFileTagDataHeaderPC::simple_size())?;
        }

        let (mut tags, cached_tags, ids) = super::util::get_all_tags(&mut map, tag_address, tag_count)?;
        map.handle_external_tags(engine, tag_count, &mut tags, &cached_tags)?;
        map.load_model_data(&tag_data_header)?;
        map.ids = ids;
        map.tags = tags;

        // Get the scenario tag to get the BSP data
        let (scenario_tag_data, bsps) = super::util::load_scenario_info(&map, &map.tags)?;
        map.bsp_data = bsps;
        map.scenario_tag_data = scenario_tag_data;
        super::util::fixup_bsp_addresses(&mut map.tags, &map.bsp_data, &map.ids)?;

        // Done one more time to make sure everything's good
        debug_assert!(map.data.get(map.tag_data.clone()).is_some());
        debug_assert!(map.data.get(map.vertex_data.clone()).is_some());
        debug_assert!(map.data.get(map.triangle_data.clone()).is_some());
        debug_assert!(map.bsp_data.iter().all(|f| map.data.get(f.range.clone()).is_some()));

        Ok(map)
    }

    fn load_model_data(&mut self, tag_data_header: &CacheFileTagDataHeaderPC) -> RinghopperResult<()> {
        let model_data_start = tag_data_header.model_data_file_offset as usize;
        let model_data_size = tag_data_header.model_data_size as usize;
        let model_data_end = model_data_start.add_overflow_checked(model_data_size)?;
        let model_triangle_offset = tag_data_header.model_triangle_offset as usize;

        let model_data_range = model_data_start..model_data_end;
        if self.data.get(model_data_range.clone()).is_none() {
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
        self.vertex_data = model_data_start..vertex_end;
        self.triangle_data = vertex_end..model_data_end;
        self.scenario_tag = tag_data_header.cache_file_tag_data_header.scenario_tag;

        Ok(())
    }

    fn handle_external_tags(&mut self, engine: &Engine, tag_count: usize, tags: &mut Vec<Option<Tag>>, cached_tags: &Vec<CacheFileTag>) -> RinghopperResult<()> {
        for i in 0..tag_count {
            let cached_tag = &cached_tags[i];
            let tag = if let Some(n) = tags[i].as_mut() { n } else { continue };

            if cached_tag.external != 0 {
                let tag_path = &tag.tag_path;

                if !engine.resource_maps.is_some_and(|r| r.externally_indexed_tags) {
                    return Err(Error::MapParseFailure(format!("`{tag_path}` marked as external when engine {} doesn't allow it", engine.name)))
                }

                let match_indexed_tag = |resource_map: &ResourceMap, resource_map_type: ResourceMapType| -> RinghopperResult<()> {
                    let item = resource_map.get(tag.address)
                        .ok_or_else(|| Error::MapParseFailure(format!("mismatched resource maps; `{tag_path}` not found in {resource_map_type:?}")))?;
                    let expected = item.get_path();
                    if expected != tag.tag_path.path() {
                        return Err(Error::MapParseFailure(
                            format!("mismatched resource maps; `{tag_path}` was actually `{expected}` in {resource_map_type:?}")
                        ))
                    }
                    Ok(())
                };

                match tag_path.group() {
                    TagGroup::Bitmap => {
                        if let Some(n) = self.bitmaps.as_ref() {
                            match_indexed_tag(n, ResourceMapType::Bitmaps)?;
                        }
                        tag.domain = DomainType::ResourceMapEntry(ResourceMapType::Bitmaps, tag_path.path().to_owned());
                        tag.address = 0;
                    }
                    TagGroup::Sound => self.patch_up_external_custom_edition_sound_tag(tag)?,
                    _ => {
                        if let Some(n) = self.loc.as_ref() {
                            match_indexed_tag(n, ResourceMapType::Loc)?;
                        }
                        tag.domain = DomainType::ResourceMapEntry(ResourceMapType::Loc, tag_path.path().to_owned());
                        tag.address = 0;
                    }
                }
            }
        }

        Ok(())
    }

    fn patch_up_external_custom_edition_sound_tag(&mut self, tag: &mut Tag) -> RinghopperResult<()> {
        if let Some(n) = self.sounds.as_ref() {
            let object = n.get_by_path(tag.tag_path.path())
                .ok_or_else(|| Error::MapParseFailure(format!("mismatched resource maps; `{tag_path}` not found in {resource_map_type:?}", tag_path=tag.tag_path, resource_map_type = ResourceMapType::Sounds)))?;

            let base_struct_size = Sound::size();
            let data_in_sounds = object.get_data();
            if data_in_sounds.len() < base_struct_size {
                return Err(Error::MapParseFailure(format!("mismatched resource maps; `{tag_path}` is corrupt in sounds.map", tag_path=tag.tag_path)));
            }
            let (base_struct_in_sounds, pitch_ranges) = data_in_sounds.split_at(base_struct_size);
            let base_struct_in_tags = self.get_data_at_address(tag.address, &tag.domain, base_struct_size)
                .ok_or_else(|| Error::MapParseFailure(format!("corrupted map; `{tag_path}` has no base struct data", tag_path=tag.tag_path)))?;

            // Pitch range data onwards starts at address 0
            let mut data = Vec::new();
            data.extend_from_slice(pitch_ranges);

            // We now add our base struct here
            tag.address = data.len();
            data.extend_from_slice(base_struct_in_tags);
            tag.domain = DomainType::ResourceMapEntry(ResourceMapType::Sounds, tag.tag_path.path().to_owned());

            // Merge some data
            let mut merged_base_struct = &mut data[tag.address..tag.address.add_overflow_checked(base_struct_size)?];

            // pitch ranges
            let pitch_range_reflexive_offset = 152;
            let mut pitch_range_reflexive = ReflexiveC::<SoundPitchRange>::read::<LittleEndian>(base_struct_in_sounds, pitch_range_reflexive_offset, base_struct_size).unwrap();
            pitch_range_reflexive.address = Address { address: 0 };
            pitch_range_reflexive.write::<LittleEndian>(&mut merged_base_struct, pitch_range_reflexive_offset, base_struct_size).unwrap();

            // sample rate
            merged_base_struct[6..8].copy_from_slice(&base_struct_in_sounds[6..8]);

            // channel count and format
            merged_base_struct[108..112].copy_from_slice(&base_struct_in_sounds[108..112]);

            self.merged_sound_resources.insert(tag.domain.clone(), data);
        }
        Ok(())
    }
}

impl Map for GearboxCacheFile {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_engine(&self) -> &'static Engine {
        self.engine
    }

    fn extract_tag(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        extract_tag_from_map(self, path, &self.scenario_tag_data, fix_bitmap_tag_normal, fix_model_tag_uncompressed)
    }

    fn get_domain(&self, domain: &DomainType) -> Option<(&[u8], usize)> {
        if let Some(n) = self.merged_sound_resources.get(&domain) {
            return Some((n, 0))
        }

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

            DomainType::ResourceMapFile(ResourceMapType::Bitmaps) => self.bitmaps.as_ref().map(|b| (b.data(), 0)),
            DomainType::ResourceMapFile(ResourceMapType::Sounds) => self.sounds.as_ref().map(|b| (b.data(), 0)),
            DomainType::ResourceMapFile(ResourceMapType::Loc) => self.loc.as_ref().map(|b| (b.data(), 0)),

            DomainType::ResourceMapEntry(r, path) => match r {
                ResourceMapType::Bitmaps => Some((self.bitmaps.as_ref()?.get_by_path(path)?.get_data(), 0)),
                ResourceMapType::Sounds => Some((self.sounds.as_ref()?.get_by_path(path)?.get_data(), 0)),
                ResourceMapType::Loc => Some((self.loc.as_ref()?.get_by_path(path)?.get_data(), 0))
            }

            _ => None
        }
    }

    fn get_tag_by_id(&self, id: ID) -> Option<&Tag> {
        self.tags.get(id.index()? as usize)?.as_ref()
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
impl MapTagTree for GearboxCacheFile {
    fn get_scenario_type(&self) -> ScenarioType {
        self.scenario_tag_data._type
    }
}
