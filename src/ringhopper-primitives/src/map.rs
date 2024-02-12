use crate::error::RinghopperResult;
use crate::primitive::{ID, IDType, Index, TagPath};
use crate::tag::PrimaryTagStructDyn;

#[derive(Copy, Clone, Debug)]
pub enum ResourceMapType {
    Bitmaps,
    Sounds,
    Loc
}

/// Domains are regions of memory that may have their own address space.
#[derive(Clone, Debug)]
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
    pub tag_path: TagPath,
    pub address: usize,
    pub domain: DomainType
}

/// Map parsing functionality.
pub trait Map {
    /// Get the name of the map.
    fn get_name(&self) -> &str;

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
