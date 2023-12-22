use super::*;
use crate::parse::*;
use std::convert::From;
use std::fmt::Display;
use std::fmt::Write;

/// Halo path separator
pub(crate) const HALO_PATH_SEPARATOR: char = '\\';

/// Refers to a tag path and provides functions for handling these.
#[derive(Clone, Debug, PartialEq)]
pub struct TagPath {
    /// The path of the tag, not including the extension
    pub(crate) path: String,

    /// The group of the tag (also used for determining the file extension)
    pub(crate) group: TagGroup
}

impl TagPath {
    /// Get the path component of the tag path.
    ///
    /// This will be the path as internally stored in tags.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{TagPath, TagGroup};
    ///
    /// let path = TagPath::from_path("weapons\\myweapon\\myweapon.weapon")
    ///                       .unwrap();
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
        let mut offset = path.find('.').ok_or(Error::InvalidTagPath)? + 1;
        let mut extension_maybe = &path[offset..];
        while let Some(offset_delta) = extension_maybe.find('.') {
            offset += offset_delta + 1;
            extension_maybe = &path[offset..];
        }

        let (mut path_without_extension, group) = path.split_at(offset);
        path_without_extension = &path_without_extension[..path_without_extension.len() - 1];
        debug_assert_eq!(extension_maybe, group);

        let group = TagGroup::from_str(group).map_err(|_| Error::InvalidTagPath)?;
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

        for c in path.chars() {
            path_fixed.push(match c {
                c if std::path::is_separator(c) => HALO_PATH_SEPARATOR,
                '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => return Err(Error::InvalidTagPath),
                c if c.is_ascii_control() => return Err(Error::InvalidTagPath),
                c => c
            });
        }

        Ok(Self {
            path: path_fixed.to_owned(), group
        })
    }
}

#[cfg(target_family="windows")]
impl Display for TagPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    /// use ringhopper_primitives::primitive::TagReference;
    ///
    /// let path = TagReference::default();
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
    /// let path = TagPath::from_path("weapons/someweapon/someweapon.weapon").unwrap();
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
    /// let path = TagPath::from_path("weapons/someweapon/someweapon.weapon").unwrap();
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
    /// let path = TagPath::from_path("weapons/someweapon/someweapon.weapon").unwrap();
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

impl Default for TagReference {
    fn default() -> Self {
        TagReference::Null(TagGroup::none())
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

        let group = TagGroup::from_fourcc(c_primitive.tag_group)?;
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
            ID::from_u32(tag_id_int)?
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
