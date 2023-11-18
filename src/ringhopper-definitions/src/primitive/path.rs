use super::*;
use crate::parse::TagData;
use crate::parse::*;
use std::convert::From;
use std::fmt::Display;
use std::fmt::Write;

pub(crate) const WIN32_PATH_SEPARATOR: char = '\\';
pub(crate) const UNIX_PATH_SEPARATOR: char = '/';

pub(crate) const WIN32_PATH_SEPARATOR_STR: &'static str = "\\";
pub(crate) const UNIX_PATH_SEPARATOR_STR: &'static str = "/";

/// Refers to a tag path and provides functions for handling these.
#[derive(Clone, Debug, PartialEq)]
pub struct TagPath {
    /// The path of the tag, not including the extension
    pub path: String,

    /// The group of the tag (also used for determining the file extension)
    pub group: TagGroup
}

impl TagPath {
    /// Return a Win32 path of the tag reference.
    ///
    /// This is what is internally stored in tags.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_definitions::primitive::{TagPath, TagGroup};
    ///
    /// let path = TagPath::from_path("weapons/myweapon/myweapon.weapon")
    ///                       .unwrap();
    ///
    /// assert_eq!(path.to_win32_path(), "weapons\\myweapon\\myweapon.weapon");
    /// ```
    pub fn to_win32_path(&self) -> String {
        format!("{}.{}", self.path, self.group)
    }

    /// Return a Unix path of the tag reference.
    ///
    /// This is useful for creating filesystem paths on Unix-like operating systems.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_definitions::primitive::{TagPath, TagGroup};
    ///
    /// let path = TagPath::from_path("weapons\\myweapon\\myweapon.weapon")
    ///                       .unwrap();
    ///
    /// assert_eq!(path.to_unix_path(), "weapons/myweapon/myweapon.weapon");
    /// ```
    pub fn to_unix_path(&self) -> String {
        self.to_win32_path().replace(WIN32_PATH_SEPARATOR, UNIX_PATH_SEPARATOR_STR)
    }

    /// Construct a tag reference from a path.
    ///
    /// This will accept both Win32 and Unix paths as input.
    ///
    /// Return `None` if the path is not valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_definitions::primitive::{TagPath, TagGroup};
    ///
    /// // Works with Unix paths!
    /// let path = TagPath::from_path("weapons/myweapon/myweapon.isthebest.weapon")
    ///                       .expect("tag path should be valid");
    ///
    /// assert_eq!(path.path, "weapons\\myweapon\\myweapon.isthebest");
    /// assert_eq!(path.group, TagGroup::Weapon);
    ///
    /// // Also works the same with Win32 paths
    /// let path2 = TagPath::from_path("weapons\\myweapon\\myweapon.isthebest.weapon")
    ///                        .expect("tag path should be valid");
    ///
    /// assert_eq!(path2.path, "weapons\\myweapon\\myweapon.isthebest");
    /// assert_eq!(path2.group, TagGroup::Weapon);
    /// assert_eq!(path, path2);
    /// ```
    pub fn from_path(path: &str) -> RinghopperResult<Self> {
        let path = path.replace(UNIX_PATH_SEPARATOR, WIN32_PATH_SEPARATOR_STR);

        let mut offset = path.find('.').ok_or(Error::InvalidTagPath)? + 1;
        let mut extension_maybe = &path[offset..];
        while let Some(offset_delta) = extension_maybe.find('.') {
            offset += offset_delta + 1;
            extension_maybe = &path[offset..];
        }

        let (mut path_without_extension, group) = path.split_at(offset);
        path_without_extension = &path_without_extension[..path_without_extension.len() - 1];
        debug_assert_eq!(extension_maybe, group);

        Ok(Self {
            path: path_without_extension.to_owned(),
            group: TagGroup::from_str(group).map_err(|_| Error::InvalidTagPath)?
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
            if c == WIN32_PATH_SEPARATOR {
                f.write_char(UNIX_PATH_SEPARATOR)?;
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
    /// use ringhopper_definitions::primitive::TagReference;
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
    /// use ringhopper_definitions::primitive::{TagPath, TagReference};
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
    /// use ringhopper_definitions::primitive::{TagPath, TagReference, TagGroup};
    ///
    /// let path = TagPath::from_path("weapons/someweapon/someweapon.weapon").unwrap();
    /// let reference = TagReference::from(path);
    /// assert_eq!(reference.group(), TagGroup::Weapon);
    /// ```
    pub const fn group(&self) -> TagGroup {
        match self {
            Self::Null(g) => *g,
            Self::Set(g) => g.group
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
        4 * <u32 as TagDataSimplePrimitive>::size()
    }

    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        let group = TagGroup::read_from_tag_file(data, at, struct_end, extra_data_cursor)?;
        let len = usize::read_from_tag_file(data, at + 0xC, struct_end, extra_data_cursor)?;

        if len == 0 {
            return Ok(TagReference::Null(group))
        }

        let real_len = len.add_overflow_checked(1)?;
        let start = *extra_data_cursor;
        let end = start.add_overflow_checked(real_len)?;
        fits(real_len, start, data.len())?;

        let str_data = &data[start..end];
        if *str_data.last().unwrap() != 0 {
            return Err(Error::InvalidTagPath)
        }

        Ok(TagReference::Set(
            TagPath {
                path: std::str::from_utf8(&data[start..end-1]).map_err(|_| Error::InvalidTagPath)?.to_owned(),
                group
            }
        ))
    }

    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        match self {
            TagReference::Null(group) => {
                group.write_to_tag_file(data, at, struct_end)?;
                (0usize).write_to_tag_file(data, at + 0x8, struct_end)?;
            },
            TagReference::Set(path) => {
                path.group.write_to_tag_file(data, at, struct_end)?;
                path.path.len().write_to_tag_file(data, at + 0x8, struct_end)?;
                data.extend_from_slice(path.path.as_bytes());
                data.push(0x00);
            }
        }

        Ok(())
    }
}
