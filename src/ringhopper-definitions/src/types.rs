use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use serde_json::Value;

#[derive(Default)]
pub struct ParsedDefinitions {
    pub objects: HashMap<String, NamedObject>,
    pub groups: HashMap<String, TagGroup>,
    pub engines: HashMap<String, Engine>
}

pub trait SizeableObject {
    /// Get the size of the object in bytes
    fn size(&self, parsed_tag_data: &ParsedDefinitions) -> usize;
}

#[derive(Clone)]
pub enum NamedObject {
    Struct(Struct),
    Enum(Enum),
    Bitfield(Bitfield)
}

impl SizeableObject for NamedObject {
    fn size(&self, parsed_tag_data: &ParsedDefinitions) -> usize {
        match self {
            NamedObject::Bitfield(b) => b.size(parsed_tag_data),
            NamedObject::Enum(e) => e.size(parsed_tag_data),
            NamedObject::Struct(s) => s.size(parsed_tag_data)
        }
    }
}

impl NamedObject {
    pub fn name(&self) -> &str {
        match self {
            Self::Struct(s) => s.name.as_str(),
            Self::Enum(e) => e.name.as_str(),
            Self::Bitfield(b) => b.name.as_str(),
        }
    }
}

pub struct TagGroup {
    pub name: String,
    pub struct_name: String,
    pub supergroup: Option<String>,
    pub supported_engines: SupportedEngines,
    pub version: u16,
    pub fourcc_binary: u32
}

#[derive(Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<StructField>,
    pub flags: Flags,

    /// The final size of the struct in bytes
    pub size: usize
}

impl SizeableObject for Struct {
    fn size(&self, _: &ParsedDefinitions) -> usize {
        self.size
    }
}

impl Struct {
    fn set_offsets_and_verify_sizes(&mut self, parsed_tag_data: &ParsedDefinitions) {
        let expected_size = self.size;
        let mut real_size = 0;
        for f in &mut self.fields {
            f.relative_offset = real_size;
            real_size += f.size(parsed_tag_data);
        }
        assert_eq!(expected_size, real_size, "Size for {name} is incorrect (expected {expected_size}, got {real_size} instead)", name=self.name);
        assert_eq!(expected_size, self.size(parsed_tag_data), "size() is implemented wrong for {name} (expected {expected_size}, got {real_size} instead)", name=self.name);
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum LimitType {
    /// Maximum allowed by the engine
    Engine(String),

    /// Maximum allowed by default
    Default,

    /// Maximum allowed by the editor
    Editor
}

#[derive(Clone)]
pub struct StructField {
    /// Name of the field
    pub name: String,

    /// Type of field
    pub field_type: StructFieldType,

    /// Is this a default value? If so, what are the default values for each field.
    pub default_value: Option<Vec<StaticValue>>,

    /// Number of fields
    pub count: FieldCount,

    /// Minimum value
    pub minimum: Option<StaticValue>,

    /// Maximum value
    pub maximum: Option<StaticValue>,

    /// Limits
    pub limit: Option<HashMap<LimitType, usize>>,

    /// Flags
    pub flags: Flags,

    /// Relative offset to the start of its structs.
    pub relative_offset: usize
}

impl SizeableObject for StructField {
    fn size(&self, parsed_tag_data: &ParsedDefinitions) -> usize {
        self.field_type.size(parsed_tag_data) * self.count.field_count()
    }
}

#[derive(Clone)]
pub struct EditorSectionData {
    pub heading: String,
    pub body: Option<String>
}

#[derive(Clone)]
pub enum StructFieldType {
    Object(ObjectType),
    Padding(usize),
    EditorSection(EditorSectionData)
}

impl SizeableObject for StructFieldType {
    fn size(&self, parsed_tag_data: &ParsedDefinitions) -> usize {
        match self {
            StructFieldType::Object(o) => o.size(parsed_tag_data),
            StructFieldType::Padding(u) => *u,
            StructFieldType::EditorSection(_) => 0
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum FieldCount {
    /// A single field
    One,

    /// Expands to from/to
    Bounds,

    /// Array of multiple fields
    Array(usize)
}

impl FieldCount {
    fn field_count(&self) -> usize {
        match self {
            Self::One => 1,
            Self::Bounds => 2,
            Self::Array(u) => *u
        }
    }
}

pub struct DefaultBehavior {
    /// Default values for each field.
    ///
    /// For bounds, this is the \[from,to\]. For arrays, this is for each array element.
    pub default_value: Vec<StaticValue>,

    /// Default if the tag is being created
    pub default_on_creation: bool,

    /// Default if the value is equal to zero and being built into a cache file
    pub default_on_cache: bool
}

#[derive(Debug, Clone)]
pub enum StaticValue {
    F32(f32),
    Uint(u64),
    Int(i64),
    String(String)
}

impl Display for StaticValue {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StaticValue::String(s) => fmt.write_fmt(format_args!("\"{s}\"")),
            StaticValue::Uint(i) => fmt.write_fmt(format_args!("{i}")),
            StaticValue::Int(i) => fmt.write_fmt(format_args!("{i}")),
            StaticValue::F32(f) => fmt.write_fmt(format_args!("{f:0.032}f32"))
        }
    }
}

#[derive(Clone)]
pub struct Bitfield {
    /// Name of the bitfield
    pub name: String,

    /// Width in bits
    pub width: u8,

    /// Fields for the bitfield
    pub fields: Vec<Field>,

    /// Flags! Capture all of them to win!
    pub flags: Flags
}

impl SizeableObject for Bitfield {
    fn size(&self, _: &ParsedDefinitions) -> usize {
        (self.width / 8) as usize
    }
}

#[derive(Clone)]
pub struct Enum {
    pub name: String,
    pub options: Vec<Field>,
    pub flags: Flags
}

impl SizeableObject for Enum {
    fn size(&self, _: &ParsedDefinitions) -> usize {
        std::mem::size_of::<u16>()
    }
}

#[derive(Clone)]
pub struct Field {
    pub name: String,
    pub flags: Flags,
    pub value: u32
}

#[derive(Clone)]
pub struct SupportedEngines {
    pub supported_engines: Option<Vec<String>>
}

impl Default for SupportedEngines {
    fn default() -> Self {
        Self {
            supported_engines: None
        }
    }
}

/// General fields. Some may be applicable to some objects, but not all.
#[derive(Default, Clone)]
pub struct Flags {
    /// This field is not readable from tag files
    pub cache_only: bool,

    /// This field is not present in cache files
    pub non_cached: bool,

    /// Hint to the editor it should be read-only by default
    pub uneditable_in_editor: bool,

    /// Hint to the editor it should not be displayed by default
    pub hidden_in_editor: bool,

    /// The field cannot be used; if it is set, it will be lost
    pub exclude: bool,

    /// Store in little endian in tag format
    pub little_endian_in_tags: bool,

    /// The value is subtracted by 1 when put into a cache file (and incremented by 1 if extracted).
    pub shifted_by_one: bool,

    /// The value must be set.
    pub non_null: bool,

    /// Supported engines for the field
    pub supported_engines: SupportedEngines,

    /// Any comment, if present
    pub comment: Option<String>,

    /// Any developer note, if present
    pub developer_note: Option<String>,

    /// Any description, if present
    pub description: Option<String>
}

impl Flags {
    pub(crate) fn combine_with(&mut self, other: &Flags) {
        self.cache_only |= other.cache_only;
        self.non_cached |= other.non_cached;
        self.uneditable_in_editor |= other.uneditable_in_editor;
        self.hidden_in_editor |= other.hidden_in_editor;
        self.exclude |= other.exclude;
        self.little_endian_in_tags |= other.little_endian_in_tags;
        self.shifted_by_one |= other.shifted_by_one;
        self.non_null |= other.non_null;
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum EngineCacheParser {
    Xbox,
    PC
}

pub struct Engine {
    pub name: String,
    pub display_name: String,
    pub version: Option<String>,
    pub build: Option<Build>,
    pub inherits: Option<String>,
    pub build_target: bool,
    /// NOTE: This property is set explicitly per engine.
    pub fallback: bool,
    pub custom: bool,
    pub cache_file_version: u32,
    /// NOTE: This property is set explicitly per engine.
    pub cache_default: bool,
    pub external_bsps: bool,
    pub external_models: bool,
    pub max_script_nodes: u64,
    pub max_tag_space: u64,
    pub compressed_models: bool,
    pub data_alignment: u64,
    pub obfuscated_header_layout: bool,
    pub bitmap_options: EngineBitmapOptions,
    pub resource_maps: Option<EngineSupportedResourceMaps>,
    pub cache_parser: EngineCacheParser,
    pub max_cache_file_size: EngineCacheFileSize,
    pub base_memory_address: BaseMemoryAddress,
    pub required_tags: EngineRequiredTags,
    pub compression_type: EngineCompressionType,
}

pub enum EngineCompressionType {
    Uncompressed,
    Deflate
}

pub struct EngineSupportedResourceMaps {
    pub externally_indexed_tags: bool,
    pub loc: bool
}

pub struct EngineCacheFileSize {
    pub user_interface: u64,
    pub singleplayer: u64,
    pub multiplayer: u64
}

#[derive(Default)]
pub struct EngineRequiredTags {
    pub all: Vec<String>,
    pub user_interface: Vec<String>,
    pub singleplayer: Vec<String>,
    pub multiplayer: Vec<String>
}

pub struct BaseMemoryAddress {
    pub address: u64,
    pub inferred: bool
}

pub struct Build {
    pub string: String,
    pub fallback: Vec<String>,
    pub enforced: bool
}

#[derive(Clone)]
pub struct TagReference {
    pub allowed_groups: Vec<String>
}

pub struct Reflexive {
    pub struct_name: String
}

pub struct EngineBitmapOptions {
    pub swizzled: bool,
    pub texture_dimension_must_modulo_block_size: bool,
    pub cubemap_faces_stored_separately: bool,
    pub alignment: u64
}

#[derive(Clone)]
pub enum ObjectType {
    NamedObject(String),
    Reflexive(String),
    TagReference(TagReference),
    TagGroup,
    Data,
    FileData,
    BSPVertexData,
    F32,
    U8,
    U16,
    U32,
    I8,
    I16,
    I32,
    TagID,
    ID,
    Index,
    Angle,
    Address,
    Vector2D,
    Vector3D,
    CompressedVector2D,
    CompressedVector3D,
    CompressedFloat,
    Vector2DInt,
    Plane2D,
    Plane3D,
    Euler2D,
    Euler3D,
    Rectangle,
    Quaternion,
    Matrix3x3,
    ColorRGBFloat,
    ColorARGBFloat,
    ColorARGBInt,
    String32,
    ScenarioScriptNodeValue,
    UTF16String,
}

impl ObjectType {
    const fn primitive_size(&self) -> usize {
        match self {
            Self::Reflexive(_) => 0xC,
            Self::TagReference(_) => 0x10,
            Self::Data | Self::FileData | Self::BSPVertexData | Self::UTF16String => 0x14,
            Self::F32
            | Self::Angle
            | Self::U32
            | Self::Address
            | Self::I32
            | Self::ColorARGBInt
            | Self::ID
            | Self::TagID
            | Self::CompressedVector2D
            | Self::CompressedVector3D => 0x4,
            Self::U16 | Self::I16 | Self::Index | Self::CompressedFloat => 0x2,
            Self::U8 | Self::I8 => 0x1,
            Self::Rectangle | Self::Vector2DInt => Self::I16.primitive_size() * self.composite_count(),
            Self::ScenarioScriptNodeValue => 0x4,
            Self::TagGroup => 0x4,
            Self::Vector2D
            | Self::Vector3D
            | Self::Plane2D
            | Self::Plane3D
            | Self::Quaternion
            | Self::Matrix3x3
            | Self::ColorRGBFloat
            | Self::Euler2D
            | Self::Euler3D
            | Self::ColorARGBFloat => ObjectType::F32.primitive_size() * self.composite_count(),
            Self::String32 => 32,

            Self::NamedObject(_) => unreachable!()
        }
    }

    const fn composite_count(&self) -> usize {
        match self {
            Self::Reflexive(_) => 1,
            Self::TagReference(_) => 1,
            Self::NamedObject(_) => 1,
            Self::Data | Self::FileData | Self::BSPVertexData | Self::UTF16String => 1,
            Self::TagID | Self::ID => 1,
            Self::TagGroup => 1,
            Self::F32 | Self::Angle | Self::U32 | Self::Address | Self::I32 | Self::ColorARGBInt | Self::CompressedVector2D | Self::CompressedVector3D | Self::CompressedFloat => 1,
            Self::U16 | Self::I16 | Self::Index => 1,
            Self::U8 | Self::I8 => 1,
            Self::Rectangle => 4,
            Self::Vector2D => 2,
            Self::Vector3D => 3,
            Self::Euler2D => 2,
            Self::Euler3D => 3,
            Self::Plane2D => 3,
            Self::Plane3D => 4,
            Self::Quaternion => 4,
            Self::Vector2DInt => 2,
            Self::Matrix3x3 => 3 * 3,
            Self::ColorRGBFloat => 3,
            Self::ColorARGBFloat => 4,
            Self::String32 => 1,
            Self::ScenarioScriptNodeValue => 1,
        }
    }

    const fn primitive_value_type(&self) -> Option<StaticValue> {
        match self {
            Self::NamedObject(_)
            | Self::Data
            | Self::FileData
            | Self::BSPVertexData
            | Self::UTF16String
            | Self::TagID
            | Self::ID
            | Self::Address
            | Self::ScenarioScriptNodeValue
            | Self::TagGroup
            | Self::CompressedVector2D
            | Self::CompressedVector3D
            | Self::CompressedFloat => None,

            Self::TagReference(_)
            | Self::String32 => Some(StaticValue::String(String::new())),

            Self::U8
            | Self::U16
            | Self::Index
            | Self::U32
            | Self::ColorARGBInt
            | Self::Reflexive(_) => Some(StaticValue::Uint(0)),

            Self::I8
            | Self::I16
            | Self::I32
            | Self::Rectangle
            | Self::Vector2DInt => Some(StaticValue::Int(0)),

            Self::F32
            | Self::Angle
            | Self::Vector2D
            | Self::Vector3D
            | Self::Plane2D
            | Self::Plane3D
            | Self::Euler2D
            | Self::Euler3D
            | Self::Quaternion
            | Self::Matrix3x3
            | Self::ColorRGBFloat
            | Self::ColorARGBFloat => Some(StaticValue::F32(0.0)),
        }
    }
}

impl SizeableObject for ObjectType {
    fn size(&self, parsed_tag_data: &ParsedDefinitions) -> usize {
        match self {
            Self::NamedObject(p) => parsed_tag_data.objects.get(p).unwrap().size(parsed_tag_data),
            _ => self.primitive_size()
        }
    }
}

mod parse;
pub(crate) use parse::*;
