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
    pub externally_indexed_tags: bool,
    pub max_cache_file_size: EngineCacheFileSize,
    pub base_memory_address: EngineBaseMemoryAddress,
    pub required_tags: EngineRequiredTags
}

pub struct EngineRequiredTags {
    pub all: &'static [&'static str],
    pub user_interface: &'static [&'static str],
    pub singleplayer: &'static [&'static str],
    pub multiplayer: &'static [&'static str],
}

pub struct EngineBuild {
    pub string: &'static str,
    pub fallback: &'static [&'static str],
    pub enforced: bool
}

pub struct EngineCacheFileSize {
    pub user_interface: u64,
    pub singleplayer: u64,
    pub multiplayer: u64
}

pub struct EngineBaseMemoryAddress {
    pub address: u64,
    pub inferred: bool
}
