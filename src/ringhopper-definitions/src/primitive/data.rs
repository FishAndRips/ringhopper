use std::ops::*;
use std::marker::PhantomData;
use crate::parse::*;
use crate::error::*;
use byteorder::*;
use std::fmt::Display;

#[derive(Copy, Clone, Default, PartialEq)]
#[repr(transparent)]
pub struct Padding<T: Sized> {
    internal: PhantomData<T>
}

impl<T> TagDataSimplePrimitive for Padding<T> {
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
            item_offset += item_size;
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

#[cfg(test)]
mod test;
