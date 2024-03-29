use std::collections::HashMap;

use definitions::{CacheFileTagDataHeaderXbox, Scenario, ScenarioType};
use primitives::crc32::CRC32;
use primitives::engine::Engine;
use primitives::error::{OverflowCheck, RinghopperResult};
use primitives::map::{DomainType, Map, Tag};
use primitives::parse::{SimpleTagData, TagData};
use primitives::primitive::{ID, TagPath};
use primitives::tag::{ParseStrictness, PrimaryTagStructDyn};

use crate::map::{BSPDomain, extract_tag_from_map, MapTagTree, SizeRange};

pub struct XboxCacheFile {
    name: String,
    engine: &'static Engine,
    data: Vec<u8>,
    tag_data: SizeRange,
    bsp_data: Vec<BSPDomain>,
    base_memory_address: usize,
    tags: Vec<Option<Tag>>,
    ids: HashMap<TagPath, ID>,
    scenario_tag: ID,
    scenario_tag_data: Scenario
}

impl XboxCacheFile {
    pub fn new(data: Vec<u8>, _: ParseStrictness) -> RinghopperResult<Self> {
        let (header, engine, tag_data_range) = super::util::get_tag_data_details(&data)?;

        let mut map = Self {
            name: String::new(),
            data,
            engine,
            tag_data: Default::default(),
            bsp_data: Default::default(),
            base_memory_address: engine.base_memory_address.address as usize,
            tags: Default::default(),
            scenario_tag: ID::null(),
            ids: Default::default(),
            scenario_tag_data: Scenario::default()
        };

        map.tag_data = tag_data_range;
        map.name = header.name.to_string();

        let tag_data_header = CacheFileTagDataHeaderXbox::read_from_map(&map, map.base_memory_address, &DomainType::TagData)?;
        let tag_address: usize = tag_data_header.cache_file_tag_data_header.tag_array_address.into();
        let tag_count = tag_data_header.cache_file_tag_data_header.tag_count as usize;
        if engine.base_memory_address.inferred {
            map.base_memory_address = tag_address.sub_overflow_checked(CacheFileTagDataHeaderXbox::simple_size())?;
        }
        map.scenario_tag = tag_data_header.cache_file_tag_data_header.scenario_tag;

        let (tags, _cached_tags, ids) = super::util::get_all_tags(&mut map, tag_address, tag_count, CacheFileTagDataHeaderXbox::simple_size())?;
        map.ids = ids;
        map.tags = tags;

        // Get the scenario tag to get the BSP data
        let (scenario_tag_data, bsps) = super::util::load_scenario_info(&map, &map.tags)?;
        map.bsp_data = bsps;
        map.scenario_tag_data = scenario_tag_data;
        super::util::fixup_bsp_addresses(&mut map.tags, &map.bsp_data, &map.ids)?;

        // Should never be true
        debug_assert!(!engine.external_bsps);

        // Done one more time to make sure everything's good
        debug_assert!(map.data.get(map.tag_data.clone()).is_some());
        debug_assert!(map.bsp_data.iter().all(|f| map.data.get(f.range.clone()).is_some()));

        // TODO: check CRC32 if we ever figure out how to do this for Xbox maps

        Ok(map)
    }
}

impl Map for XboxCacheFile {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_engine(&self) -> &'static Engine {
        self.engine
    }

    fn extract_tag(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        extract_tag_from_map(self, path, &self.scenario_tag_data)
    }

    fn get_domain(&self, domain: &DomainType) -> Option<(&[u8], usize)> {
        match domain {
            DomainType::MapData => Some((self.data.as_slice(), 0)),

            // OK because these are checked on load
            DomainType::TagData => Some((unsafe { self.data.get_unchecked(self.tag_data.clone()) }, self.base_memory_address)),
            DomainType::BSP(b) => {
                let bsp = self.bsp_data.get(*b)?;
                Some((unsafe { self.data.get_unchecked(bsp.range.clone()) }, bsp.base_address))
            }

            DomainType::ResourceMapFile(_) => None,
            DomainType::ResourceMapEntry(_, _) => None,
            DomainType::BSPVertices(_) => None,
            DomainType::ModelVertexData => None,
            DomainType::ModelTriangleData => None,
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

    fn calculate_crc32(&self) -> u32 {
        let mut hasher = CRC32::new();

        for bsp in 0..self.bsp_data.len() {
            if self.engine.external_bsps {
                hasher.update(self.get_domain(&DomainType::BSPVertices(bsp)).unwrap().0);
            }
            hasher.update(self.get_domain(&DomainType::BSP(bsp)).unwrap().0);
        }
        hasher.update(self.get_domain(&DomainType::ModelVertexData).unwrap().0);
        hasher.update(self.get_domain(&DomainType::ModelTriangleData).unwrap().0);
        hasher.update(self.get_domain(&DomainType::TagData).unwrap().0);

        hasher.crc()
    }
}
impl MapTagTree for XboxCacheFile {
    fn get_scenario_type(&self) -> ScenarioType {
        self.scenario_tag_data._type
    }
}
