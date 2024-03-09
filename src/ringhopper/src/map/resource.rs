use definitions::{ResourceMapHeader, ResourceMapResource};
use primitives::byteorder::LittleEndian;
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::parse::SimpleTagData;
use crate::map::SizeRange;

#[derive(Default, Clone)]
pub struct ResourceMap {
    resources: Vec<ResourceItem>,
    data: Vec<u8>
}

impl ResourceMap {
    /// Instantiate a ResourceMap from data, consuming it.
    ///
    /// Returns an error if parsing failed.
    pub fn from_data(data: Vec<u8>) -> RinghopperResult<ResourceMap> {
        let data_slice = data.as_slice();

        let header = ResourceMapHeader::read::<LittleEndian>(data_slice, 0, data_slice.len())
            .map_err(|e| Error::MapParseFailure(format!("Resource map parse failure: can't read resource map header: {e:?}")))?;

        let path_data_offset = header.path_data_offset as usize;
        let path_data = data_slice.get(path_data_offset..)
            .ok_or_else(|| Error::MapParseFailure(format!("Resource map parse failure: path data offset 0x{path_data_offset:08X} is out-of-bounds")))?;

        let mut current_offset = header.array_offset as usize;
        let array_len = header.count as usize;
        let mut resources = Vec::with_capacity(array_len);
        let resource_len = ResourceMapResource::simple_size();

        for i in 0..array_len {
            let data = ResourceMapResource::read::<LittleEndian>(data_slice, current_offset, data_slice.len())
                .map_err(|e| Error::MapParseFailure(format!("Resource map parse failure: array index {i}: {e:?}")))?;
            current_offset = current_offset.add_overflow_checked(resource_len)?;

            let path_range = (|| -> RinghopperResult<SizeRange> {
                let path_offset_start = data.path_offset as usize;
                let string_start = path_data.get(path_offset_start..)
                    .ok_or_else(|| Error::MapParseFailure(format!("0x{path_offset_start:08X} out-of-bounds in path data")))?;

                let string = std::ffi::CStr::from_bytes_until_nul(string_start)
                    .map_err(|_| Error::MapParseFailure(format!("0x{path_offset_start:08X} has no C string")))?
                    .to_str()
                    .map_err(|_| Error::MapParseFailure(format!("0x{path_offset_start:08X} is not a valid UTF-8 string")))?;

                let length = string.len();
                let normalized_path_start = path_offset_start + path_data_offset;
                let normalized_path_end = normalized_path_start + length;

                let path_range = normalized_path_start..normalized_path_end;

                debug_assert_eq!(data_slice.get(path_range.clone()).expect("path get"), string.as_bytes());

                Ok(path_range)
            })().map_err(|e| Error::MapParseFailure(format!("Resource map parse failure: array index {i}: can't get path offset: {e:?}")))?;

            let data_range = (|| -> RinghopperResult<SizeRange> {
                let data_offset_start = data.data_offset as usize;
                let data_size = data.data_size as usize;
                let data_offset_end = data_offset_start.add_overflow_checked(data_size)?;

                let range = data_offset_start..data_offset_end;
                data_slice.get(range.clone()).ok_or_else(
                    || Error::MapParseFailure(format!("0x{data_offset_start:08X}[0x{data_size:08X}] out-of-bounds"))
                )?;
                Ok(range)
            })().map_err(|e| Error::MapParseFailure(format!("Resource map parse failure: array index {i}: can't get data offset: {e:?}")))?;

            resources.push(ResourceItem {
                data: data_range,
                path: path_range
            })
        }

        Ok(ResourceMap {
            resources,
            data
        })
    }

    /// Get a reference to all data in the resource map.
    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Consume the ResourceMap back into data.
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// Get the number of elements in the resource map.
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Get a resource at an index.
    ///
    /// If `index` is out-of-bounds, `None` is returned.
    pub fn get(&self, index: usize) -> Option<Resource> {
        self.resources.get(index).map(|resource|
            // This is fine, because the invariant is upheld in from_data()
            unsafe { Resource::from_resource_item(resource, self) }
        )
    }

    /// Get a resource at an index.
    ///
    /// # Safety
    ///
    /// If `index` is out-of-bounds, this is undefined behavior.
    pub unsafe fn get_unchecked(&self, index: usize) -> Resource {
        // Provided get_unchecked does not have UB, from_resource_item is fine because the invariant is upheld in from_data()
        Resource::from_resource_item(self.resources.get_unchecked(index), self)
    }

    /// Get a resource by its path.
    ///
    /// If `path` does not match anything, `None` is returned.
    pub fn get_by_path(&self, path: &str) -> Option<Resource> {
        for resource in &self.resources {
            // Fine because we already checked this in from_data()
            let resource = unsafe { Resource::from_resource_item(resource, self) };
            if resource.path == path {
                return Some(resource)
            }
        }
        None
    }
}

#[derive(Clone)]
struct ResourceItem {
    path: SizeRange,
    data: SizeRange
}

/// Indicates a single resource in a [`ResourceMap`].
pub struct Resource<'a> {
    path: &'a str,
    data: &'a [u8],
    data_offset: usize,
}

impl<'a> Resource<'a> {
    /// Creates a resource.
    ///
    /// This is undefined behavior if ResourceItem does not point to correct offsets or the path is not UTF-8.
    unsafe fn from_resource_item(item: &'a ResourceItem, map: &'a ResourceMap) -> Resource<'a> {
        let data = item.data.clone();
        let path = item.path.clone();

        Resource {
            data_offset: data.start,
            data: map.data.get_unchecked(data),
            path: std::str::from_utf8_unchecked(map.data.get_unchecked(path))
        }
    }

    /// Get the offset to the data in the resource map.
    pub fn get_data_offset(&self) -> usize {
        self.data_offset
    }

    /// Get the data in the resource map.
    pub fn get_data(&self) -> &'a [u8] {
        self.data
    }

    /// Get the name of the resource.
    pub fn get_path(&self) -> &'a str {
        self.path
    }
}
