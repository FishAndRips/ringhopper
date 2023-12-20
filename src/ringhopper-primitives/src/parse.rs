use crate::error::{Error, OverflowCheck};

use byteorder::*;

use crate::error::RinghopperResult;

/// Maximum length for an array.
///
/// This is enforced by the [`TagDataSimplePrimitive`] (and, by extension, [`TagData`]) implementation of [`usize`].
pub const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;

/// Tag data parsing/writing methods.
///
/// This is used for parsing data that can be processed later as well as serializing data into formats that can be read later.
pub trait TagData {
    /// Get the size of the tag data.
    fn size() -> usize where Self: Sized;

    /// Read data from the tag.
    ///
    /// - `data` is the entire tag.
    /// - `at` is the position of the data to read.
    /// - `struct_end` is the end of the struct.
    /// - `extra_data_cursor` is the cursor where extra data (e.g. strings) may be read
    ///
    /// Errors if the data is out of bounds.
    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> where Self: Sized;

    /// Write data to the tag.
    ///
    /// - `data` is the entire tag.
    /// - `at` is the position of the data to write.
    /// - `struct_end` is the end of the struct.
    ///
    /// This can get an error if some form of invariant is violated, such as a size being greater than [`MAX_ARRAY_LENGTH`].
    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()>;
}

/// Automatically implements types for [`TagData`] for simple types.
///
/// These types include:
/// - All [`Color`](crate::primitive::Color) types
/// - All [`Plane`](crate::primitive::Plane) types
/// - All [`Vector`](crate::primitive::Vector) types
/// - [`Address`](crate::primitive::Address)
/// - [`String32`](crate::primitive::String32)
/// - [`TagGroup`](crate::primitive::TagGroup)
/// - [`u8`]
/// - [`i8`]
/// - [`u16`]
/// - [`i16`]
/// - [`u32`]
/// - [`i32`]
/// - [`f32`]
pub trait TagDataSimplePrimitive: Sized {
    /// Get the raw size of the data in bytes.
    fn size() -> usize;

    /// Read a number from tag data.
    ///
    /// - `data` is the data to read from.
    /// - `at` is the position of the data to read.
    /// - `struct_end` is the end of the struct.
    ///
    /// Errors if the data is out of bounds.
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self>;

    /// Write a number to tag data.
    ///
    /// - `data` is the data to write to.
    /// - `at` is the position of the data to write.
    /// - `struct_end` is the end of the struct.
    ///
    /// This will only return an error if attempting to write a [`usize`] that is greater than [`MAX_ARRAY_LENGTH`].
    ///
    /// In all other error cases, it will panic.
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()>;
}

impl<T: TagDataSimplePrimitive + Sized> TagData for T {
    fn size() -> usize {
        <T as TagDataSimplePrimitive>::size()
    }
    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, _: &mut usize) -> RinghopperResult<Self> {
        T::read::<BE>(data, at, struct_end)
    }
    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.write::<BE>(data, at, struct_end)
    }
}

pub(crate) fn fits(size: usize, at: usize, vec_size: usize) -> RinghopperResult<usize> {
    let end = at.add_overflow_checked(size)?;

    // If we're outside of the data bounds, fail.
    if end > vec_size {
        Err(Error::TagParseFailure)
    }
    else {
        Ok(end)
    }
}

pub(crate) fn tag_data_fits<T: TagDataSimplePrimitive>(at: usize, struct_end: usize, vec_size: usize) -> RinghopperResult<usize> {
    let size = T::size();
    let end = fits(size, at, vec_size)?;

    // If data is out of the struct bounds, then this is a programming error rather than bad tag data as it means our struct size is wrong.
    debug_assert!(end <= struct_end, "Data is outside of the struct (this is a bug!) - (0x{at:08X} [offset] + 0x{size:08X} [size] = 0x{end:08X} [end]) <= 0x{struct_end:08X} [struct_end]", at=at, size=size, end=end, struct_end=struct_end);

    Ok(end)
}

macro_rules! generate_tag_data_for_number {
    ($type:tt, $bo_read:tt, $bo_write:tt) => {
        impl TagDataSimplePrimitive for $type {
            fn size() -> usize {
                std::mem::size_of::<Self>()
            }
            fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
                tag_data_fits::<Self>(at, struct_end, data.len())?;
                Ok(B::$bo_read(&data[at..]))
            }
            fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
                tag_data_fits::<Self>(at, struct_end, data.len()).expect("should fit");
                B::$bo_write(&mut data[at..], *self);
                Ok(())
            }
        }
    };
}

generate_tag_data_for_number!(i16, read_i16, write_i16);
generate_tag_data_for_number!(u16, read_u16, write_u16);
generate_tag_data_for_number!(i32, read_i32, write_i32);
generate_tag_data_for_number!(u32, read_u32, write_u32);
generate_tag_data_for_number!(f32, read_f32, write_f32);

impl TagDataSimplePrimitive for u8 {
    fn size() -> usize {
        std::mem::size_of::<u8>()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        tag_data_fits::<Self>(at, struct_end, data.len())?;
        Ok(data[at])
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        tag_data_fits::<Self>(at, struct_end, data.len()).expect("should fit");
        data[at] = *self;
        Ok(())
    }
}

impl TagDataSimplePrimitive for i8 {
    fn size() -> usize {
        std::mem::size_of::<i8>()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        tag_data_fits::<Self>(at, struct_end, data.len())?;
        Ok(data[at] as i8)
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        tag_data_fits::<Self>(at, struct_end, data.len()).expect("should fit");
        data[at] = *self as u8;
        Ok(())
    }
}

/// Enforces sizes to be less than [`MAX_ARRAY_LENGTH`].
///
/// We have special handling for `usize` because Rust internally uses `usize` for vectors, but tags are defined using
/// 32-bit sizes, instead.
impl TagDataSimplePrimitive for usize {
    fn size() -> usize {
        std::mem::size_of::<u32>()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        u32::read::<B>(data, at, struct_end).map(|r| r as usize)
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        if *self > MAX_ARRAY_LENGTH {
            return Err(Error::ArrayLimitExceeded)
        }
        let self_as_u32 = *self as u32;
        self_as_u32.write::<B>(data, at, struct_end)
    }
}

pub(crate) trait U32SizeConversion {
    fn into_u32(self) -> RinghopperResult<u32>;
}

impl U32SizeConversion for usize {
    fn into_u32(self) -> RinghopperResult<u32> {
        if self > u32::MAX as usize {
            Err(Error::ArrayLimitExceeded)
        }
        else {
            Ok(self as u32)
        }
    }
}

#[cfg(test)]
mod test;
