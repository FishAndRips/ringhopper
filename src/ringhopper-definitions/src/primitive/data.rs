use std::ops::*;
use std::marker::PhantomData;
use crate::accessor::*;
use crate::parse::*;
use crate::error::*;
use byteorder::*;
use std::fmt::Display;

#[derive(Copy, Clone, Default, PartialEq)]
#[repr(transparent)]
pub struct Padding<T: Sized> {
    internal: PhantomData<T>
}

impl<T: Copy> TagDataSimplePrimitive for Padding<T> {
    fn size() -> usize {
        std::mem::size_of::<T>()
    }
    fn read<B: ByteOrder>(_: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        fits(<Self as TagDataSimplePrimitive>::size(), at, struct_end)?;
        Ok(Padding { internal: PhantomData::default() })
    }
    fn write<B: ByteOrder>(&self, _: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        fits(<Self as TagDataSimplePrimitive>::size(), at, struct_end).expect("should fit");
        Ok(())
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
        5 * <u32 as TagDataSimplePrimitive>::size()
    }

    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        let size = usize::read_from_tag_file(data, at, struct_end, extra_data_cursor)?;
        let data_location = *extra_data_cursor;
        fits(size, data_location, data.len())?;
        *extra_data_cursor += size;

        Ok(Data {
            bytes: data[data_location..*extra_data_cursor].to_owned()
        })
    }

    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.len().write_to_tag_file(data, at, struct_end)?;
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
        3 * <u32 as TagDataSimplePrimitive>::size()
    }

    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        let count = usize::read_from_tag_file(data, at, struct_end, extra_data_cursor)?;

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
        self.len().write_to_tag_file(data, at, struct_end)?;

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

#[cfg(test)]
mod test;
