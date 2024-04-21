//! Error-handling for methods that can fail in Ringhopper.

use std::borrow::Cow;
use std::fmt::Display;
use std::path::PathBuf;
use crate::primitive::TagPath;

/// General Result type for Ringhopper that uses [`Error`].
pub type RinghopperResult<T> = Result<T, Error>;

/// General error type for Ringhopper.
#[derive(Debug)]
pub enum Error {
    InvalidFourCC,
    InvalidTagPath,
    InvalidID,
    InvalidEnum,
    InvalidTagFile,
    TagParseFailure(String),
    MapParseFailure(String),
    FailedToReadTag(TagPath, Vec<Error>),
    TagHeaderGroupTypeMismatch,
    TagHeaderGroupVersionMismatch,
    TagGroupUnimplemented,
    ChecksumMismatch,
    ArrayLimitExceeded,
    IndexLimitExceeded,
    SizeLimitExceeded,
    String32SizeLimitExceeded,
    TagNotFound(TagPath),
    /// First one is the tag with the dependency, and the second one is the tag being depended
    BrokenDependency(TagPath, TagPath),
    FailedToReadFile(PathBuf, std::io::Error),
    FailedToWriteFile(PathBuf, std::io::Error),
    InvalidTagsDirectory,
    MapDataOutOfBounds(String),
    InvalidTagData(String),
    Other(String)
}

impl Error {
    /// Get the string representation of the error.
    pub fn as_str(&self) -> Cow<'static, str> {
        match self {
            Error::InvalidTagPath => Cow::Borrowed("invalid tag path"),
            Error::InvalidID => Cow::Borrowed("invalid ID"),
            Error::InvalidEnum => Cow::Borrowed("invalid enum value"),
            Error::InvalidFourCC => Cow::Borrowed("invalid tag group FourCC"),
            Error::InvalidTagFile => Cow::Borrowed("tag file is invalid (bad header)"),
            Error::TagParseFailure(reason) => Cow::Owned(format!("failed to parse the tag (tag is likely corrupt): {reason}")),
            Error::MapParseFailure(reason) => Cow::Owned(format!("failed to parse the map: {reason}")),
            Error::FailedToReadTag(tag, error) => {
                match error.len() {
                    0 => Cow::Owned(format!("tag `{tag}` could not be read")),
                    1 => Cow::Owned(format!("tag `{tag}` could not be read: {}", error[0])),
                    _ => {
                        let mut error_msg = error[0].as_str().to_string();
                        for i in error.iter().skip(1) {
                            error_msg += "\", \"";
                            error_msg += &i.as_str()
                        }
                        Cow::Owned(format!("tag `{tag}` could not be read: [\"{error_msg}\"]"))
                    }
                }
            },
            Error::TagHeaderGroupTypeMismatch => Cow::Borrowed("failed to parse the tag due to it being the wrong group"),
            Error::TagHeaderGroupVersionMismatch => Cow::Borrowed("failed to parse the tag due to it being the wrong group version"),
            Error::TagGroupUnimplemented => Cow::Borrowed("tag group is unimplemented"),
            Error::ChecksumMismatch => Cow::Borrowed("refused to parse the data (CRC32 mismatch)"),
            Error::SizeLimitExceeded => Cow::Borrowed("usize limit exceeded"),
            Error::ArrayLimitExceeded => Cow::Borrowed("array limit of 0xFFFFFFFF (4294967295) exceeded"),
            Error::IndexLimitExceeded => Cow::Borrowed("index limit of 0xFFFE (65534) exceeded"),
            Error::String32SizeLimitExceeded => Cow::Borrowed("string data is longer than 31 characters"),
            Error::TagNotFound(tag) => Cow::Owned(format!("tag `{tag}` not found")),
            Error::BrokenDependency(tag, dependent) => Cow::Owned(format!("tag `{tag}` has a broken dependency: `{dependent}` was not found")),
            Error::FailedToReadFile(file, err) => Cow::Owned(format!("failed to read file `{}`: {err}", file.display())),
            Error::FailedToWriteFile(file, err) => Cow::Owned(format!("failed to write file `{}`: {err}", file.display())),
            Error::MapDataOutOfBounds(explanation) => Cow::Owned(format!("map data out of bounds: {explanation}")),
            Error::InvalidTagData(explanation) => Cow::Owned(format!("invalid tag data: `{explanation}`")),
            Error::InvalidTagsDirectory => Cow::Borrowed("invalid tags directory"),
            Error::Other(explanation) => Cow::Owned(explanation.to_owned())
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_str())
    }
}

/// Used for enforcing overflow checks for usize to prevent unexpected behavior even on release builds
pub trait OverflowCheck: Sized {
    fn add_overflow_checked(self, other: Self) -> RinghopperResult<Self>;
    fn sub_overflow_checked(self, other: Self) -> RinghopperResult<Self>;
    fn mul_overflow_checked(self, other: Self) -> RinghopperResult<Self>;
}

impl OverflowCheck for usize {
    fn add_overflow_checked(self, other: Self) -> RinghopperResult<Self> {
        self.checked_add(other).ok_or(Error::SizeLimitExceeded)
    }
    fn sub_overflow_checked(self, other: Self) -> RinghopperResult<Self> {
        self.checked_sub(other).ok_or(Error::SizeLimitExceeded)
    }
    fn mul_overflow_checked(self, other: Self) -> RinghopperResult<Self> {
        self.checked_mul(other).ok_or(Error::SizeLimitExceeded)
    }
}
