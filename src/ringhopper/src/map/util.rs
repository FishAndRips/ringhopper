use std::collections::HashMap;
use std::ops::Range;
use definitions::{CacheFileTag, Scenario, ScenarioStructureBSPCompiledHeader};
use primitives::engine::Engine;
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::map::{DomainType, Map, Tag};
use primitives::parse::{SimpleTagData, TagData};
use primitives::primitive::{ID, TagGroup, TagPath};
use crate::map::{BSPDomain, header};
use crate::map::header::ParsedCacheFileHeader;

/// Get all scenario and BSP data for a map.
pub fn load_scenario_info<M: Map>(map: &M, tags: &[Option<Tag>]) -> RinghopperResult<(Scenario, Vec<BSPDomain>)> {
    let scenario = map.get_scenario_tag();
    if scenario.tag_path.group() != TagGroup::Scenario {
        return Err(Error::MapParseFailure(format!("scenario tag is marked as a {} tag; likely protected/corrupted map", scenario.tag_path.group())))
    }

    for t in tags {
        let tag = if let Some(n) = t.as_ref() { n } else { continue };
        if tag.tag_path.group() == TagGroup::Scenario && tag.tag_path != scenario.tag_path {
            return Err(Error::MapParseFailure(format!("Extraneous scenario tag {} in the map (map likely protected/corrupted)", tag.tag_path)))
        }
    }

    let scenario_tag_data = Scenario::read_from_map(map, scenario.address, &scenario.domain)?;
    let bsp_count = scenario_tag_data.structure_bsps.items.len();
    let mut bsps: Vec<BSPDomain> = Vec::with_capacity(bsp_count);
    for bsp_index in 0..bsp_count {
        let bsp = &scenario_tag_data.structure_bsps.items[bsp_index];
        let reference = &bsp.structure_bsp;
        let start = bsp.bsp_start as usize;
        let length = bsp.bsp_size as usize;
        let end = start.add_overflow_checked(length)?;
        let range = start..end;

        if map.get_data_at_address(range.start, &DomainType::MapData, length).is_none() {
            return Err(Error::MapParseFailure(format!("BSP #{bsp_index} ({reference}) has an invalid range 0x{start:08X}[0x{length:08X}]")))
        }

        let bsp_base_address = bsp.bsp_address as usize;
        if let Some(path) = bsp.structure_bsp.path() {
            for i in &bsps {
                if i.path.as_ref().is_some_and(|p| p == path) {
                    return Err(Error::MapParseFailure(format!("BSP tag {path} has ambiguous data")))
                }
            }
        }

        let header = ScenarioStructureBSPCompiledHeader::read_from_map(
            map,
            range.start,
            &DomainType::MapData
        )?;

        bsps.push(BSPDomain {
            range,
            base_address: bsp_base_address,
            path: bsp.structure_bsp.path().map(|p| p.to_owned()),
            tag_address: header.pointer.into()
        });
    }
    Ok((scenario_tag_data, bsps))
}

/// Gets the header, engine, and tag data range (after verifying it).
pub fn get_tag_data_details(data: &[u8]) -> RinghopperResult<(ParsedCacheFileHeader, &'static Engine, Range<usize>)> {
    let (header, engine) = header::get_map_details(&data)?;
    let tag_data_start = header.tag_data_offset;
    let tag_data_end = tag_data_start.add_overflow_checked(header.tag_data_size)?;
    let tag_data_range = tag_data_start..tag_data_end;
    if data.get(tag_data_range.clone()).is_none() {
        return Err(
            Error::MapParseFailure(format!("tag data region out of bounds 0x{tag_data_start:08X} - 0x{tag_data_start:08X}"))
        )
    }
    Ok((header, engine, tag_data_range))
}

/// Get all tag info, returning the parsed tags, cached tags, and a map of tag paths to IDs.
pub fn get_all_tags<M: Map>(map: &M, tag_address: usize, tag_count: usize, tag_header_size: usize) -> RinghopperResult<(Vec<Option<Tag>>, Vec<CacheFileTag>, HashMap<TagPath, ID>)> {
    if tag_count > u16::MAX as usize {
        return Err(
            Error::MapParseFailure(format!("maximum tag count exceeded (map claims to have {tag_count} tags"))
        )
    }

    let mut tags = Vec::with_capacity(tag_count);
    let mut cached_tags = Vec::with_capacity(tag_count);
    let mut ids: HashMap<TagPath, ID> = HashMap::with_capacity(tag_count);

    for i in CacheFileTag::read_chunks_from_map_to_iterator(map, tag_count, tag_address, &DomainType::TagData)? {
        // Did we fail to get a tag due to an invalid fourcc? It's probably protected, or our definitions don't match this map.
        if i.as_ref().is_err_and(|e| matches!(e, Error::InvalidFourCC)) {
            let engine = map.get_engine();

            // Is it inferred? If so, it probably shouldn't be, or the map is broken.
            if engine.base_memory_address.inferred {
                return Err(Error::MapParseFailure(format!("unable to read the tag array; the cache file may be corrupted/protected, or the tag array is not directly after the header (base memory address is inferred for engine `{}`)", engine.name)));
            }

            // What would it be if we inferred it? Is it different? Maybe our engine definitions fail to cover this engine.
            let inferred_address = tag_address.checked_sub(tag_header_size);
            if inferred_address.is_some_and(|inferred| inferred as u64 != engine.base_memory_address.address) {
                return Err(Error::MapParseFailure(format!("unable to read the tag array; the cache file may be corrupted/protected (and the tag array is not immediately after the header), or the base memory address is incorrect (base memory address for engine `{}` is 0x{:08X}, but if inferred, it would be 0x{:08X})", engine.name, engine.base_memory_address.address, inferred_address.unwrap())))
            }

            // If not, the map is DEFINITELY broken.
            return Err(Error::MapParseFailure("unable to read the tag array; the cache file is likely corrupted/protected".to_string()));
        }

        cached_tags.push(i?);
    }

    let tag_array = cached_tags.iter().zip(0..tag_count);
    for (cached_tag, t) in tag_array {
        let group = cached_tag.tag_group;
        if group == TagGroup::_Unset {
            tags.push(None);
            continue
        }

        let expected_index = cached_tag.id.index();
        if expected_index != Some(t as u16) {
            return Err(Error::MapParseFailure(format!("tag #{t} has an invalid tag ID")))
        }

        let tag_path_address = cached_tag.path.into();
        let path = map
            .get_c_string_at_address(tag_path_address, &DomainType::TagData)
            .ok_or_else(|| Error::MapParseFailure(format!("unable to get the tag path for tag #{t} due to a bad address 0x{tag_path_address:08X}")))?;

        let tag_path = TagPath::new(path, group)
            .map_err(|e| Error::MapParseFailure(format!("unable to get the tag path for tag #{t} ({path}) due to a parse failure: {e}")))?;

        if group != TagGroup::_Unset {
            if ids.get(&tag_path).is_some() {
                return Err(Error::MapParseFailure(format!("multiple instances of tag {tag_path} detected")))
            }
            ids.insert(tag_path.clone(), cached_tag.id);
        }

        tags.push(Some(Tag {
            tag_path,
            address: cached_tag.data.into(),
            domain: DomainType::TagData
        }));
    }
    Ok((tags, cached_tags, ids))
}

/// Fix all BSP tags to use the BSP domains.
pub fn fixup_bsp_addresses(tags: &mut [Option<Tag>], bsps: &[BSPDomain], ids: &HashMap<TagPath, ID>) -> Result<(), Error> {
    for i in 0..bsps.len() {
        let bsp = &bsps[i];
        if let Some(p) = &bsp.path {
            let tag = tags[ids[p].index().unwrap() as usize].as_mut().unwrap();
            tag.address = bsp.tag_address;
            tag.domain = DomainType::BSP(i);
        }
    }
    for i in tags {
        if let Some(n) = i {
            if n.tag_path.group() == TagGroup::ScenarioStructureBSP && !matches!(n.domain, DomainType::BSP(_)) {
                return Err(Error::MapParseFailure(format!("BSP tag {} has no corresponding data in the scenario tag", n.tag_path)))
            }
        }
    }
    Ok(())
}
