use definitions::{CEAFlags, CacheFileHeader, CacheFileHeaderPCDemo, ScenarioType};
use primitives::byteorder::{ByteOrder, LittleEndian};
use primitives::engine::Engine;
use primitives::error::{Error, RinghopperResult};
use primitives::parse::SimpleTagData;
use primitives::primitive::{FourCC, String32};
use ringhopper_engines::ALL_SUPPORTED_ENGINES;

/// Functionality to read headers of cache files.
#[derive(Copy, Clone)]
pub struct ParsedCacheFileHeader {
    /// The name of the scenario.
    ///
    /// Note that this may not necessarily correspond to the actual scenario.
    pub name: String32,

    /// The build of the cache file.
    ///
    /// This is used to determine the engine on some engines depending on `cache_version`, but may be set to anything on others.
    pub build: String32,

    /// The cache version.
    ///
    /// This is used to determine the engine.
    pub cache_version: u32,

    /// The offset to the tag data in bytes.
    pub tag_data_offset: usize,

    /// The length of the tag data in bytes.
    pub tag_data_size: usize,

    /// Final decompressed size of the map.
    ///
    /// NOTE: This size may be wrong on corrupted custom maps either due to bad tools (e.g. old tools like Arsenic) or
    /// to bypass Halo PC's 128/384 MB limitation.
    pub decompressed_size: usize,

    /// Bytes at the end to ignore if the map is compressed.
    pub compression_padding: usize,

    /// The type of scenario.
    ///
    /// Note that this may not necessarily correspond to the actual scenario type.
    pub map_type: ScenarioType,

    /// The CRC32 of the cache file.
    ///
    /// Note that this may not necessarily be accurate.
    pub crc32: u32,
}

const HEAD_FOURCC: FourCC = 0x68656164;
const FOOT_FOURCC: FourCC = 0x666F6F74;

const HEAD_FOURCC_DEMO: FourCC = 0x45686564;
const FOOT_FOURCC_DEMO: FourCC = 0x47666F74;

impl ParsedCacheFileHeader {
    /// Read the header from the map data.
    pub fn read_from_map_data(map_data: &[u8]) -> RinghopperResult<ParsedCacheFileHeader> {
        let header_slice = match map_data.get(0..0x800) {
            Some(n) => n,
            None => return Err(Error::MapParseFailure("can't read the cache file header (too small to be a cache file)".to_owned()))
        };

        match CacheFileHeader::read::<LittleEndian>(header_slice, 0, header_slice.len()) {
            Ok(n) => if n.head_fourcc == HEAD_FOURCC && n.foot_fourcc == FOOT_FOURCC { return Ok(n.into()) },
            Err(_) => ()
        };

        match CacheFileHeaderPCDemo::read::<LittleEndian>(header_slice, 0, header_slice.len()) {
            Ok(n) => if n.head_fourcc == HEAD_FOURCC_DEMO && n.foot_fourcc == FOOT_FOURCC_DEMO { return Ok(n.into()) },
            Err(_) => ()
        };

        Err(Error::MapParseFailure("can't read the cache file header (not in retail or pc demo format)".to_owned()))
    }

    /// Match the header to an engine.
    pub fn match_engine(&self) -> Option<&'static Engine> {
        let mut best_result = None;
        let build_as_str = self.build.as_str();

        for i in ALL_SUPPORTED_ENGINES {
            // Must always be equal to match.
            if i.cache_file_version != self.cache_version {
                continue
            }

            if let Some(n) = &i.build {
                // Exact match?
                if n.string == build_as_str {
                    return Some(i);
                }

                // Any past maps that were released for this engine with a matching build string?
                for &fallback in n.fallback {
                    if fallback == build_as_str {
                        return Some(i);
                    }
                }

                // Are non-exact matches not allowed?
                if n.enforced {
                    continue
                }
            }

            // Default to this if cache_default (but only if we don't find something else first)
            if i.cache_default {
                debug_assert!(best_result.is_none());
                best_result = Some(i)
            }
        }

        best_result
    }
    
    pub fn into_header_for_engine(mut self, engine: &Engine) -> Self {
        if let Some(build) = engine.build {
            if build.enforced {
                self.build = String32::from_str(build.string).expect("engine has bad build string")
            }
        }
        self.cache_version = engine.cache_file_version;
        self
    }
    
    pub fn as_bytes<B: ByteOrder>(&self) -> [u8; 0x800] {
        let use_demo = match self.match_engine() {
            Some(e) => e.obfuscated_header_layout,
            None => false
        };
        
        let mut data = [0u8; 0x800];
        let len = data.len();
        
        if use_demo {
            CacheFileHeaderPCDemo {
                map_type: self.map_type,
                head_fourcc: HEAD_FOURCC_DEMO,
                foot_fourcc: FOOT_FOURCC_DEMO,
                tag_data_size: u32::try_from(self.tag_data_size).expect("tag_data_size > u32::MAX"),
                tag_data_offset: u32::try_from(self.tag_data_offset).expect("tag_data_offset > u32::MAX"),
                build: self.build,
                cache_version: self.cache_version,
                name: self.name,
                crc32: self.crc32,
                decompressed_size: u32::try_from(self.decompressed_size).expect("decompressed_size > u32::MAX"),
            }.write::<B>(&mut data, 0, len).expect("writing obfuscated header failed");
        }
        else {
            CacheFileHeader {
                map_type: self.map_type,
                head_fourcc: HEAD_FOURCC,
                foot_fourcc: FOOT_FOURCC,
                tag_data_size: u32::try_from(self.tag_data_size).expect("tag_data_size > u32::MAX"),
                tag_data_offset: u32::try_from(self.tag_data_offset).expect("tag_data_offset > u32::MAX"),
                build: self.build,
                cache_version: self.cache_version,
                name: self.name,
                crc32: self.crc32,
                decompressed_size: u32::try_from(self.decompressed_size).expect("decompressed_size > u32::MAX"),
                cea_flags: CEAFlags::default(),
                compression_padding: u32::try_from(self.compression_padding).expect("compression_padding > u32::MAX")
            }.write::<B>(&mut data, 0, len).expect("writing header failed");
        };
        
        data
    }
}

/// Get the map details, returning a cache file header and an engine.
///
/// Returns an error if the map could not be identified.
pub fn get_map_details(map_data: &[u8]) -> RinghopperResult<(ParsedCacheFileHeader, &'static Engine)> {
    let header = ParsedCacheFileHeader::read_from_map_data(map_data)?;
    match header.match_engine() {
        Some(n) => Ok((header, n)),
        None => Err(Error::MapParseFailure("unable to identify the map's engine (unknown engine)".to_string()))
    }
}

impl From<CacheFileHeaderPCDemo> for ParsedCacheFileHeader {
    fn from(value: CacheFileHeaderPCDemo) -> Self {
        Self {
            name: value.name,
            build: value.build,
            cache_version: value.cache_version,
            tag_data_offset: value.tag_data_offset as usize,
            tag_data_size: value.tag_data_size as usize,
            map_type: value.map_type,
            crc32: value.crc32,
            compression_padding: 0,
            decompressed_size: value.decompressed_size as usize
        }
    }
}

impl From<CacheFileHeader> for ParsedCacheFileHeader {
    fn from(value: CacheFileHeader) -> Self {
        Self {
            name: value.name,
            build: value.build,
            cache_version: value.cache_version,
            tag_data_offset: value.tag_data_offset as usize,
            tag_data_size: value.tag_data_size as usize,
            map_type: value.map_type,
            crc32: value.crc32,
            compression_padding: value.compression_padding as usize,
            decompressed_size: value.decompressed_size as usize
        }
    }
}
