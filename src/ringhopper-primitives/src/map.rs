use crate::engine::Engine;
use crate::error::RinghopperResult;
use crate::primitive::{ID, IDType, Index, TagPath, TagReference};
use crate::tag::PrimaryTagStructDyn;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceMapType {
    Bitmaps,
    Sounds,
    Loc
}

/// Domains are regions of memory that may have their own address space.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DomainType {
    /// Map file data (0x0 = cache file header).
    MapData,

    /// Main tag data (0x0 = tag data header).
    TagData,

    /// BSP data for the given BSP (0x0 = BSP main struct).
    BSP(usize),

    /// BSP vertices for the given BSP (MCC only) (0x0 = start of vertices).
    BSPVertices(usize),

    /// Resource map file (0x0 = start of resource map).
    ResourceMapFile(ResourceMapType),

    /// Resource map type and path (0x0 = start of resource map entry).
    ResourceMapEntry(ResourceMapType, String),

    /// Vertex data (0x0 = start of vertices)
    ModelVertexData,

    /// Triangle data (0x0 = start of triangles)
    ModelTriangleData
}

/// Cached tag data.
#[derive(Clone)]
pub struct Tag {
    /// Path of the tag.
    pub tag_path: TagPath,

    /// Main address of the tag's base struct.
    pub address: usize,

    /// Domain of the tag's base struct.
    pub domain: DomainType
}

/// Map parsing functionality.
pub trait Map {
    /// Get the name of the map.
    fn get_name(&self) -> &str;

    /// Get the build string of the map.
    ///
    /// NOTE: This is not useful for identifying the engine. Use [`get_engine`](Map::get_engine) for this.
    fn get_build_string(&self) -> &str;

    /// Get the CRC32 of the map, if available.
    ///
    /// The first value is the CRC32 in the header. The second value is the calculated CRC32.
    fn get_crc32(&self) -> Option<(u32, u32)> {
        None
    }

    /// Get an estimated max tag space for the map.
    ///
    /// This may or may not be reliable, as it assumes the map was built with the BSP data at the end of tag data.
    fn get_estimated_max_tag_space(&self) -> Option<usize> {
        None
    }

    /// Get the uncompressed size of the map, if available.
    fn get_uncompressed_size(&self) -> Option<usize> {
        None
    }

    /// Get the tag space usage, if available.
    fn get_used_tag_space(&self) -> Option<usize> {
        None
    }

    /// Get the engine for the map.
    fn get_engine(&self) -> &'static Engine;

    /// Extract the tag.
    fn extract_tag(
        &self,
        path: &TagPath
    ) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>>;

    /// Get the domain data, returning a slice and the base address offset.
    fn get_domain(
        &self,
        domain: &DomainType
    ) -> Option<(&[u8], usize)>;

    /// Get the tag for the tag id.
    ///
    /// Returns None if the ID is invalid or null.
    fn get_tag_by_id(
        &self,
        id: ID
    ) -> Option<&Tag>;

    /// Get the tag for the tag path.
    ///
    /// Returns None if the tag does not exist in the map.
    fn get_tag(
        &self,
        path: &TagPath
    ) -> Option<&Tag>;

    /// Get the scenario tag for the tag ID.
    fn get_scenario_tag(&self) -> &Tag;

    /// Get the tag for the given index.
    ///
    /// Returns `None` if the tag index is not a valid tag index.
    ///
    /// The default implementation should satisfy most use-cases.
    fn get_tag_by_index(
        &self,
        index: usize
    ) -> Option<&Tag> {
        if index > u16::MAX as usize {
            return None
        }
        let id = ID::new(Index::from(index.try_into().ok()), IDType::Tag as u16);
        self.get_tag_by_id(id)
    }

    /// Get the tag for the given tag reference.
    ///
    /// Returns `None` if the tag index is not a valid tag index.
    ///
    /// The default implementation should satisfy all use-cases.
    fn get_tag_by_tag_reference(
        &self,
        reference: &TagReference
    ) -> Option<&Tag> {
        self.get_tag(reference.path()?)
    }

    /// Get the data at the given location.
    ///
    /// Returns `None` if the data is unavailable or out-of-bounds.
    ///
    /// The default implementation should satisfy most use-cases. Additionally, it will try tag data if the
    /// address is out-of-bounds for BSP data.
    fn get_data_at_address(
        &self,
        address: usize,
        domain: &DomainType,
        size: usize
    ) -> Option<&[u8]> {
        let (domain_start, domain_base) = self.get_domain(domain)?;
        let offset = address.checked_sub(domain_base)?;
        let end = offset.checked_add(size)?;

        let data = domain_start.get(offset..end);
        if data.is_none() && matches!(domain, DomainType::BSP(_)) {
            // try the tag data
            self.get_data_at_address(address, &DomainType::TagData, size)
        }
        else {
            data
        }
    }

    /// Returns all tags in the cache file.
    fn get_all_tags(&self) -> Vec<TagPath>;

    /// Get the C string at the given location.
    ///
    /// Returns `None` if the data is unavailable or out-of-bounds, or it is not UTF-8.
    ///
    /// The default implementation should satisfy most use-cases. Additionally, it will try tag data if the
    /// address is out-of-bounds for BSP data.
    fn get_c_string_at_address(
        &self,
        address: usize,
        domain: &DomainType
    ) -> Option<&str> {
        let (domain_start, domain_base) = self.get_domain(domain)?;
        let offset = address.checked_sub(domain_base)?;
        let data = domain_start.get(offset..);
        let data = if data.is_none() && matches!(domain, DomainType::BSP(_)) {
            // try the tag data
            return self.get_c_string_at_address(address, &DomainType::TagData)
        }
        else {
            data?
        };

        std::ffi::CStr::from_bytes_until_nul(data)
            .ok()?
            .to_str()
            .ok()
    }
}
