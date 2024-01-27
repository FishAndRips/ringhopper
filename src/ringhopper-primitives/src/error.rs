//! Error-handling for methods that can fail in Ringhopper.

use std::borrow::Cow;
use std::fmt::Display;
use crate::primitive::TagPath;

/// General Result type for Ringhopper that uses [`Error`].
pub type RinghopperResult<T> = Result<T, Error>;

/// General error type for Ringhopper.
#[derive(Clone, PartialEq, Debug)]
pub enum Error {
    InvalidFourCC,
    InvalidTagPath,
    InvalidID,
    InvalidEnum,
    InvalidTagFile,
    TagParseFailure,
    CorruptedTag(TagPath),
    TagHeaderGroupTypeMismatch,
    TagHeaderGroupVersionMismatch,
    TagGroupUnimplemented,
    ChecksumMismatch,
    ArrayLimitExceeded,
    IndexLimitExceeded,
    SizeLimitExceeded,
    String32SizeLimitExceeded,
    TagNotFound(TagPath),
    FailedToReadFile,
    FailedToWriteFile,
    InvalidTagsDirectory
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
            Error::TagParseFailure => Cow::Borrowed("failed to parse the tag (tag is likely corrupt)"),
            Error::CorruptedTag(tag) => Cow::Owned(format!("tag `{tag}` is unreadable and/or corrupt")),
            Error::TagHeaderGroupTypeMismatch => Cow::Borrowed("failed to parse the tag due to it being the wrong group"),
            Error::TagHeaderGroupVersionMismatch => Cow::Borrowed("failed to parse the tag due to it being the wrong group version"),
            Error::TagGroupUnimplemented => Cow::Borrowed("tag group is unimplemented"),
            Error::ChecksumMismatch => Cow::Borrowed("refused to parse the data (CRC32 mismatch)"),
            Error::SizeLimitExceeded => Cow::Borrowed("usize limit exceeded"),
            Error::ArrayLimitExceeded => Cow::Borrowed("array limit of 0xFFFFFFFF (4294967295) exceeded"),
            Error::IndexLimitExceeded => Cow::Borrowed("index limit of 0xFFFF (65535) exceeded"),
            Error::String32SizeLimitExceeded => Cow::Borrowed("string data is longer than 31 characters"),
            Error::TagNotFound(tag) => Cow::Owned(format!("tag `{tag}` not found")),
            Error::FailedToReadFile => Cow::Borrowed("failed to read file"),
            Error::FailedToWriteFile => Cow::Borrowed("failed to write file"),
            Error::InvalidTagsDirectory => Cow::Borrowed("invalid tags directory"),
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
    fn mul_overflow_checked(self, other: Self) -> RinghopperResult<Self>;
}

impl OverflowCheck for usize {
    fn add_overflow_checked(self, other: Self) -> RinghopperResult<Self> {
        self.checked_add(other).ok_or(Error::SizeLimitExceeded)
    }
    fn mul_overflow_checked(self, other: Self) -> RinghopperResult<Self> {
        self.checked_mul(other).ok_or(Error::SizeLimitExceeded)
    }
}
