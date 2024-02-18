use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use crate::parse::*;
use crate::error::*;
use byteorder::*;
use std::fmt::Display;
use crate::dynamic::{DynamicReflexive, DynamicTagData, DynamicTagDataArray, DynamicTagDataType, SimplePrimitiveType};
use crate::map::{DomainType, Map, ResourceMapType};

/// 16-bit index type
pub type Index = Option<u16>;

impl TagDataSimplePrimitive for Index {
    fn size() -> usize {
        <u16 as TagDataSimplePrimitive>::size()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        match u16::read::<B>(data, at, struct_end)? {
            0xFFFF => Ok(None),
            n => Ok(Some(n))
        }
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        match self {
            Some(0xFFFF) => return Err(Error::IndexLimitExceeded),
            Some(n) => n.write::<B>(data, at, struct_end),
            None => 0xFFFFu16.write::<B>(data, at, struct_end)
        }
    }

    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::Index
    }
}

/// Nullable index for referring to an element in an array.
///
/// IDs are stored with a 16-bit salt and a 16-bit index. If both are `0xFFFF`, then the ID is null. Otherwise, the
/// salt is array-specific and used for uniquely identifying indices from different arrays (it is XOR'd with the index
/// and 0x8000), thus identical indices from different array types (e.g. script nodes vs. tags) will not equal each
/// other.
#[derive(Copy, Clone, PartialEq)]
#[repr(transparent)]
pub struct ID {
    id: u32
}

impl ID {
    /// Construct an ID from a salt and index.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ringhopper_primitives::primitive::ID;
    ///
    /// let value = ID::new(Some(0x1234), 0x5678);
    /// assert_eq!(value.index().unwrap(), 0x1234);
    /// assert_eq!(value.salt().unwrap(), 0x5678);
    /// ```
    pub const fn new(index: Index, salt: u16) -> Self {
        let index = if let Some(n) = index {
            n
        }
        else {
            return ID::null()
        };

        let id = (((salt ^ index) as u32 | 0x8000) << 16) | (index as u32);
        let rv = Self { id };
        debug_assert!(!rv.is_null());
        rv
    }

    /// Create a null ID.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ringhopper_primitives::primitive::ID;
    ///
    /// let value = ID::null();
    /// assert!(value.is_null());
    /// ```
    pub const fn null() -> Self {
        ID { id: 0xFFFFFFFF }
    }

    /// Get the index value of the ID.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ringhopper_primitives::primitive::ID;
    ///
    /// let value = ID::new(Some(0x1234), 0x5678).index().expect("not null");
    /// assert_eq!(value, 0x1234);
    /// ```
    pub const fn index(self) -> Index {
        match self.is_null() {
            false => Some((self.id & 0xFFFF) as u16),
            true => None
        }
    }

    /// Get the salt value of the ID.
    ///
    /// Returns `None` if the ID is null.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ringhopper_primitives::primitive::ID;
    ///
    /// let value = ID::new(Some(0x1234), 0x5678).salt().expect("not null");
    /// assert_eq!(value, 0x5678);
    /// ```
    pub const fn salt(self) -> Option<u16> {
        let salt = match self.index() {
            Some(index) => ((self.id ^ ((index as u32) << 16)) >> 16) as u16,
            None => return None
        };
        Some(salt & !0x8000)
    }

    /// Convert into a [`u32`].
    pub const fn as_u32(self) -> u32 {
        self.id
    }

    /// Convert from a [`u32`] with validity checks.
    pub const fn from_u32_checked(id: u32) -> RinghopperResult<Self> {
        let id = Self::from_u32(id);
        if id.is_valid() {
            Err(Error::InvalidID)
        }
        else {
            Ok(id)
        }
    }

    /// Convert from a [`u32`].
    pub const fn from_u32(id: u32) -> Self {
        Self {
            id
        }
    }

    /// Return `true` if the ID is null.
    pub const fn is_null(self) -> bool {
        self.id == 0xFFFFFFFF
    }

    /// Return `true` if the ID is a valid ID.
    pub const fn is_valid(self) -> bool {
        (self.id & 0x80000000) != 0
    }
}

impl Default for ID {
    fn default() -> Self {
        Self::null()
    }
}

/// ID types used for salt values in IDs.
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u16)]
pub enum IDType {
    Tag = 0x6174,
    ScriptNode = 0x6373
}
impl IDType {
    /// Get the salt value for the ID type.
    pub const fn salt(self) -> u16 {
        self as u16
    }
}

impl TagDataSimplePrimitive for ID {
    fn size() -> usize {
        <u32 as TagDataSimplePrimitive>::size()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(ID::from_u32(<u32 as TagDataSimplePrimitive>::read::<B>(data, at, struct_end)?))
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        let id: u32 = (*self).into();
        id.write::<B>(data, at, struct_end)
    }

    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::ID
    }
}

impl From<ID> for u32 {
    fn from(value: ID) -> Self {
        value.as_u32()
    }
}

impl Display for ID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.is_null() {
            true => f.write_str("(id=null, salt=null)"),
            false => write!(f, "(id={}, salt={})", self.index().unwrap(), self.salt().unwrap())
        }
    }
}

impl Debug for ID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <ID as Display>::fmt(&self, f)
    }
}

/// Used to denote unused memory.
///
/// When stored in memory, it is represented as an array of `00` bytes.
#[derive(Copy, Clone, Default, PartialEq)]
#[repr(transparent)]
pub struct Padding<T: Sized + Default> {
    internal: T
}

impl<T: Copy + Default> TagDataSimplePrimitive for Padding<T> {
    fn size() -> usize {
        std::mem::size_of::<T>()
    }
    fn read<B: ByteOrder>(_: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        fits(<Self as TagDataSimplePrimitive>::size(), at, struct_end)?;
        Ok(Padding { internal: T::default() })
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        fits(<Self as TagDataSimplePrimitive>::size(), at, struct_end).expect("should fit");
        data[at..at+<Self as TagDataSimplePrimitive>::size()].fill(0u8);
        Ok(())
    }

    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::Padding
    }
}

impl<T: Copy + Default> Debug for Padding<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} bytes padding)", <Self as TagDataSimplePrimitive>::size())
    }
}

/// Container of bytes for data that isn't structured tag data but is stored in tag data when built into a cache file
/// and uses a memory address pointer.
///
/// In Ringhopper, this type simply wraps a `Vec<u8>` object.
///
/// # Limitations
///
/// A limitation over [`Vec`] is that the number of elements cannot exceed [`u32::MAX`] (i.e. 2<sup>32</sup> − 1), as
/// lengths are internally stored as 32-bit. As such, serialization in tag or cache format is not possible if this
/// limit is exceeded.
#[derive(Clone, Default, PartialEq, Debug)]
#[repr(transparent)]
pub struct Data {
    pub bytes: Vec<u8>
}

/// Container of bytes for data that is explicitly stored outside of tag data when built into a cache file and uses a
/// file offset.
///
/// In Ringhopper, this type simply wraps a `Vec<u8>` object.
///
/// # Limitations
///
/// A limitation over [`Vec`] is that the number of elements cannot exceed [`u32::MAX`] (i.e. 2<sup>32</sup> − 1), as
/// lengths are internally stored as 32-bit. As such, serialization in tag or cache format is not possible if this
/// limit is exceeded.
#[derive(Clone, Default, PartialEq, Debug)]
#[repr(transparent)]
pub struct FileData {
    pub bytes: Vec<u8>
}

macro_rules! make_data_tag_data_fns {
    () => {
        fn size() -> usize {
            <DataC as TagData>::size()
        }

        fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
            let c_primitive = DataC::read_from_tag_file(data, at, struct_end, extra_data_cursor)?;

            let size = c_primitive.size as usize;
            let data_location = *extra_data_cursor;
            fits(size, data_location, data.len())?;
            *extra_data_cursor += size;

            Ok(Self {
                bytes: data[data_location..*extra_data_cursor].to_owned()
            })
        }

        fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
            (DataC {
                size: self.bytes.len().into_u32()?,
                ..Default::default()
            }).write_to_tag_file(data, at, struct_end)?;
            data.extend_from_slice(&self.bytes);
            Ok(())
        }
    };
}

impl TagData for FileData {
    make_data_tag_data_fns!();

    fn read_from_map<M: Map>(map: &M, address: usize, domain_type: &DomainType) -> RinghopperResult<Self> {
        let c_primitive = DataC::read_from_map(map, address, domain_type)?;

        let address = c_primitive.file_offset as usize;
        let length = c_primitive.size as usize;
        if length == 0 {
            return Ok(Self::default())
        }

        // If in a sounds.map file
        let domain_to_use = if (c_primitive.flags & 1) != 0 {
            &DomainType::ResourceMapFile(ResourceMapType::Sounds)
        }
        else {
            &DomainType::MapData
        };
        let data = map.get_data_at_address(address, domain_to_use, length);
        let data = match data {
            Some(n) => n,
            None => return Err(Error::MapDataOutOfBounds(format!("can't read 0x{address:08X}[{length:08X}] bytes from {domain_to_use:?}")))
        };

        Ok(Self { bytes: data.to_vec() })
    }
}

impl TagData for Data {
    make_data_tag_data_fns!();

    fn read_from_map<M: Map>(map: &M, address: usize, domain_type: &DomainType) -> RinghopperResult<Self> {
        let c_primitive = DataC::read_from_map(map, address, domain_type)?;
        let address = c_primitive.address.address as usize;
        let length = c_primitive.size as usize;
        if length == 0 {
            return Ok(Self::default())
        }
        let data = map.get_data_at_address(address, domain_type, length);
        let data = match data {
            Some(n) => n,
            None => return Err(Error::MapDataOutOfBounds(format!("can't read 0x{address:08X}[0x{length}] bytes from {domain_type:?}")))
        };

        Ok(Self { bytes: data.to_vec() })
    }
}

macro_rules! make_data_dynamic_tag_data {
    ($t:ty) => {
        impl $t {
            pub fn new(bytes: Vec<u8>) -> $t {
                Self {
                    bytes
                }
            }
        }

        impl FromIterator<u8> for $t {
            fn from_iter<I: IntoIterator<Item=u8>>(iter: I) -> Self {
                let mut bytes = Vec::new();
                for i in iter {
                    bytes.push(i);
                }
                Self { bytes }
            }
        }

        impl DynamicTagData for $t {
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
                DynamicTagDataType::Data
            }
        }
    };
}

make_data_dynamic_tag_data!(Data);
make_data_dynamic_tag_data!(FileData);

/// Container for contiguously stored blocks.
///
/// A `Reflexive` is functionally an array, containing a size value and, when stored in cache files, a memory address.
/// Elements are stored contiguously (i.e. back-to-back) in all formats.
///
/// In Ringhopper, this type simply wraps a `Vec<T>` object.
///
/// # Limitations
///
/// A limitation over [`Vec`] is that the number of elements cannot exceed [`u32::MAX`] (i.e. 2<sup>32</sup> − 1), as
/// lengths are internally stored as 32-bit. As such, serialization in tag or cache format is not possible if this
/// limit is exceeded.
#[derive(Clone, Default, PartialEq, Debug)]
#[repr(transparent)]
pub struct Reflexive<T: TagData + Sized> {
    pub items: Vec<T>
}

impl<T: TagData + Sized> Reflexive<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }
}

impl<T: TagData + Sized> FromIterator<T> for Reflexive<T> {
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
        let mut items = Vec::new();
        for i in iter {
            items.push(i);
        }
        Self { items }
    }
}

impl<T: TagData + Sized> TagData for Reflexive<T> {
    fn size() -> usize {
        <ReflexiveC<T> as TagDataSimplePrimitive>::size()
    }

    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        let c_primitive = ReflexiveC::<T>::read_from_tag_file(data, at, struct_end, extra_data_cursor)?;

        let count = c_primitive.count as usize;
        let item_size = T::size();
        let total_length = count.mul_overflow_checked(item_size)?;
        let mut result = Reflexive {
            items: Vec::with_capacity(count)
        };

        let mut item_offset = *extra_data_cursor;
        *extra_data_cursor = (item_offset).add_overflow_checked(total_length)?;

        for _ in 0..count {
            let struct_end = item_offset + item_size;
            result.items.push(T::read_from_tag_file(data, item_offset, struct_end, extra_data_cursor)?);
            item_offset = struct_end;
        }

        Ok(result)
    }

    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        ReflexiveC::<T>::with_params(self.items.len().into_u32()?, Address::default()).write_to_tag_file(data, at, struct_end)?;

        let item_size = T::size();
        let total_bytes_to_write = self.items.len().mul_overflow_checked(item_size)?;

        let mut write_offset = data.len();
        let new_len = write_offset.add_overflow_checked(total_bytes_to_write)?;
        data.resize(new_len, 0);

        for i in &self.items {
            let struct_end = write_offset + item_size;
            i.write_to_tag_file(data, write_offset, struct_end)?;
            write_offset = struct_end;
        }

        Ok(())
    }

    fn read_from_map<M: Map>(map: &M, address: usize, domain_type: &DomainType) -> RinghopperResult<Self> {
        let c_primitive = ReflexiveC::<T>::read_from_map(map, address, domain_type)?;

        let count = c_primitive.count as usize;
        let item_size = T::size();
        let mut address = c_primitive.address.address as usize;

        // Make sure we can add all of these
        count.mul_overflow_checked(item_size)?.add_overflow_checked(address)?;

        let mut result = Reflexive {
            items: Vec::with_capacity(count)
        };

        for _ in 0..count {
            result.items.push(T::read_from_map(map, address, domain_type)?);
            address += item_size;
        }

        Ok(result)
    }
}


/// Represents an address for cache files.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(transparent)]
pub struct Address {
    pub address: u32
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:08X}", self.address)
    }
}

impl From<u32> for Address {
    fn from(value: u32) -> Self {
        Address { address: value }
    }
}

impl From<Address> for u32 {
    fn from(value: Address) -> Self {
        value.address
    }
}

impl TagDataSimplePrimitive for Address {
    fn size() -> usize {
        <u32 as TagDataSimplePrimitive>::size()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(Self {
            address: u32::read::<B>(data, at, struct_end)?
        })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.address.write::<B>(data, at, struct_end)
    }

    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::Address
    }
}

pub(crate) type ReflexiveAccessRange = (usize, usize); // [start, end]
pub(crate) fn parse_range(matcher: &str, len: usize) -> Result<Vec<ReflexiveAccessRange>, &'static str> {
    const RANGE_SEPARATOR: u8 = ',' as u8;
    const START_END_SEPARATOR: u8 = '-' as u8;
    const EVERYTHING: u8 = '*' as u8;
    const END: u8 = 'e' as u8;

    let matcher_bytes = matcher.as_bytes();

    for &b in matcher_bytes {
        if b.is_ascii_digit() || b == RANGE_SEPARATOR || b == START_END_SEPARATOR || b == END || b == EVERYTHING {
            continue
        }
        return Err("invalid character in matcher")
    }

    let all_ranges = matcher_bytes.split(|b| *b == RANGE_SEPARATOR);
    let mut returned_ranges = Vec::new();

    for r in all_ranges {
        if r.is_empty() {
            return Err("empty range")
        }
        if r[0] == START_END_SEPARATOR || r[r.len() - 1] == START_END_SEPARATOR {
            return Err("range cannot start or end with `-`")
        }

        let first: usize;
        let end: usize;

        if r == [EVERYTHING] {
            // Normally would be out of bounds, but we can skip
            if len == 0 {
                continue;
            }

            // Otherwise, pass everything
            first = 0;
            end = len - 1;
        }
        else {
            if len == 0 {
                return Err("out of bounds");
            }

            // Get the range as a number
            let range_to_number = |n: &[u8]| -> Result<usize, &'static str> {
                if n.contains(&END) {
                    if n == [END] {
                        return Ok(len - 1)
                    }
                    return Err("cannot use exponents")
                }
                std::str::from_utf8(n).unwrap().parse().map_err(|_| "cannot parse number")
            };

            let mut numbers = r.split(|c| *c == START_END_SEPARATOR);

            first = range_to_number(numbers.next().unwrap())?;
            end = numbers.next().map(range_to_number).unwrap_or(Ok(first))?;

            if numbers.next().is_some() {
                return Err("only one `-` allowed per range")
            }
            if first > end {
                return Err("start of range must be before the end");
            }
            if end >= len {
                return Err("out of bounds");
            }
        }

        returned_ranges.push((first, end));
    }

    // Dedupe, sort, and create unions of all ranges if they can be condensed into one range
    // - 0-5 also contains 2-3, so we can remove 2-3
    // - 0-5 contains part of 4-6, so change to 0-6
    // - 0-5 doesn't contain 6-7, but we can still change it to 0-7
    // - 0-5 can't engulf 7-8, because 6 is not in either
    returned_ranges.dedup();

    // sort ranges by start index in ascending order
    returned_ranges.sort_by(|(start1, _), (start2, _)| start1.cmp(start2));

    let mut i = 0;
    while i < returned_ranges.len() {
        let mut end = *&returned_ranges[i].1;

        'remove_engulfed_loop: while returned_ranges.len() > i+1 {
            for j in i+1..returned_ranges.len() {
                let &(start_inner, end_inner) = &returned_ranges[j];
                if start_inner <= end || end + 1 == start_inner {
                    end = end.max(end_inner);
                    returned_ranges.remove(j);
                    continue 'remove_engulfed_loop;
                }
            }
            break;
        }

        returned_ranges[i].1 = end;
        i += 1;
    }

    Ok(returned_ranges)
}

impl<T: DynamicTagData + Sized + Default + Clone> DynamicTagData for Reflexive<T> {
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
        DynamicTagDataType::Reflexive
    }

    fn as_array(&self) -> Option<&dyn DynamicTagDataArray> {
        Some(self as &dyn DynamicTagDataArray)
    }

    fn as_array_mut(&mut self) -> Option<&mut dyn DynamicTagDataArray> {
        Some(self as &mut dyn DynamicTagDataArray)
    }
}

impl<T: DynamicTagData + Sized + Default + Clone> DynamicTagDataArray for Reflexive<T> {
    fn get_at_index(&self, index: usize) -> Option<&dyn DynamicTagData> {
        Some(&self.items[index])
    }

    fn get_at_index_mut(&mut self, index: usize) -> Option<&mut dyn DynamicTagData> {
        Some(&mut self.items[index])
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}

impl<T: DynamicTagData + Sized + Default + Clone> DynamicReflexive for Reflexive<T> {
    fn insert_default(&mut self, index: usize) {
        self.items.insert(index, Default::default());
    }

    fn insert_copy(&mut self, index: usize, item: &dyn DynamicTagData) {
        let item: &T = item.as_any().downcast_ref::<T>().unwrap();
        self.items.insert(index, item.clone());
    }

    fn insert_moved(&mut self, index: usize, item: &mut dyn DynamicTagData) {
        let item = std::mem::take(item.as_any_mut().downcast_mut::<T>().unwrap());
        self.items.insert(index, item);
    }
}

/// Lower level C implementation of a reflexive.
///
/// `T` refers to the type of object that the reflexive is supposed to point to.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct ReflexiveC<T: TagData + Sized> {
    /// Number of elements in the reflexive.
    pub count: u32,

    /// Address in tag data of the first element of the reflexive if count > 0.
    pub address: Address,

    /// Unused.
    pub padding: Padding<[u8; 4]>,

    /// Used for identifying the type being referred to.
    pub phantom: PhantomData<T>
}
impl<T: TagData + Sized> TagDataSimplePrimitive for ReflexiveC<T> {
    fn size() -> usize {
        std::mem::size_of::<Self>()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        let count = u32::read::<B>(data, at, struct_end)?;
        let address = Address::read::<B>(data, at + 0x4, struct_end)?;
        Ok(Self::with_params(count, address))
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.count.write::<B>(data, at, struct_end)?;
        self.address.write::<B>(data, at + 0x4, struct_end)?;
        self.padding.write::<B>(data, at + 0x8, struct_end)?;
        Ok(())
    }

    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::ReflexiveC
    }
}

impl<T: TagData + Sized> ReflexiveC<T> {
    /// Convenience function for initializing a `ReflexiveC` instance with `count` and `address`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{ReflexiveC, Address, Vector3D};
    ///
    /// let count: u32 = 1024;
    /// let address: Address = 0x12345678.into();
    ///
    /// let convenience: ReflexiveC<Vector3D> = ReflexiveC::with_params(count, address);
    /// let manual: ReflexiveC<Vector3D> = ReflexiveC { count, address, ..Default::default() };
    ///
    /// assert_eq!(manual, convenience);
    /// ```
    pub const fn with_params(count: u32, address: Address) -> Self {
        Self {
            count,
            address,
            padding: Padding { internal: [0u8; 4] },
            phantom: PhantomData { }
        }
    }
}

impl<T: TagData + Sized> Default for ReflexiveC<T> {
    fn default() -> Self {
        Self::with_params(u32::default(), Address::default())
    }
}

/// Lower level C implementation of tag data
#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct DataC {
    /// Length of the data in bytes.
    pub size: u32,

    /// Flags which are context-dependent; for example, whether or not the data is stored in sounds.map for a sound.
    pub flags: u32,

    /// File offset in bytes if not stored in tag data.
    pub file_offset: u32,

    /// Unused.
    pub padding: Padding<[u8; 4]>,

    /// Memory address if stored in tag data.
    pub address: Address
}
impl TagDataSimplePrimitive for DataC {
    fn size() -> usize {
        std::mem::size_of::<Self>()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        let size = u32::read::<B>(data, at, struct_end)?;
        let flags = u32::read::<B>(data, at + 0x4, struct_end)?;
        let file_offset = u32::read::<B>(data, at + 0x8, struct_end)?;
        let address = Address::read::<B>(data, at + 0xC, struct_end)?;
        Ok(Self { size, flags, file_offset, address, ..Default::default() })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.size.write::<B>(data, at, struct_end)?;
        self.flags.write::<B>(data, at + 0x4, struct_end)?;
        self.file_offset.write::<B>(data, at + 0x8, struct_end)?;
        self.padding.write::<B>(data, at + 0xC, struct_end)?;
        self.address.write::<B>(data, at + 0x10, struct_end)?;
        Ok(())
    }

    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::DataC
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq)]
#[repr(transparent)]
pub struct ScenarioScriptNodeValue {
    pub data: u32
}
impl From<i8> for ScenarioScriptNodeValue {
    fn from(value: i8) -> Self {
        Self { data: value as u32 }
    }
}
impl From<i16> for ScenarioScriptNodeValue {
    fn from(value: i16) -> Self {
        Self { data: value as u32 }
    }
}
impl From<i32> for ScenarioScriptNodeValue {
    fn from(value: i32) -> Self {
        Self { data: value as u32 }
    }
}
impl From<f32> for ScenarioScriptNodeValue {
    fn from(value: f32) -> Self {
        Self { data: unsafe { std::mem::transmute(value) } }
    }
}
impl From<ID> for ScenarioScriptNodeValue {
    fn from(value: ID) -> Self {
        Self { data: value.as_u32() }
    }
}
impl From<ScenarioScriptNodeValue> for i8 {
    fn from(value: ScenarioScriptNodeValue) -> Self {
        value.data as i8
    }
}
impl From<ScenarioScriptNodeValue> for i16 {
    fn from(value: ScenarioScriptNodeValue) -> Self {
        value.data as i16
    }
}
impl From<ScenarioScriptNodeValue> for i32 {
    fn from(value: ScenarioScriptNodeValue) -> Self {
        value.data as i32
    }
}
impl From<ScenarioScriptNodeValue> for f32 {
    fn from(value: ScenarioScriptNodeValue) -> Self {
        unsafe { std::mem::transmute(value.data) }
    }
}
impl From<ScenarioScriptNodeValue> for ID {
    fn from(value: ScenarioScriptNodeValue) -> Self {
        unsafe { std::mem::transmute(value.data) }
    }
}

impl TagDataSimplePrimitive for ScenarioScriptNodeValue {
    fn size() -> usize {
        <u32 as TagDataSimplePrimitive>::size()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(Self { data: u32::read::<B>(data, at, struct_end)? })
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.data.write::<B>(data, at, struct_end)
    }
    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::ScenarioScriptNodeValue
    }
}

#[cfg(test)]
mod test;
