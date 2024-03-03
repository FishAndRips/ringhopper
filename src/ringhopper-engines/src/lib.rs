pub use ringhopper_definitions::{BaseMemoryAddress, EngineCacheFileSize};

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
    pub max_cache_file_size: EngineCacheFileSize,
    pub base_memory_address: BaseMemoryAddress,
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

ringhopper_engines_codegen::generate_ringhopper_engines!();
