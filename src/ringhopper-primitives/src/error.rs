//! Error-handling for methods that can fail in Ringhopper.

use std::fmt::Display;

/// General Result type for Ringhopper that uses [`Error`].
pub type RinghopperResult<T> = Result<T, Error>;

/// General error type for Ringhopper.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Error {
    NoSuchTagGroup,
    InvalidTagPath,
    InvalidID,
    InvalidEnum,
    InvalidTagFile,
    CorruptedTagFile,
    TagHeaderGroupTypeMismatch,
    TagHeaderGroupVersionMismatch,
    TagGroupUnimplemented,
    ChecksumMismatch,
    ArrayLimitExceeded,
    IndexLimitExceeded,
    SizeLimitExceeded,
    String32SizeLimitExceeded,
    FileNotFound,
    FailedToReadFile,
    FailedToWriteFile,
    InvalidTagsDirectory
}

impl Error {
    /// Get the string representation of the error.
    pub fn as_str(self) -> &'static str {
        match self {
            Error::InvalidTagPath => "invalid tag path",
            Error::InvalidID => "invalid ID",
            Error::InvalidEnum => "invalid enum value",
            Error::NoSuchTagGroup => "no such tag group",
            Error::InvalidTagFile => "tag file is invalid (bad header)",
            Error::CorruptedTagFile => "failed to parse the tag (tag is likely corrupt)",
            Error::TagHeaderGroupTypeMismatch => "failed to parse the tag due to it being the wrong group",
            Error::TagHeaderGroupVersionMismatch => "failed to parse the tag due to it being the wrong group version",
            Error::TagGroupUnimplemented => "tag group is unimplemented",
            Error::ChecksumMismatch => "refused to parse the data (CRC32 mismatch)",
            Error::SizeLimitExceeded => "usize limit exceeded",
            Error::ArrayLimitExceeded => "array limit of 0xFFFFFFFF (4294967295) exceeded",
            Error::IndexLimitExceeded => "index limit of 0xFFFF (65535) exceeded",
            Error::String32SizeLimitExceeded => "string data is longer than 31 characters",
            Error::FileNotFound => "file not found",
            Error::FailedToReadFile => "failed to read file",
            Error::FailedToWriteFile => "failed to write file",
            Error::InvalidTagsDirectory => "invalid tags directory"
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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
