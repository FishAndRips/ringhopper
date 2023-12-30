use std::any::Any;
use crate::dynamic::*;
use crate::parse::*;
use crate::error::*;

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
        self.lower.write_to_tag_file(data, 0, struct_end)?;
        self.upper.write_to_tag_file(data, at.add_overflow_checked(T::size())?, struct_end)?;
        Ok(())
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

impl<T: TagData + Sized + Default, const U: usize> TagData for [T; U] {
    fn size() -> usize {
        T::size() * U
    }
    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        let element_length = T::size();
        let mut at_offset = at;
        let mut error: RinghopperResult<()> = Ok(());

        // TODO: replace with std::array::try_from_fn when it is stabilized
        let s: Self = std::array::from_fn(|_| {
            let mut read_it = || -> RinghopperResult<T> {
                error?;

                let result = T::read_from_tag_file(data, at_offset, struct_end, extra_data_cursor)?;
                at_offset = at_offset.add_overflow_checked(element_length)?;
                Ok(result)
            };

            match read_it() {
                Ok(n) => n,
                Err(e) => {
                    error = Err(e);
                    Default::default()
                }
            }
        });

        error.map(|_| s)
    }
    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        let element_length = T::size();
        let mut at_offset = at;

        for i in 0..U {
            self[i].write_to_tag_file(data, at_offset, struct_end)?;
            at_offset = at_offset.add_overflow_checked(element_length)?;
        }

        Ok(())
    }
}

impl<T: DynamicTagData + Sized + Default, const U: usize> DynamicTagData for [T; U] {
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

impl<T: DynamicTagData + Sized + Default, const U: usize> DynamicTagDataArray for [T; U] {
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
