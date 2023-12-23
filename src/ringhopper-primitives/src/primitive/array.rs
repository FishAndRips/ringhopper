use crate::accessor::*;
use crate::parse::*;
use crate::error::*;

/// Defines the lower and upper bound with fields.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Bounds<T: TagData> {
    /// Minimum value
    pub from: T,

    /// Maximum value
    pub to: T
}

// TODO: implement TagDataAccessor for all types?

impl<T: TagData + TagDataAccessor> TagDataAccessor for Bounds<T> {
    fn access(&self, matcher: &str) -> Vec<AccessorResult> {
        if matcher.is_empty() {
            vec![AccessorResult::Accessor(self)]
        }
        else if matcher.starts_with(".from") {
            self.from.access(&matcher[".from".len()..])
        }
        else if matcher.starts_with(".to") {
            self.to.access(&matcher[".to".len()..])
        }
        else {
            vec![AccessorResult::Error("only .from and .to can be used".to_owned())]
        }
    }
    fn access_mut(&mut self, matcher: &str) -> Vec<AccessorResultMut> {
        if matcher.is_empty() {
            vec![AccessorResultMut::Accessor(self)]
        }
        else if matcher.starts_with(".from") {
            self.from.access_mut(&matcher[".from".len()..])
        }
        else if matcher.starts_with(".to") {
            self.to.access_mut(&matcher[".to".len()..])
        }
        else {
            vec![AccessorResultMut::Error("only .from and .to can be used".to_owned())]
        }
    }
    fn all_fields(&self) -> &'static [&'static str] {
        &["from", "to"]
    }
    fn get_type(&self) -> TagDataAccessorType {
        TagDataAccessorType::Block
    }
}

impl<T: TagData> TagData for Bounds<T> {
    fn size() -> usize {
        T::size() * 2
    }
    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        Ok(Self {
            from: T::read_from_tag_file(data, at, struct_end, extra_data_cursor)?,
            to: T::read_from_tag_file(data, at.add_overflow_checked(T::size())?, struct_end, extra_data_cursor)?
        })
    }
    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.from.write_to_tag_file(data, 0, struct_end)?;
        self.to.write_to_tag_file(data, at.add_overflow_checked(T::size())?, struct_end)?;
        Ok(())
    }
}

impl<T: TagData + TagDataAccessor + Default> Default for Bounds<T> {
    fn default() -> Self {
        Self {
            from: T::default(),
            to: T::default()
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

#[cfg(test)]
mod test;
