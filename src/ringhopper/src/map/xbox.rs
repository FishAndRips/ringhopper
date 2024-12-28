use std::collections::HashMap;

use definitions::{CacheFileTagDataHeaderInternalModels, Scenario, ScenarioType};
use primitives::engine::Engine;
use primitives::error::{OverflowCheck, RinghopperResult};
use primitives::map::{DomainType, Map, Tag};
use primitives::parse::{SimpleTagData, TagData};
use primitives::primitive::{ID, TagPath};
use primitives::tag::{ParseStrictness, PrimaryTagStructDyn};

use crate::map::{BSPDomain, extract_tag_from_map, MapTagTree, SizeRange};

pub struct XboxCacheFile {
    name: String,
    build_string: String,
    estimated_max_tag_space: Option<usize>,
    engine: &'static Engine,
    data: Vec<u8>,
    tag_data: SizeRange,
    bsp_data: Vec<BSPDomain>,
    base_memory_address: usize,
    tags: Vec<Option<Tag>>,
    ids: HashMap<TagPath, ID>,
    scenario_tag: ID,
    scenario_tag_data: Scenario,
    uncompressed_size: usize,
    used_tag_space: usize,

}

impl XboxCacheFile {
    pub fn new(data: Vec<u8>, _: ParseStrictness) -> RinghopperResult<Self> {
        let (header, engine, tag_data_range) = super::util::get_tag_data_details(&data)?;

        let mut map = Self {
            name: String::new(),
            build_string: String::new(),
            uncompressed_size: data.len(),
            estimated_max_tag_space: None,
            data,
            engine,
            used_tag_space: 0,
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
        map.build_string = header.build.to_string();

        debug_assert!(!engine.external_models, "external models not supported on Xbox maps (if you triggered this, you broke an engine, or you need to add the support yourself)");

        let tag_data_header = CacheFileTagDataHeaderInternalModels::read_from_map(&map, map.base_memory_address, &DomainType::TagData)?;
        let tag_address: usize = tag_data_header.cache_file_tag_data_header.tag_array_address.into();
        let tag_count = tag_data_header.cache_file_tag_data_header.tag_count as usize;
        if engine.base_memory_address.inferred {
            map.base_memory_address = tag_address.sub_overflow_checked(CacheFileTagDataHeaderInternalModels::simple_size())?;
        }
        map.scenario_tag = tag_data_header.cache_file_tag_data_header.scenario_tag;

        let (tags, _cached_tags, ids) = super::util::get_all_tags(&mut map, tag_address, tag_count, CacheFileTagDataHeaderInternalModels::simple_size())?;
        map.ids = ids;
        map.tags = tags;

        // Get the scenario tag to get the BSP data
        let (scenario_tag_data, bsps) = super::util::load_scenario_info(&map, &map.tags)?;
        map.bsp_data = bsps;
        map.scenario_tag_data = scenario_tag_data;
        super::util::fixup_bsp_addresses(&mut map.tags, &map.bsp_data, &map.ids)?;

        map.used_tag_space = map.tag_data.end - map.tag_data.start;
        if !engine.external_bsps && !map.bsp_data.is_empty() {
            map.used_tag_space = map.used_tag_space.saturating_add(map.bsp_data.iter().map(|b| b.range.len()).max().expect("no bsps?"));

            let first_bsp = map.bsp_data.first().expect("bsp");
            map.estimated_max_tag_space = first_bsp
                .base_address
                .checked_add(first_bsp.range.len()).and_then(|b| b.checked_sub(map.base_memory_address));
        }

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

    fn get_build_string(&self) -> &str {
        &self.build_string
    }

    fn get_estimated_max_tag_space(&self) -> Option<usize> {
        self.estimated_max_tag_space
    }

    fn get_uncompressed_size(&self) -> Option<usize> {
        Some(self.uncompressed_size)
    }

    fn get_used_tag_space(&self) -> Option<usize> {
        Some(self.used_tag_space)
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
}
impl MapTagTree for XboxCacheFile {
    fn get_scenario_type(&self) -> ScenarioType {
        self.scenario_tag_data._type
    }
}
