#[derive(Copy, Clone)]
pub struct Engine {
    pub name: &'static str,
    pub display_name: &'static str,
    pub version: Option<&'static str>,
    pub build: Option<EngineBuild>,
    pub build_target: bool,
    pub cache_default: bool,
    pub cache_file_version: u32,
    pub max_script_nodes: u64,
    pub max_tag_space: u64,
    pub external_bsps: bool,
    pub resource_maps: Option<EngineSupportedResourceMaps>,
    pub compression_type: EngineCompressionType,
    pub max_cache_file_size: EngineCacheFileSize,
    pub base_memory_address: EngineBaseMemoryAddress,
    pub required_tags: EngineRequiredTags,
    pub cache_parser: EngineCacheParser
}

#[derive(Copy, Clone)]
pub enum EngineCacheParser {
    PC,
    Xbox
}

#[derive(Copy, Clone)]
pub struct EngineSupportedResourceMaps {
    pub externally_indexed_tags: bool,
    pub loc: bool
}

#[derive(Copy, Clone, PartialEq)]
pub enum EngineCompressionType {
    Uncompressed,
    Deflate
}

#[derive(Copy, Clone)]
pub struct EngineRequiredTags {
    pub all: &'static [&'static str],
    pub user_interface: &'static [&'static str],
    pub singleplayer: &'static [&'static str],
    pub multiplayer: &'static [&'static str],
}

#[derive(Copy, Clone)]
pub struct EngineBuild {
    pub string: &'static str,
    pub fallback: &'static [&'static str],
    pub enforced: bool
}

#[derive(Copy, Clone)]
pub struct EngineCacheFileSize {
    pub user_interface: u64,
    pub singleplayer: u64,
    pub multiplayer: u64
}

#[derive(Copy, Clone)]
pub struct EngineBaseMemoryAddress {
    pub address: u64,
    pub inferred: bool
}
