use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use crate::parse::*;
use crate::error::*;
use byteorder::*;
use std::fmt::Display;
use crate::dynamic::{DynamicReflexive, DynamicTagData, DynamicTagDataArray, DynamicTagDataType, SimplePrimitiveType, TagFieldMetadata};
use crate::map::{DomainType, Map, ResourceMapType};

/// 16-bit index type
pub type Index = Option<u16>;

impl SimpleTagData for Index {
    fn simple_size() -> usize {
        u16::simple_size()
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
}
impl SimplePrimitive for Index {
    fn primitive_type() -> SimplePrimitiveType {
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

impl SimpleTagData for ID {
    fn simple_size() -> usize {
        u32::simple_size()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(ID::from_u32(<u32 as SimpleTagData>::read::<B>(data, at, struct_end)?))
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        let id: u32 = (*self).into();
        id.write::<B>(data, at, struct_end)
    }
}
impl SimplePrimitive for ID {
    fn primitive_type() -> SimplePrimitiveType {
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

impl<T: Copy + Default> SimpleTagData for Padding<T> {
    fn simple_size() -> usize {
        std::mem::size_of::<T>()
    }
    fn read<B: ByteOrder>(_: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        fits(Self::simple_size(), at, struct_end)?;
        Ok(Padding { internal: T::default() })
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        fits(Self::simple_size(), at, struct_end).expect("should fit");
        data[at..at+Self::simple_size()].fill(0u8);
        Ok(())
    }
}

impl<T: Sized + Default> TagDataDefaults for Padding<T> {}

impl<T: Copy + Default> Debug for Padding<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} bytes padding)", Self::simple_size())
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

/// Container of bytes for data that is used for storing BSP vertex data.
///
/// On non-CEA maps, this acts identically to a [`Data`], where on CEA maps, this can load BSP vertex data from other
/// locations.
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
pub struct BSPVertexData {
    pub bytes: Vec<u8>
}

impl DataData for FileData {
    fn from_bytes(bytes: &[u8]) -> RinghopperResult<Self> {
        Ok(Self { bytes: bytes.to_owned() })
    }

    fn get_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }

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

pub(crate) trait DataData: TagDataDefaults + Sized {
    fn from_bytes(bytes: &[u8]) -> RinghopperResult<Self>;

    fn get_bytes(&self) -> &[u8];

    fn read_from_map<M: Map>(map: &M, address: usize, domain_type: &DomainType) -> RinghopperResult<Self> {
        let c_primitive = DataC::read_from_map(map, address, domain_type)?;
        let address = c_primitive.address.into();
        let length = c_primitive.size as usize;
        if length == 0 {
            return Self::from_bytes(&[])
        }
        let data = map.get_data_at_address(address, domain_type, length);
        let data = match data {
            Some(n) => n,
            None => return Err(Error::MapDataOutOfBounds(format!("can't read 0x{address:08X}[0x{length}] bytes from {domain_type:?}")))
        };

        Self::from_bytes(data.as_ref())
    }
}

// Used to bypass conflicting implementation error
macro_rules! make_tag_data_implementation_for_datadata {
    ($ty:ty) => {
        impl TagData for $ty {
            fn size() -> usize {
                <DataC as TagData>::size()
            }

            fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
                let c_primitive = DataC::read_from_tag_file(data, at, struct_end, extra_data_cursor)?;

                let size = c_primitive.size as usize;
                let data_location = *extra_data_cursor;
                fits(size, data_location, data.len())?;
                *extra_data_cursor += size;

                DataData::from_bytes(&data[data_location..*extra_data_cursor])
            }

            fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
                let bytes = self.get_bytes();
                (DataC {
                    size: bytes.len().into_u32()?,
                    ..Default::default()
                }).write_to_tag_file(data, at, struct_end)?;
                data.extend_from_slice(bytes);
                Ok(())
            }

            fn read_from_map<M: Map>(map: &M, address: usize, domain_type: &DomainType) -> RinghopperResult<Self> {
                DataData::read_from_map(map, address, domain_type)
            }
        }
    };
}
pub(crate) use make_tag_data_implementation_for_datadata;

impl DataData for Data {
    fn from_bytes(bytes: &[u8]) -> RinghopperResult<Self> {
        Ok(Self { bytes: bytes.to_owned() })
    }

    fn get_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

impl DataData for BSPVertexData {
    fn from_bytes(bytes: &[u8]) -> RinghopperResult<Self> {
        Ok(Self { bytes: bytes.to_owned() })
    }

    fn get_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }

    fn read_from_map<M: Map>(_map: &M, _address: usize, _domain_type: &DomainType) -> RinghopperResult<Self> {
        unimplemented!("read_from_map is unimplemented for BSP vertex data; use read_from_map_with_offset instead")
    }
}

impl BSPVertexData {
    pub fn read_from_map_with_offset<M: Map>(
        map: &M,
        address: usize,
        domain_type: &DomainType,
        compressed: bool,
        rendered_count: usize, rendered_offset: usize,
        lightmap_count: usize, lightmap_offset: usize
    ) -> RinghopperResult<Self> {
        if map.get_engine().compressed_models != compressed {
            return Ok(Default::default())
        }

        let c_primitive = DataC::read_from_map(map, address, domain_type)?;

        let p_address = c_primitive.address.into();
        let p_length = c_primitive.size as usize;

        let bsp = match domain_type {
            &DomainType::BSP(bsp) => bsp,
            d => unreachable!("domain_type not a BSP type but a {d:?}", d=d)
        };

        let rendered_size = rendered_count.mul_overflow_checked(if compressed { 32 } else { 56 })?;
        let lightmap_size = lightmap_count.mul_overflow_checked(if compressed { 8 } else { 20 })?;
        let total_size = rendered_size.add_overflow_checked(lightmap_size)?;
        let mut data: Vec<u8> = Vec::with_capacity(total_size);

        if !map.get_engine().external_bsps || p_address != 0 {
            match map.get_data_at_address(p_address, domain_type, total_size) {
                Some(n) => data.extend_from_slice(n),
                None => return Err(Error::MapDataOutOfBounds(format!("can't read combined BSP vertex data 0x{address:08X}[0x{p_length}] bytes from {domain_type:?}")))
            }
        }
        else {
            let rendered_data = match map.get_data_at_address(rendered_offset, &DomainType::BSPVertices(bsp), rendered_size) {
                Some(n) => n,
                None => return Err(Error::MapDataOutOfBounds(format!("can't read render BSP vertex data 0x{rendered_offset:08X}[0x{rendered_size}] bytes from {domain_type:?}")))
            };

            let lightmap_data = match map.get_data_at_address(lightmap_offset, &DomainType::BSPVertices(bsp), lightmap_size) {
                Some(n) => n,
                None => return Err(Error::MapDataOutOfBounds(format!("can't read lightmap BSP vertex data 0x{lightmap_offset:08X}[0x{lightmap_size}] bytes from {domain_type:?}")))
            };

            data.extend_from_slice(rendered_data);
            data.extend_from_slice(lightmap_data);
        }

        Ok(Self { bytes: data })
    }
}

macro_rules! make_data_dynamic_tag_data {
    ($t:tt) => {
        impl $t {
            pub fn new(bytes: Vec<u8>) -> $t {
                Self {
                    bytes
                }
            }
        }

        impl TagDataDefaults for $t {}

        impl FromIterator<u8> for $t {
            fn from_iter<I: IntoIterator<Item=u8>>(iter: I) -> Self {
                let mut bytes = Vec::new();
                for i in iter {
                    bytes.push(i);
                }
                Self { bytes }
            }
        }

        make_tag_data_implementation_for_datadata!($t);

        impl DynamicTagData for $t {
            fn get_field(&self, _field: &str) -> Option<&dyn DynamicTagData> {
                None
            }

            fn get_field_mut(&mut self, _field: &str) -> Option<&mut dyn DynamicTagData> {
                None
            }

            fn get_metadata_for_field(&self, _field: &str) -> Option<TagFieldMetadata> {
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
                DynamicTagDataType::$t
            }
        }
    };
}

make_data_dynamic_tag_data!(Data);
make_data_dynamic_tag_data!(FileData);
make_data_dynamic_tag_data!(BSPVertexData);

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

impl<'a, T: TagData + Sized> IntoIterator for &'a Reflexive<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.as_slice().into_iter()
    }
}

impl<'a, T: TagData + Sized> IntoIterator for &'a mut Reflexive<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.as_mut_slice().into_iter()
    }
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
        <ReflexiveC<T>>::simple_size()
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

        for i in self {
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
        let mut address = c_primitive.address.into();

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

impl<T: TagData + Sized> TagDataDefaults for Reflexive<T> {
    fn set_defaults(&mut self) {
        for i in &mut self.items {
            i.set_defaults()
        }
    }
    fn unset_defaults(&mut self) {
        for i in &mut self.items {
            i.unset_defaults()
        }
    }
}


/// Represents an address for cache files.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(transparent)]
pub struct Address {
    pub address: u32
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:08X}", self.address)
    }
}

impl From<u32> for Address {
    fn from(value: u32) -> Self {
        Address { address: value }
    }
}

impl From<Address> for usize {
    fn from(value: Address) -> Self {
        value.address as usize
    }
}

impl From<Address> for u32 {
    fn from(value: Address) -> Self {
        value.address
    }
}

impl SimpleTagData for Address {
    fn simple_size() -> usize {
        u32::simple_size()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(Self {
            address: u32::read::<B>(data, at, struct_end)?
        })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.address.write::<B>(data, at, struct_end)
    }
}
impl SimplePrimitive for Address {
    fn primitive_type() -> SimplePrimitiveType {
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

    fn get_metadata_for_field(&self, _field: &str) -> Option<TagFieldMetadata> {
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
impl<T: TagData + Sized> SimpleTagData for ReflexiveC<T> {
    fn simple_size() -> usize {
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
}

impl<T: TagData + Sized> TagDataDefaults for ReflexiveC<T> {}

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
impl SimpleTagData for DataC {
    fn simple_size() -> usize {
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
}
impl TagDataDefaults for DataC {}

#[derive(Copy, Clone, Default, PartialEq)]
#[repr(transparent)]
pub struct ScenarioScriptNodeValue {
    pub data: u32
}
impl Display for ScenarioScriptNodeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("0x{:08X}", self.data))
    }
}
impl Debug for ScenarioScriptNodeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl From<bool> for ScenarioScriptNodeValue {
    fn from(value: bool) -> Self {
        Self::from(value as i8)
    }
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
        Self { data: value.to_bits() }
    }
}
impl From<ID> for ScenarioScriptNodeValue {
    fn from(value: ID) -> Self {
        Self { data: value.as_u32() }
    }
}
impl From<ScenarioScriptNodeValue> for bool {
    fn from(value: ScenarioScriptNodeValue) -> Self {
        i8::from(value) != 0
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
        f32::from_bits(value.data)
    }
}
impl From<ScenarioScriptNodeValue> for ID {
    fn from(value: ScenarioScriptNodeValue) -> Self {
        unsafe { std::mem::transmute(value.data) }
    }
}

impl SimpleTagData for ScenarioScriptNodeValue {
    fn simple_size() -> usize {
        u32::simple_size()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(Self { data: u32::read::<B>(data, at, struct_end)? })
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.data.write::<B>(data, at, struct_end)
    }
}

impl SimplePrimitive for ScenarioScriptNodeValue {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::ScenarioScriptNodeValue
    }
}

/// Calculate the amount of padding needed to pad to the next alignment.
///
/// # Examples
///
/// ```
/// use ringhopper_primitives::primitive::calculate_padding_for_alignment;
///
/// assert_eq!(127, calculate_padding_for_alignment(1, 128));
/// assert_eq!(64, calculate_padding_for_alignment(64, 128));
/// assert_eq!(0, calculate_padding_for_alignment(128, 128));
/// assert_eq!(0, calculate_padding_for_alignment(0, 128));
/// ```
pub fn calculate_padding_for_alignment(size: usize, alignment: usize) -> usize {
    debug_assert!(alignment != 0);
    (alignment - (size % alignment)) % alignment
}

#[cfg(test)]
mod test;
