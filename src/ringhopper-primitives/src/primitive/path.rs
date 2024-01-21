use std::any::Any;
use std::cmp::Ordering;
use super::*;
use crate::parse::*;
use std::convert::From;
use std::fmt::Display;
use std::fmt::Write;
use crate::dynamic::{DynamicTagData, DynamicTagDataType};

/// Halo path separator
pub(crate) const HALO_PATH_SEPARATOR: char = '\\';

/// Refers to a tag path and provides functions for handling these.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TagPath {
    /// The path of the tag, not including the extension
    pub(crate) path: String,

    /// The group of the tag (also used for determining the file extension)
    pub(crate) group: TagGroup
}

impl Ord for TagPath {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.path.cmp(&other.path) {
            Ordering::Equal => self.group.cmp(&other.group),
            n => n
        }
    }
}

impl PartialOrd for TagPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl TagPath {
    /// Split a string path by its tag group, if one is valid.
    ///
    /// Returns `None` if `string_path` has no extension that corresponds to a valid tag group.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{TagPath, TagGroup};
    ///
    /// let (path, group) = TagPath::split_str_path("weapons\\myweapon\\myweapon.weapon").unwrap();
    /// assert_eq!("weapons\\myweapon\\myweapon", path);
    /// assert_eq!(TagGroup::Weapon, group);
    pub fn split_str_path(string_path: &str) -> Option<(&str, TagGroup)> {
        let path_extension_index = string_path.rfind('.')?;
        let (path, extension) = string_path.split_at(path_extension_index);
        let tag_group = TagGroup::from_str(&extension[1..]).ok()?;
        Some((path, tag_group))
    }

    /// Get the path component of the tag path.
    ///
    /// This will be the path as internally stored in tags, using Halo path separators.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{TagPath, TagGroup};
    ///
    /// let path = TagPath::from_path("weapons\\myweapon\\myweapon.weapon").unwrap();
    ///
    /// assert_eq!("weapons\\myweapon\\myweapon", path.path());
    /// ```
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the group component of the tag path.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{TagPath, TagGroup};
    ///
    /// let path = TagPath::from_path("weapons\\myweapon\\myweapon.weapon")
    ///                       .unwrap();
    ///
    /// assert_eq!(TagGroup::Weapon, path.group());
    /// ```
    pub const fn group(&self) -> TagGroup {
        self.group
    }

    /// Return an internal path of the tag reference.
    ///
    /// This is what is internally stored in tags.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::TagPath;
    ///
    /// let path = TagPath::from_path("weapons\\myweapon\\myweapon.weapon")
    ///                       .unwrap();
    ///
    /// assert_eq!(path.to_internal_path(), "weapons\\myweapon\\myweapon.weapon");
    /// ```
    pub fn to_internal_path(&self) -> String {
        format!("{}.{}", self.path, self.group)
    }

    /// Return a native path of the tag reference.
    ///
    /// This is useful for creating filesystem paths on the native OS.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::TagPath;
    /// use std::path::Path;
    /// use std::ffi::OsStr;
    ///
    /// let path = TagPath::from_path("weapons\\myweapon\\myweapon.weapon")
    ///                       .unwrap();
    /// let native_path = path.to_native_path();
    ///
    /// let std_path = Path::new(&native_path);
    /// let components: Vec<&OsStr> = std_path.iter().collect();
    ///
    /// assert_eq!(vec!["weapons", "myweapon", "myweapon.weapon"], components);
    /// ```
    pub fn to_native_path(&self) -> String {
        self.to_internal_path().replace(HALO_PATH_SEPARATOR, std::path::MAIN_SEPARATOR_STR)
    }

    /// Construct a tag reference from a path.
    ///
    /// This will accept both Halo (i.e. `\`) and native paths as input.
    ///
    /// Return `None` if the path is not valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{TagPath, TagGroup};
    ///
    /// #[cfg(any(target_family = "unix", target_family = "windows"))]
    /// {
    ///     // Works with Unix paths if on a Unix-like OS or Windows!
    ///     let path = TagPath::from_path("weapons/myweapon/myweapon.isthebest.weapon")
    ///                           .expect("tag path should be valid");
    ///
    ///     assert_eq!(path.path(), "weapons\\myweapon\\myweapon.isthebest");
    ///     assert_eq!(path.group(), TagGroup::Weapon);
    /// }
    ///
    /// // Works with Halo paths
    /// let path = TagPath::from_path("weapons\\myweapon\\myweapon.isthebest.weapon")
    ///                        .expect("tag path should be valid");
    ///
    /// assert_eq!(path.path(), "weapons\\myweapon\\myweapon.isthebest");
    /// assert_eq!(path.group(), TagGroup::Weapon);
    /// ```
    pub fn from_path(path: &str) -> RinghopperResult<Self> {
        let (path_without_extension, group) = Self::split_str_path(path).ok_or(Error::InvalidTagFile)?;
        Self::new(path_without_extension, group)
    }

    /// Construct a tag reference from a path with separate path and group components.
    ///
    /// This will accept both Halo (i.e. `\`) and native paths as input.
    ///
    /// Return `None` if the path is not valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{TagPath, TagGroup};
    ///
    /// #[cfg(any(target_family = "unix", target_family = "windows"))]
    /// {
    ///     // Works with Unix paths if on a Unix-like OS or Windows!
    ///     let path = TagPath::new("weapons/myweapon/myweapon.isthebest", TagGroup::Weapon)
    ///                     .expect("tag path should be valid");
    ///
    ///     assert_eq!(path.path(), "weapons\\myweapon\\myweapon.isthebest");
    ///     assert_eq!(path.group(), TagGroup::Weapon);
    /// }
    ///
    /// // Works with Halo paths
    /// let path = TagPath::new("weapons\\myweapon\\myweapon.isthebest", TagGroup::Weapon)
    ///                 .expect("tag path should be valid");
    ///
    /// assert_eq!(path.path(), "weapons\\myweapon\\myweapon.isthebest");
    /// assert_eq!(path.group(), TagGroup::Weapon);
    /// ```
    pub fn new(path: &str, group: TagGroup) -> RinghopperResult<Self> {
        let mut path_fixed = String::with_capacity(path.len());
        let mut last_char = '\x00';

        for c in path.chars() {
            let character = match c {
                c if std::path::is_separator(c) => HALO_PATH_SEPARATOR,
                '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => return Err(Error::InvalidTagPath),
                c if c.is_ascii_control() => return Err(Error::InvalidTagPath),
                c => c.to_ascii_lowercase()
            };

            // Ignore double path separators
            if last_char == HALO_PATH_SEPARATOR && character == HALO_PATH_SEPARATOR {
                continue
            }

            last_char = character;
            path_fixed.push(last_char);
        }

        // Cannot start at the root
        if path_fixed.starts_with(HALO_PATH_SEPARATOR) {
            return Err(Error::InvalidTagPath)
        }

        // Check for problematic directory names for Windows
        const BANNED_DIRECTORY_NAMES: &'static [&'static str] = &[
            "aux",
            "com0",
            "com1",
            "com2",
            "com3",
            "com4",
            "com5",
            "com6",
            "com7",
            "com8",
            "com9",
            "con",
            "lpt0",
            "lpt1",
            "lpt2",
            "lpt3",
            "lpt4",
            "lpt5",
            "lpt6",
            "lpt7",
            "lpt8",
            "lpt9",
            "nul",
            "prn",
        ];

        if path_fixed.split(HALO_PATH_SEPARATOR).any(|dir| dir.ends_with('.') || BANNED_DIRECTORY_NAMES.binary_search(&dir).is_ok()) {
            return Err(Error::InvalidTagPath)
        }

        Ok(Self {
            path: path_fixed.to_owned(), group
        })
    }
}

#[cfg(target_family="windows")]
impl Display for TagPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // We can print the path as-is on Windows
        const _: () = assert!(std::path::MAIN_SEPARATOR == HALO_PATH_SEPARATOR);

        f.write_str(&self.path)?;
        f.write_char('.')?;
        f.write_str(self.group.as_str())?;
        Ok(())
    }
}

#[cfg(not(target_family="windows"))]
impl Display for TagPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in self.path.chars() {
            if c == HALO_PATH_SEPARATOR {
                f.write_char(std::path::MAIN_SEPARATOR)?;
            }
            else {
                f.write_char(c)?;
            }
        }
        f.write_char('.')?;
        f.write_str(self.group.as_str())?;
        Ok(())
    }
}

/// Refers to a reference to a tag and its tag path.
///
/// When set, a tag path is present. When unset, a group is still present.
///
/// Note that, in some cases, a tag group is still needed, thus the `Option` type isn't usable here.
#[derive(Clone, Debug, PartialEq)]
pub enum TagReference {
    /// Refers to a reference that is set to a path.
    Set(TagPath),

    /// Refers to a reference that isn't set to a path. Note that all references have groups, though.
    Null(TagGroup)
}

impl TagReference {
    /// Return `true` if the reference is null.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{TagGroup, TagReference};
    ///
    /// let path = TagReference::Null(TagGroup::Model);
    /// assert!(path.is_null());
    /// ```
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null(_))
    }

    /// Return `true` if the reference is set.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{TagPath, TagReference};
    ///
    /// let path = TagPath::from_path("weapons\\someweapon\\someweapon.weapon").unwrap();
    /// let reference = TagReference::from(path);
    /// assert!(reference.is_set());
    /// ```
    pub const fn is_set(&self) -> bool {
        matches!(self, Self::Set(_))
    }

    /// Return the tag group of the reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{TagPath, TagReference, TagGroup};
    ///
    /// let path = TagPath::from_path("weapons\\someweapon\\someweapon.weapon").unwrap();
    /// let reference = TagReference::from(path);
    /// assert_eq!(reference.group(), TagGroup::Weapon);
    /// ```
    pub const fn group(&self) -> TagGroup {
        match self {
            Self::Null(g) => *g,
            Self::Set(p) => p.group
        }
    }

    /// Return the tag path of the reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{TagPath, TagReference, TagGroup};
    ///
    /// let path = TagPath::from_path("weapons\\someweapon\\someweapon.weapon").unwrap();
    /// let reference = TagReference::from(path.clone());
    /// assert_eq!(*reference.path().expect("should be a path here"), path);
    /// ```
    pub const fn path(&self) -> Option<&TagPath> {
        match self {
            Self::Null(_) => None,
            Self::Set(p) => Some(&p)
        }
    }
}

impl From<TagPath> for TagReference {
    fn from(item: TagPath) -> TagReference {
        Self::Set(item)
    }
}

impl Display for TagReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Set(s) => s.fmt(f)?,
            Self::Null(g) => {
                f.write_str("(null ")?;
                g.fmt(f)?;
                f.write_char(')')?;
            }
        }
        Ok(())
    }
}

impl TagData for TagReference {
    fn size() -> usize {
        <TagReferenceC as TagDataSimplePrimitive>::size()
    }

    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        let c_primitive = TagReferenceC::read_from_tag_file(data, at, struct_end, extra_data_cursor)?;

        let group = c_primitive.tag_group.try_into()?;
        let len = c_primitive.path_length as usize;

        if len == 0 {
            return Ok(TagReference::Null(group))
        }

        let real_len = len.add_overflow_checked(1)?;
        let start = *extra_data_cursor;
        let end = start.add_overflow_checked(real_len)?;
        let null_byte_index = end-1;
        fits(real_len, start, data.len())?;

        if data[null_byte_index] != 0 {
            return Err(Error::InvalidTagPath)
        }

        *extra_data_cursor = end;

        Ok(TagReference::Set(
            TagPath {
                path: std::str::from_utf8(&data[start..null_byte_index]).map_err(|_| Error::InvalidTagPath)?.to_owned(),
                group
            }
        ))
    }

    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        let construct_to_write = match self {
            TagReference::Null(group) => {
                TagReferenceC {
                    tag_group: group.as_fourcc(),
                    ..Default::default()
                }
            },
            TagReference::Set(path) => {
                data.extend_from_slice(path.path.as_bytes());
                data.push(0x00);
                TagReferenceC {
                    tag_group: path.group.as_fourcc(),
                    path_length: path.path.len().into_u32()?,
                    ..Default::default()
                }
            }
        };
        construct_to_write.write_to_tag_file(data, at, struct_end)
    }
}

impl DynamicTagData for TagReference {
    fn get_field(&self, _field: &str) -> Option<&dyn DynamicTagData> {
        None
    }

    fn get_field_mut(&mut self, _field: &str) -> Option<&mut dyn DynamicTagData> {
        None
    }

    fn fields(&self) -> &'static [&'static str] {
        &[]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn data_type(&self) -> DynamicTagDataType {
        DynamicTagDataType::TagReference
    }
}

/// Lower level C implementation of a tag reference
#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct TagReferenceC {
    pub tag_group: FourCC,
    pub path_address: Address,
    pub path_length: u32,
    pub tag_id: ID
}
impl TagDataSimplePrimitive for TagReferenceC {
    fn size() -> usize {
        std::mem::size_of::<Self>()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        let tag_group = FourCC::read::<B>(data, at, struct_end)?;
        let path_address = Address::read::<B>(data, at + 0x4, struct_end)?;
        let path_length = u32::read::<B>(data, at + 0x8, struct_end)?;

        let tag_id_int = u32::read::<B>(data, at + 0xC, struct_end)?;
        let tag_id = if tag_id_int == 0 {
            ID::null()
        }
        else {
            ID::from_u32(tag_id_int)
        };

        Ok(Self { tag_group, path_address, path_length, tag_id })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.tag_group.write::<B>(data, at, struct_end)?;
        self.path_address.write::<B>(data, at + 0x4, struct_end)?;
        self.path_length.write::<B>(data, at + 0x8, struct_end)?;
        self.tag_id.write::<B>(data, at + 0xC, struct_end)?;
        Ok(())
    }
}

#[cfg(test)]
mod test;
