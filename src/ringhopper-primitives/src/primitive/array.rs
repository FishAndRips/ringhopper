use std::any::Any;
use std::mem::MaybeUninit;
use byteorder::ByteOrder;
use crate::dynamic::*;
use crate::parse::*;
use crate::error::*;
use crate::map::{DomainType, Map};

/// Defines the lower and upper bound with fields.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Bounds<T: TagData> {
    /// Minimum value
    pub lower: T,

    /// Maximum value
    pub upper: T
}

impl<T: DynamicTagData> DynamicTagData for Bounds<T> {
    fn get_field(&self, item: &str) -> Option<&dyn DynamicTagData> {
        match item {
            "lower" => Some(&self.lower),
            "upper" => Some(&self.upper),
            _ => None
        }
    }

    fn get_field_mut(&mut self, item: &str) -> Option<&mut dyn DynamicTagData> {
        match item {
            "lower" => Some(&mut self.lower),
            "upper" => Some(&mut self.upper),
            _ => None
        }
    }

    fn fields(&self) -> &'static [&'static str] {
        &["lower", "upper"]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn data_type(&self) -> DynamicTagDataType {
        DynamicTagDataType::Block
    }
}

impl<T: TagData> TagData for Bounds<T> {
    fn size() -> usize {
        T::size() * 2
    }
    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        Ok(Self {
            lower: T::read_from_tag_file(data, at, struct_end, extra_data_cursor)?,
            upper: T::read_from_tag_file(data, at.add_overflow_checked(T::size())?, struct_end, extra_data_cursor)?
        })
    }
    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.lower.write_to_tag_file(data, at, struct_end)?;
        self.upper.write_to_tag_file(data, at.add_overflow_checked(T::size())?, struct_end)?;
        Ok(())
    }

    fn read_from_map<M: Map>(map: &M, address: usize, domain_type: &DomainType) -> RinghopperResult<Self> {
        Ok(Self {
            lower: T::read_from_map(map, address, domain_type)?,
            upper: T::read_from_map(map, address.add_overflow_checked(T::size())?, domain_type)?
        })
    }
}

impl<T: TagData + Default> Default for Bounds<T> {
    fn default() -> Self {
        Self {
            lower: T::default(),
            upper: T::default()
        }
    }
}

fn make_array<T: TagData + Sized, const U: usize, F: FnMut() -> RinghopperResult<T>>(mut next: F) -> RinghopperResult<[T; U]> {
    let mut error: Option<Error> = None;

    let mut uninited: [MaybeUninit<T>; U] = unsafe { MaybeUninit::uninit().assume_init() };
    let mut actual_count = 0;

    for elem in &mut uninited {
        let value = match next() {
            Ok(n) => n,
            Err(e) => {
                error = Some(e);
                break;
            }
        };
        elem.write(value);
        actual_count += 1;
    }

    if let Some(e) = error {
        for i in &mut uninited[0..actual_count] {
            unsafe { i.assume_init_drop() }
        }
        return Err(e);
    }

    // workaround: std::mem::transmute complains that it is not a fixed size even though it is
    unsafe {
        Ok(std::ptr::read(&uninited as *const _ as *const [T; U]))
    }
}

impl<T: DynamicTagData + SimpleTagData + Sized, const U: usize> DynamicTagData for [T; U] {
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
        DynamicTagDataType::Array
    }

    fn as_array(&self) -> Option<&dyn DynamicTagDataArray> {
        Some(self as &dyn DynamicTagDataArray)
    }

    fn as_array_mut(&mut self) -> Option<&mut dyn DynamicTagDataArray> {
        Some(self as &mut dyn DynamicTagDataArray)
    }
}

impl<T: DynamicTagData + SimpleTagData + Sized, const U: usize> SimpleTagData for [T; U] {
    fn simple_size() -> usize {
        T::simple_size() * U
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        let element_length = T::size();
        let mut at_offset = at;
        make_array(|| {
            let result = T::read::<B>(data, at_offset, struct_end)?;
            at_offset = at_offset.add_overflow_checked(element_length)?;
            Ok(result)
        })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        let element_length = T::size();
        let mut at_offset = at;

        for i in self {
            i.write::<B>(data, at_offset, struct_end)?;
            at_offset = at_offset.add_overflow_checked(element_length)?;
        }

        Ok(())
    }
}

impl<T: DynamicTagData + SimpleTagData + Sized, const U: usize> DynamicTagDataArray for [T; U] {
    fn get_at_index(&self, index: usize) -> Option<&dyn DynamicTagData> {
        Some(&self[index])
    }

    fn get_at_index_mut(&mut self, index: usize) -> Option<&mut dyn DynamicTagData> {
        Some(&mut self[index])
    }

    fn len(&self) -> usize {
        U
    }
}

#[cfg(test)]
mod test;
