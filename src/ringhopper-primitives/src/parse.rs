use std::any::Any;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::slice::Chunks;
use crate::error::{Error, OverflowCheck};

use byteorder::*;
use crate::dynamic::{DynamicTagData, DynamicTagDataType, SimplePrimitiveType};

use crate::error::RinghopperResult;
use crate::map::{DomainType, Map};

/// Maximum length for an array.
///
/// This is enforced by the [`SimpleTagData`] (and, by extension, [`TagData`]) implementation of [`usize`].
pub const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;

/// Tag data parsing/writing methods.
///
/// This is used for parsing data that can be processed later as well as serializing data into formats that can be read later.
pub trait TagData: TagDataDefaults {
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

    /// Read data from the map.
    fn read_from_map<M: Map>(map: &M, address: usize, domain_type: &DomainType) -> RinghopperResult<Self> where Self: Sized;
}

/// Functionality for defaulting zeroed values.
pub trait TagDataDefaults {
    fn set_defaults(&mut self) {}
    fn unset_defaults(&mut self) {}
}

/// Automatically implements types for [`TagData`] for simple types.
pub trait SimpleTagData: TagDataDefaults + Sized {
    /// Get the raw size of the data in bytes.
    fn simple_size() -> usize;

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

    /// Iterate on bytes for an array of structs.
    ///
    /// Returns an error if `data` is not divisible by `simple_size`.
    fn read_chunks_to_iterator<B: ByteOrder>(data: &[u8]) -> RinghopperResult<RawStructIterator<Self, B>> {
        let chunk_size = Self::simple_size();
        if chunk_size == 0 || data.len() % chunk_size != 0 {
            return Err(Error::TagParseFailure(format!("cannot read_all_to_iterator; data cannot be evenly divided into {chunk_size} byte chunks")))
        }

        Ok(RawStructIterator {
            chunks: data.chunks(Self::simple_size()),
            _phantom_a: Default::default(),
            _phantom_b: Default::default(),
        })
    }

    /// Write the struct into bytes.
    ///
    /// If `simple_size()` exceeds [`SIMPLE_TAG_DATA_BYTES_LOCAL_LEN`] bytes, this will be heap-allocated. Otherwise, this will be allocated locally.
    ///
    /// This can be useful to avoid allocating a buffer manually, especially on the heap if the struct is small.
    fn as_bytes<B: ByteOrder>(&self) -> RinghopperResult<SimpleTagDataBytes> {
        let size = Self::simple_size();
        if size > SIMPLE_TAG_DATA_BYTES_LOCAL_LEN {
            let mut heap = vec![0u8; size];
            self.write::<B>(&mut heap, 0, size)?;
            Ok(SimpleTagDataBytes::Heap(heap))
        }
        else {
            let mut data = [0u8; SIMPLE_TAG_DATA_BYTES_LOCAL_LEN];
            self.write::<B>(&mut data, 0, SIMPLE_TAG_DATA_BYTES_LOCAL_LEN)?;
            Ok(SimpleTagDataBytes::Local { buffer: data, size })
        }
    }

    fn read_chunks_from_map_to_iterator<'a, M: Map>(map: &'a M, count: usize, address: usize, domain_type: &DomainType) -> RinghopperResult<RawStructIterator<'a, Self, LittleEndian>> {
        let chunk_size = Self::simple_size();
        let size = count.mul_overflow_checked(chunk_size)?;

        map.get_data_at_address(address, domain_type, size)
            .map(|data| RawStructIterator {
                chunks: data.chunks(chunk_size),
                _phantom_a: Default::default(),
                _phantom_b: Default::default(),
            })
            .ok_or_else(|| Error::MapDataOutOfBounds(format!("can't read {count} {chunk_size}-sized chunk(s) from 0x{address:08X} in {domain_type:?}")))
    }
}

pub const SIMPLE_TAG_DATA_BYTES_LOCAL_LEN: usize = 32;

#[derive(Clone)]
pub enum SimpleTagDataBytes {
    /// Not allocated on the heap.
    Local { buffer: [u8; SIMPLE_TAG_DATA_BYTES_LOCAL_LEN], size: usize },

    /// Allocated on the heap.
    Heap(Vec<u8>)
}

impl SimpleTagDataBytes {
    /// Get a reference to the bytes.
    pub fn bytes(&self) -> &[u8] {
        match &self {
            Self::Local { buffer: data, size } => &data[..*size],
            Self::Heap(v) => v.as_slice()
        }
    }

    /// Convert the bytes into a `Vec<u8>`.
    pub fn to_vec(self) -> Vec<u8> {
        match self {
            Self::Local { buffer: data, size } => data[..size].to_vec(),
            Self::Heap(v) => v
        }
    }
}

/// Reader for SimpleTagData types.
///
/// If an error occurs when calling `next()`, an error will be returned, but the cursor will still be advanced.
pub struct RawStructIterator<'a, T: SimpleTagData, B: ByteOrder> {
    chunks: Chunks<'a, u8>,
    _phantom_a: PhantomData<T>,
    _phantom_b: PhantomData<B>
}

impl<'a, T: SimpleTagData, B: ByteOrder> RawStructIterator<'a, T, B> {
    fn read(c: &[u8]) -> RinghopperResult<T> {
        T::read::<B>(c, 0, c.len())
    }

    /// Consume the iterator, converting it into a [`RawStructIteratorInfallible`].
    pub fn into_infallible(self) -> RawStructIteratorInfallible<'a, T, B> {
        RawStructIteratorInfallible {
            chunks: self.chunks,
            _phantom_a: self._phantom_a,
            _phantom_b: self._phantom_b
        }
    }
}

impl<'a, T: SimpleTagData, B: ByteOrder> From<RawStructIterator<'a, T, B>> for RawStructIteratorInfallible<'a, T, B> {
    fn from(value: RawStructIterator<'a, T, B>) -> Self {
        value.into_infallible()
    }
}

/// Reader for SimpleTagData types.
///
/// If an error occurs when calling `next()`, a panic will occur.
pub struct RawStructIteratorInfallible<'a, T: SimpleTagData, B: ByteOrder> {
    chunks: Chunks<'a, u8>,
    _phantom_a: PhantomData<T>,
    _phantom_b: PhantomData<B>
}

impl<'a, T: SimpleTagData, B: ByteOrder> RawStructIteratorInfallible<'a, T, B> {
    fn read(c: &[u8]) -> T {
        T::read::<B>(c, 0, c.len()).unwrap()
    }
}

macro_rules! define_raw_struct_iterator {
    ($type:tt, $item:ty) => {
        impl<'a, T: SimpleTagData, B: ByteOrder> Iterator for $type<'a, T, B> {
            type Item = $item;

            fn next(&mut self) -> Option<Self::Item> {
                self.chunks.next().map(Self::read)
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.chunks.size_hint()
            }
        }

        impl<'a, T: SimpleTagData, B: ByteOrder> DoubleEndedIterator for $type<'a, T, B> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.chunks.next_back().map(Self::read)
            }
        }

        impl<'a, T: SimpleTagData, B: ByteOrder> ExactSizeIterator for $type<'a, T, B> {}
        impl<'a, T: SimpleTagData, B: ByteOrder> FusedIterator for $type<'a, T, B> {}
    };
}

define_raw_struct_iterator!(RawStructIterator, RinghopperResult<T>);
define_raw_struct_iterator!(RawStructIteratorInfallible, T);

/// Automatically implements DynamicTagData for simple primitives.
pub trait SimplePrimitive: SimpleTagData {
    /// Get the primitive type.
    fn primitive_type() -> SimplePrimitiveType;
}

impl<T: SimpleTagData + TagDataDefaults + Sized> TagData for T {
    fn size() -> usize {
        T::simple_size()
    }
    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, _: &mut usize) -> RinghopperResult<Self> {
        T::read::<BE>(data, at, struct_end)
    }
    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.write::<BE>(data, at, struct_end)
    }
    fn read_from_map<M: Map>(map: &M, address: usize, domain_type: &DomainType) -> RinghopperResult<Self> {
        let size = T::simple_size();
        let data = match map.get_data_at_address(address, domain_type, size) {
            Some(n) => n,
            None => {
                 return match map.get_domain(domain_type) {
                    Some((data, base)) => {
                        let len = data.len();
                        Err(Error::MapDataOutOfBounds(format!("cannot read 0x{address:08X}[0x{size:04X}] bytes from {domain_type:?} because it's out of bounds (range: 0x{base:08X}[0x{len:08X}])")))
                    },
                    None => Err(Error::MapDataOutOfBounds(format!("cannot read 0x{address:08X}[0x{size:04X}] bytes from {domain_type:?} because the domain doesn't exist")))
                }
            }
        };
        T::read::<LittleEndian>(data, 0, data.len())
    }
}

impl <T: SimplePrimitive> TagDataDefaults for T {}

impl <T: SimplePrimitive + SimpleTagData + Sized + 'static> DynamicTagData for T {
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
        DynamicTagDataType::SimplePrimitive(T::primitive_type())
    }
}

pub(crate) fn fits(size: usize, at: usize, vec_size: usize) -> RinghopperResult<usize> {
    let end = at.add_overflow_checked(size)?;

    // If we're outside of the data bounds, fail.
    if end > vec_size {
        Err(Error::TagParseFailure(format!("data is out-of-bounds: 0x{end:04X} (required) > 0x{vec_size:04X} (available)")))
    }
    else {
        Ok(end)
    }
}

pub(crate) fn tag_data_fits<T: SimpleTagData>(at: usize, struct_end: usize, vec_size: usize) -> RinghopperResult<usize> {
    let size = T::size();
    let end = fits(size, at, vec_size)?;

    // If data is out of the struct bounds, then this is a programming error rather than bad tag data as it means our struct size is wrong.
    debug_assert!(end <= struct_end, "Data is outside of the struct (this is a bug!) - (0x{at:08X} [offset] + 0x{size:08X} [size] = 0x{end:08X} [end]) <= 0x{struct_end:08X} [struct_end]", at=at, size=size, end=end, struct_end=struct_end);

    Ok(end)
}

macro_rules! generate_tag_data_for_number {
    ($type:tt, $bo_read:tt, $bo_write:tt, $primitive_type:tt) => {
        impl SimpleTagData for $type {
            fn simple_size() -> usize {
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
        impl SimplePrimitive for $type {
            fn primitive_type() -> SimplePrimitiveType {
                SimplePrimitiveType::$primitive_type
            }
        }
    };
}

generate_tag_data_for_number!(i16, read_i16, write_i16, I16);
generate_tag_data_for_number!(u16, read_u16, write_u16, U16);
generate_tag_data_for_number!(i32, read_i32, write_i32, I32);
generate_tag_data_for_number!(u32, read_u32, write_u32, U32);
generate_tag_data_for_number!(f32, read_f32, write_f32, F32);

impl SimpleTagData for bool {
    fn simple_size() -> usize {
        u8::simple_size()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(u8::read::<B>(data, at, struct_end)? != 0)
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        (*self as u8).write::<B>(data, at, struct_end)
    }
}
impl SimplePrimitive for bool {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::Bool
    }
}

impl SimpleTagData for u8 {
    fn simple_size() -> usize {
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
impl SimplePrimitive for u8 {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::U8
    }
}

impl SimpleTagData for i8 {
    fn simple_size() -> usize {
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
impl SimplePrimitive for i8 {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::I8
    }
}

/// Enforces sizes to be less than [`MAX_ARRAY_LENGTH`].
///
/// We have special handling for `usize` because Rust internally uses `usize` for vectors, but tags are defined using
/// 32-bit sizes, instead.
impl SimpleTagData for usize {
    fn simple_size() -> usize {
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
impl SimplePrimitive for usize {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::Size
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
