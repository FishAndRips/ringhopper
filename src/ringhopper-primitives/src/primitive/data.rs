use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::*;
use crate::accessor::*;
use crate::parse::*;
use crate::error::*;
use byteorder::*;
use std::fmt::Display;

/// 16-bit index type
pub type Index = u16;

#[derive(Copy, Clone, Default, PartialEq)]
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
    /// let value = ID::from_index(0x1234, 0x5678);
    /// assert_eq!(value.index().unwrap(), 0x1234);
    /// assert_eq!(value.salt().unwrap(), 0x5678);
    /// ```
    pub const fn from_index(index: Index, salt: u16) -> Self {
        let id = (((salt ^ index) as u32) << 16) | (index as u32);
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
    /// let value = ID::from_null();
    /// assert!(value.is_null());
    /// ```
    pub const fn from_null() -> Self {
        ID { id: 0xFFFFFFFF }
    }

    /// Get the index value of the ID.
    ///
    /// Returns `None` if the ID is null.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ringhopper_primitives::primitive::ID;
    ///
    /// let value = ID::from_index(0x1234, 0x5678).index().expect("not null");
    /// assert_eq!(value, 0x1234);
    /// ```
    pub const fn index(self) -> Option<Index> {
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
    /// let value = ID::from_index(0x1234, 0x5678).salt().expect("not null");
    /// assert_eq!(value, 0x5678);
    /// ```
    pub const fn salt(self) -> Option<u16> {
        match self.index() {
            Some(index) => Some(((self.id ^ ((index as u32) << 16)) >> 16) as u16),
            None => None
        }
    }

    /// Convert into a [`u32`].
    pub const fn as_u32(self) -> u32 {
        self.id
    }

    /// Convert from a [`u32`].
    pub const fn from_u32(id: u32) -> Self {
        Self { id }
    }

    /// Return `true` if the ID is null.
    pub const fn is_null(self) -> bool {
        self.id == 0xFFFFFFFF
    }
}

impl TagDataSimplePrimitive for ID {
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(<u32 as TagDataSimplePrimitive>::read::<B>(data, at, struct_end)?.into())
    }
    fn size() -> usize {
        <u32 as TagDataSimplePrimitive>::size()
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        let id: u32 = (*self).into();
        id.write::<B>(data, at, struct_end)
    }
}

impl From<u32> for ID {
    fn from(value: u32) -> Self {
        ID::from_u32(value)
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
}

impl<T: Copy + Default> Debug for Padding<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} bytes padding)", <Self as TagDataSimplePrimitive>::size())
    }
}

#[derive(Clone, Default, PartialEq)]
#[repr(transparent)]
pub struct Data {
    pub bytes: Vec<u8>
}

impl Deref for Data {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl DerefMut for Data {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}

impl TagData for Data {
    fn size() -> usize {
        <DataC as TagData>::size()
    }

    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        let c_primitive = DataC::read_from_tag_file(data, at, struct_end, extra_data_cursor)?;

        let size = c_primitive.size as usize;
        let data_location = *extra_data_cursor;
        fits(size, data_location, data.len())?;
        *extra_data_cursor += size;

        Ok(Data {
            bytes: data[data_location..*extra_data_cursor].to_owned()
        })
    }

    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        (DataC {
            size: self.len().into_u32()?,
            ..Default::default()
        }).write_to_tag_file(data, at, struct_end)?;
        data.extend_from_slice(self);
        Ok(())
    }
}


#[derive(Clone, Default, PartialEq)]
#[repr(transparent)]
pub struct Reflexive<T: TagData + Sized> {
    pub items: Vec<T>
}

impl<T: TagData + Sized> Deref for Reflexive<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T: TagData + Sized> DerefMut for Reflexive<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
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
            result.push(T::read_from_tag_file(data, item_offset, struct_end, extra_data_cursor)?);
            item_offset = struct_end;
        }

        Ok(result)
    }

    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        ReflexiveC::<T>::with_params(self.len().into_u32()?, Address::default()).write_to_tag_file(data, at, struct_end)?;

        let item_size = T::size();
        let total_bytes_to_write = self.len().mul_overflow_checked(item_size)?;

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

generate_tag_data_simple_primitive_code!(Address, u32, address);

type ReflexiveAccessRange = (usize, usize); // [start, end]
fn parse_range(matcher: &str, len: usize) -> Result<Vec<ReflexiveAccessRange>, &'static str> {
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


impl<T: TagData + TagDataAccessor> Reflexive<T> {
    /// Separate the [] from the rest of the matcher, returning the matcher after the [] and range(s).
    fn parse_matcher<'a>(&self, matcher: &'a str) -> Result<(&'a str, Vec<ReflexiveAccessRange>), String> {
        if !matcher.is_ascii() {
            return Err(format!("invalid matcher {matcher}: must be ASCII"))
        }

        let mut chars = matcher.bytes();
        if chars.next() != Some('[' as u8) {
            return Err(format!("invalid matcher {matcher}: must be .length or []"))
        }
        let mut closer = None;
        let mut index: usize = 1;
        for c in chars {
            if c == ']' as u8 {
                closer = Some(index);
                break;
            }
            index += 1;
        }
        let closer = match closer {
            Some(n) => n,
            None => return Err(format!("invalid matcher {matcher}: unclosed ["))
        };

        let remaining = &matcher[closer + 1..];
        let matcher_to_parse = &matcher[1..closer];

        let ranges = match parse_range(matcher_to_parse, self.len()) {
            Ok(r) => r,
            Err(err) => return Err(format!("invalid matcher {matcher}: {err}"))
        };

        Ok((remaining, ranges))
    }
}

impl<T: TagData + TagDataAccessor> TagDataAccessor for Reflexive<T> {
    fn access(&self, matcher: &str) -> Vec<AccessorResult> {
        if matcher.is_empty() {
            return vec![AccessorResult::Accessor(self)]
        }
        if matcher == ".length" {
            return vec![AccessorResult::Size(self.len())]
        }
        let (remaining, ranges) = match self.parse_matcher(matcher) {
            Ok(n) => n,
            Err(e) => return vec![AccessorResult::Error(e)]
        };

        let mut matches = Vec::new();
        for (start, end) in ranges {
            for q in start..=end {
                matches.append(&mut self.get(q)
                                        .expect("we just verified this in parse_range!")
                                        .access(remaining))
            }
        }

        matches
    }
    fn access_mut(&mut self, matcher: &str) -> Vec<AccessorResultMut> {
        if matcher.is_empty() {
            return vec![AccessorResultMut::Accessor(self)]
        }
        if matcher == ".length" {
            return vec![AccessorResultMut::Size(self.len())]
        }
        let (remaining, ranges) = match self.parse_matcher(matcher) {
            Ok(n) => n,
            Err(e) => return vec![AccessorResultMut::Error(e)]
        };

        let mut matches: Vec<AccessorResultMut> = Vec::new();
        let mut index: usize = 0;
        for m in self.iter_mut() {
            'search_loop: for &(start, end) in &ranges {
                if start >= index && end <= index {
                    matches.append(&mut m.access_mut(remaining));
                    break 'search_loop;
                }
            }
            index += 1;
        }

        matches
    }
    fn all_fields(&self) -> &'static [&'static str] {
        &["length", "[]"]
    }
    fn get_type(&self) -> TagDataAccessorType {
        TagDataAccessorType::Reflexive
    }
}

/// Lower level C implementation of a reflexive.
///
/// The <T> refers to what the reflexive is supposed to point to, which can be useful.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct ReflexiveC<T: TagData + Sized> {
    pub count: u32,
    pub address: Address,
    pub padding: Padding<[u8; 4]>,

    phantom: PhantomData<T>
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
}

impl<T: TagData + Sized> ReflexiveC<T> {
    pub fn with_params(count: u32, address: Address) -> Self {
        Self {
            count,
            address,
            padding: Padding::default(),
            phantom: PhantomData::default()
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
    pub size: u32,
    pub external: u32,
    pub file_offset: u32,
    pub padding: Padding<[u8; 4]>,
    pub address: Address
}
impl TagDataSimplePrimitive for DataC {
    fn size() -> usize {
        std::mem::size_of::<Self>()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        let size = u32::read::<B>(data, at, struct_end)?;
        let external = u32::read::<B>(data, at + 0x4, struct_end)?;
        let file_offset = u32::read::<B>(data, at + 0x8, struct_end)?;
        let address = Address::read::<B>(data, at + 0xC, struct_end)?;
        Ok(Self { size, external, file_offset, address, ..Default::default() })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.size.write::<B>(data, at, struct_end)?;
        self.external.write::<B>(data, at + 0x4, struct_end)?;
        self.file_offset.write::<B>(data, at + 0x8, struct_end)?;
        self.padding.write::<B>(data, at + 0xC, struct_end)?;
        self.address.write::<B>(data, at + 0x10, struct_end)?;
        Ok(())
    }
}

#[cfg(test)]
mod test;
