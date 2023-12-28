use std::fmt::{Display, Debug};

use super::*;

fn strlen(bytes: &[u8; 32]) -> usize {
    for i in 0..bytes.len() {
        if bytes[i] == 0 {
            return i
        }
    }
    bytes.len()
}

/// A 32-byte null-terminated string.
#[derive(Copy, Clone, Default, PartialEq)]
#[repr(transparent)]
pub struct String32 {
    string_data: [u8; 32] // NOTE: ALWAYS UTF-8, NULL-TERMINATED!
}

impl String32 {
    /// Convert the string to a string32.
    ///
    /// Returns [`Error::String32SizeLimitExceeded`] if the string is longer than 31 bytes.
    pub fn from_str(str: &str) -> RinghopperResult<Self> {
        let bytes = str.as_bytes();
        let strlen = bytes.len();

        let mut output = String32::default();
        if strlen >= output.string_data.len() {
            return Err(Error::String32SizeLimitExceeded)
        }

        output.string_data[0..strlen].copy_from_slice(bytes);
        Ok(output)
    }

    /// Convert an array of 32 bytes to String32.
    ///
    /// Invalid characters are replaced with `_`
    pub fn from_bytes_lossy(bytes: &[u8; 32]) -> String32 {
        use std::borrow::Cow;

        let mut string_data = *bytes;
        let mut len = strlen(&string_data);
        if len == bytes.len() {
            len -= 1;
        }
        string_data.split_at_mut(len).1.fill(0); // clean the string in case there are lingering non-zero bytes

        let cstr = std::ffi::CStr::from_bytes_until_nul(&string_data).unwrap();
        match cstr.to_string_lossy() {
            Cow::Borrowed(_) => String32 { string_data },
            Cow::Owned(n) => String32::from_str(&n.replace(char::REPLACEMENT_CHARACTER, "_")).unwrap()
        }
    }

    /// Get the string from this.
    pub fn as_str(&self) -> &str {
        // SAFETY: We uphold this invariant in `from_bytes_lossy` by replacing invalid characters. And `from_str` assumes a valid str.
        //
        // Also strlen() never goes outside the bounds here.
        let string_length = self.strlen();
        unsafe {
            debug_assert!(self.string_data.get(0..string_length).is_some());
            let bytes = self.string_data.get_unchecked(0..string_length);

            debug_assert!(std::str::from_utf8(bytes).is_ok());
            std::str::from_utf8_unchecked(bytes)
        }
    }

    /// Get the length of the string.
    fn strlen(&self) -> usize {
        strlen(&self.string_data)
    }
}

impl TagDataSimplePrimitive for String32 {
    fn size() -> usize {
        32
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        tag_data_fits::<Self>(at, struct_end, data.len())?;
        Ok(String32::from_bytes_lossy(&data[at..at+32].try_into().unwrap()))
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        tag_data_fits::<Self>(at, struct_end, data.len()).expect("should fit");
        data[at..at+32].clone_from_slice(&self.string_data[..]);
        Ok(())
    }
}

impl Display for String32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl Debug for String32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl PartialEq<str> for String32 {
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl PartialEq<String> for String32 {
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl PartialEq<String32> for &str {
    fn eq(&self, other: &String32) -> bool {
        PartialEq::eq(other.as_str(), *self)
    }
}

impl PartialEq<String32> for String {
    fn eq(&self, other: &String32) -> bool {
        PartialEq::eq(other.as_str(), self)
    }
}

#[cfg(test)]
mod test;
