use byteorder::ByteOrder;

use crate::primitive::*;
use crate::accessor::*;
use crate::parse::*;
use crate::error::*;

use std::any::Any;

pub trait PrimaryTagStruct: Sized + TagDataAccessor + TagData {}

/// Defines traits for a primary tag struct (i.e. the main struct in a tag).
pub trait PrimaryTagStructGroup: PrimaryTagStruct {
    fn fourcc() -> FourCC;
    fn version() -> u16;
}

/// If CRC32 is set to this, then disable CRC32 checks.
pub const IGNORED_CRC32: u32 = 0xFFFFFFFF;

/// Required to be set for the tag header.
pub const BLAM_FOURCC: u32 = 0x626C616D;

#[derive(Copy, Clone, PartialEq, Default)]
#[repr(C)]
pub struct TagFileHeader {
    /// Some ID value; not set on most tags, and always unused.
    pub id: ID,

    /// Some string value; not set on most tags, and always unused.
    pub string: String32,

    /// FourCC of the tag.
    pub fourcc: FourCC,

    /// CRC32 of all data following the header.
    ///
    /// Set to [`IGNORED_CRC32`] to disable checks.
    pub crc32: u32,

    /// Size of the header. Equal to 0x40.
    pub header_size: u32,

    /// Here be dragons.
    pub padding: Padding<[u8; 8]>,

    /// Version of the tag struct.
    pub version: u16,

    /// Always 255!
    pub two_hundred_and_fifty_five: u16,

    /// `blam` FourCC; must be equal to [`BLAM_FOURCC`].
    pub blam_fourcc: u32,
}

impl TagDataSimplePrimitive for TagFileHeader {
    fn size() -> usize {
        const _: () = assert!(std::mem::size_of::<TagFileHeader>() == 0x40);
        std::mem::size_of::<TagFileHeader>()
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.id.write::<B>(data, at + 0x00, struct_end)?;
        self.string.write::<B>(data, at + 0x04, struct_end)?;
        self.fourcc.write::<B>(data, at + 0x24, struct_end)?;
        self.crc32.write::<B>(data, at + 0x28, struct_end)?;
        self.header_size.write::<B>(data, at + 0x2C, struct_end)?;
        self.padding.write::<B>(data, at + 0x30, struct_end)?;
        self.version.write::<B>(data, at + 0x38, struct_end)?;
        self.two_hundred_and_fifty_five.write::<B>(data, at + 0x3A, struct_end)?;
        self.blam_fourcc.write::<B>(data, at + 0x3C, struct_end)?;
        Ok(())
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(Self {
            id: ID::read::<B>(data, at + 0x00, struct_end)?,
            string: String32::read::<B>(data, at + 0x04, struct_end)?,
            fourcc: FourCC::read::<B>(data, at + 0x24, struct_end)?,
            crc32: u32::read::<B>(data, at + 0x28, struct_end)?,
            header_size: u32::read::<B>(data, at + 0x2C, struct_end)?,
            padding: Padding::<[u8; 8]>::read::<B>(data, at + 0x30, struct_end)?,
            version: u16::read::<B>(data, at + 0x38, struct_end)?,
            two_hundred_and_fifty_five: u16::read::<B>(data, at + 0x3A, struct_end)?,
            blam_fourcc: u32::read::<B>(data, at + 0x3C, struct_end)?,
        })
    }
}

impl TagFileHeader {
    /// Return `true` if the header is valid.
    pub fn valid<T: PrimaryTagStructGroup>(&self) -> bool {
        self.fourcc == T::fourcc()
        && self.version == T::version()
        && self.blam_fourcc == BLAM_FOURCC
        && self.two_hundred_and_fifty_five == 0xFF
        && self.header_size as usize == <Self as TagDataSimplePrimitive>::size()
    }
}

/// Determine how strict parsing should be.
///
/// In all cases, out-of-bounds data cannot be parsed. However, allowing data with bad checksums to be parsed can be enabled, if desired.
#[derive(Default)]
pub enum ParseStrictness {
    /// Require the CRC32 to match; should be used by default.
    #[default]
    Strict,

    /// Allow the CRC32 to mismatch; should only be used if accessing potentially broken tags is necessary.
    Relaxed,
}

pub struct TagFile {
    pub header: TagFileHeader,
    pub actual_crc32: Option<u32>,
    pub data: Box<dyn Any>
}

impl TagFile {
    /// Check if the CRC32 in the header matches the CRC32 in the struct.
    ///
    /// Returns one of three values:
    /// - `Some(true)` if the CRC32 in the header matches the CRC32 in the struct
    /// - `Some(false)` if the CRC32 in the header does not match the CRC32 in the struct
    /// - `None` if the CRC32 was not calculated (i.e. the CRC32 in the header was [`IGNORED_CRC32`])
    pub fn crc32_matches(&self) -> Option<bool> {
        self.actual_crc32.map(|c| c == self.header.crc32)
    }

    /// Generate a byte array containing the tag data and a header.
    pub fn to_tag_file<T: PrimaryTagStructGroup>(tag_data: &T) -> RinghopperResult<Vec<u8>> {
        let header_len = <TagFileHeader as TagData>::size();
        let tag_file_len = <T as TagData>::size();
        let capacity = header_len + tag_file_len;

        let mut data = Vec::new();
        data.resize(capacity, 0);
        tag_data.write_to_tag_file(&mut data, header_len, capacity)?;

        let crc32 = crate::crc32::crc32(&data[header_len..]);
        let new_header = TagFileHeader {
            blam_fourcc: BLAM_FOURCC,
            crc32,
            fourcc: T::fourcc(),
            header_size: header_len as u32,
            version: T::version(),
            two_hundred_and_fifty_five: 255,
            ..Default::default()
        };
        new_header.write_to_tag_file(&mut data, 0, header_len).expect("writing the tag file header should always work");

        Ok(data)
    }

    /// Read the tag file.
    pub fn read_tag_file<T: PrimaryTagStructGroup + 'static>(file: &[u8], strictness: ParseStrictness) -> RinghopperResult<Self> {
        let header = TagFileHeader::read_from_tag_file(file, 0, 0x40, &mut 0)?;
        let data_after_header = &file[<TagFileHeader as TagData>::size()..];

        if !header.valid::<T>() {
            return Err(Error::TagParseFailure)
        }

        let actual_crc32 = if header.crc32 == IGNORED_CRC32 { None } else { Some(crate::crc32::crc32(data_after_header)) };
        match strictness {
            ParseStrictness::Relaxed => (),
            ParseStrictness::Strict => {
                if let Some(c) = actual_crc32 {
                    if c != header.crc32 {
                        return Err(Error::ChecksumMismatch)
                    }
                }
            }
        }

        let mut cursor = T::size();
        let data = Box::new(T::read_from_tag_file(data_after_header, 0, T::size(), &mut cursor)?);

        Ok(Self {
            header,
            actual_crc32,
            data
        })
    }

    pub fn get_ref<T: PrimaryTagStructGroup + 'static>(&self) -> &T {
        self.data.as_ref().downcast_ref::<T>().unwrap()
    }

    pub fn get_mut<T: PrimaryTagStructGroup + 'static>(&mut self) -> &mut T {
        self.data.as_mut().downcast_mut::<T>().unwrap()
    }

    pub fn take<T: PrimaryTagStructGroup + 'static + Default>(self) -> T {
        let mut data = self.data;
        std::mem::take(data.as_mut().downcast_mut::<T>().unwrap())
    }
}

#[cfg(test)]
mod test;
