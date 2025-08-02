#[derive(Copy, Clone, Debug)]
pub struct Engine {
    /// Shorthand name of the engine.
    pub name: &'static str,

    /// Long name of the engine.
    pub display_name: &'static str,

    /// Version data of the engine.
    pub version: Option<&'static str>,

    /// Build string of the engine.
    pub build: Option<EngineBuild>,

    /// The engine can have maps built for it.
    pub build_target: bool,

    /// This is a custom target.
    pub custom: bool,

    /// This is purely a fallback engine (i.e. a virtual engine) and is not a real target.
    pub fallback: bool,

    /// Prefer this engine if the build/version are ambiguous.
    pub cache_default: bool,

    /// Cache file version in header.
    pub cache_file_version: u32,

    /// Maximum number of script nodes.
    pub max_script_nodes: u64,

    /// Maximum tag space in bytes.
    pub max_tag_space: u64,

    /// BSPs can be loaded external from the actual BSP tag.
    pub external_bsps: bool,

    /// Models can be loaded external from tag data.
    pub external_models: bool,

    /// Model/BSP data uses lossy compression.
    pub compressed_models: bool,

    /// File offsets/sizes must be modulo this in bytes.
    pub data_alignment: usize,

    /// Additional settings for bitmaps.
    pub bitmap_options: EngineBitmapOptions,

    /// Resource map handling.
    pub resource_maps: Option<EngineSupportedResourceMaps>,

    /// Compression format for all data past the header.
    pub compression_type: EngineCompressionType,

    /// Maximum cache file size, per-scenario type.
    pub max_cache_file_size: EngineCacheFileSize,

    /// Base memory address for tag data.
    pub base_memory_address: EngineBaseMemoryAddress,

    /// All tags required to build the cache file besides the scenario tag.
    pub required_tags: EngineRequiredTags,

    /// Cache parser type.
    pub cache_parser: EngineCacheParser,
    
    /// If true, this uses an obfuscated header layout.
    pub obfuscated_header_layout: bool
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum EngineCacheParser {
    PC,
    Xbox
}

/// Determines how bitmaps are stored.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct EngineBitmapOptions {
    /// Uncompressed textures must be stored swizzled if they are power-of-two.
    pub swizzled: bool,

    /// Texture dimensions, including mipmaps, must be divisible by its block size.
    pub texture_dimension_must_modulo_block_size: bool,

    /// Rather than storing each face on each mipmap, store each face as a separate 2D texture, stored contiguously.
    pub cubemap_faces_stored_separately: bool,

    /// The required alignment for bitmap data.
    ///
    /// This does not override data alignment, but is used for padding within the bitmap data itself.
    pub alignment: usize
}

#[derive(Copy, Clone, Debug)]
pub struct EngineSupportedResourceMaps {
    pub externally_indexed_tags: bool
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum EngineCompressionType {
    Uncompressed,
    Deflate
}

#[derive(Copy, Clone, Debug)]
pub struct EngineRequiredTags {
    pub all: &'static [&'static str],
    pub user_interface: &'static [&'static str],
    pub singleplayer: &'static [&'static str],
    pub multiplayer: &'static [&'static str],
}

#[derive(Copy, Clone, Debug)]
pub struct EngineBuild {
    pub string: &'static str,
    pub aliases: &'static [&'static str],
    pub enforced: bool
}

#[derive(Copy, Clone, Debug)]
pub struct EngineCacheFileSize {
    pub user_interface: u64,
    pub singleplayer: u64,
    pub multiplayer: u64
}

#[derive(Copy, Clone, Debug)]
pub struct EngineBaseMemoryAddress {
    pub address: u64,
    pub inferred: bool
}
