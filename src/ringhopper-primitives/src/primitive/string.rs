use std::any::Any;
use std::fmt::{Display, Debug, Formatter};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use crate::dynamic::{DynamicTagData, DynamicTagDataType, SimplePrimitiveType, TagFieldMetadata};
use crate::map::{DomainType, Map};
use crate::parse::{fits, SimplePrimitive, TagData, TagDataDefaults, U32SizeConversion};

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
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash)]
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

impl SimpleTagData for String32 {
    fn simple_size() -> usize {
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
impl SimplePrimitive for String32 {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::String32
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

#[derive(Clone)]
struct VerifiedString {
    data: Arc<String>,
    corrupt: bool
}

/// A UTF-16, null terminated string.
///
/// All characters are little endian, with no BOM.
pub struct UTF16String {
    cache: Mutex<Option<VerifiedString>>,
    data: Vec<u8>
}

impl UTF16String {
    /// Get a reference to the String.
    ///
    /// Internally, UTF16String uses a cache, so this is a reference to the string in the cache.
    pub fn get_string(&self) -> RinghopperResult<Arc<String>> {
        let result = self.try_parse();

        if result.corrupt {
            Err(Error::InvalidTagData("invalid UTF-16 data".to_owned()))
        }
        else {
            Ok(result.data)
        }
    }

    /// Get a string.
    ///
    /// If the string is invalid, the Err value will be the lossy data.
    pub fn get_string_lossy(&self) -> Result<Arc<String>, Arc<String>> {
        let result = self.try_parse();

        if result.corrupt {
            Err(result.data)
        }
        else {
            Ok(result.data)
        }
    }

    /// Create a UTF-16 string from a string reference.
    ///
    /// The return value is guaranteed to be a valid UTF-16 string.
    pub fn from_str(string: &str) -> Self {
        let mut data = Vec::with_capacity((string.len() + 1) * 2);
        for i in string.encode_utf16() {
            data.extend_from_slice(&i.to_le_bytes());
        }
        data.extend_from_slice(&[0,0]);

        let string_data = Self {
            data,
            cache: Mutex::new(None)
        };

        match string_data.get_string_lossy() {
            Ok(_) => string_data,
            Err(n) => Self::from_str(n.as_str())
        }
    }

    /// Get the bytes for the underlying UTF-16 data.
    pub fn get_utf16_bytes(&self) -> &[u8] {
        self.data.as_slice()
    }

    fn try_parse(&self) -> VerifiedString {
        let mut lock = self.cache.lock().unwrap();
        if let Some(n) = lock.as_ref() {
            return n.to_owned();
        }

        if self.data.len() % 2 != 0 || self.data.is_empty() {
            let result = VerifiedString {
                data: Arc::new(String::new()),
                corrupt: true,
            };
            *lock = Some(result.clone());
            return result;
        }

        let mut corrupted = false;

        let null_terminated = self.data.ends_with(&[0,0]);
        corrupted |= !null_terminated;

        let range_to_check = if null_terminated {
            &self.data[..self.data.len() - 2]
        }
        else {
            self.data.as_slice()
        };

        let chunks: Vec<u16> = range_to_check
            .chunks(2)
            .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
            .take_while(|&b| {
                if b == 0 {
                    corrupted = true;
                    false
                }
                else {
                    true
                }
            })
            .collect();

        let string = match String::from_utf16(&chunks) {
            Ok(n) => n,
            Err(_) => {
                corrupted = true;
                String::from_utf16_lossy(&chunks)
            }
        };

        let mut filtered_string = String::with_capacity(string.len());
        let mut iterator = string.chars().peekable();

        let mut last_character = None;
        while let Some(n) = iterator.next() {
            let afterwards = iterator.peek().map(|&p| p);

            match n {
                '\n' => if last_character != Some('\r') {
                    filtered_string.push('\r');
                    corrupted = true;
                }
                '\r' => if afterwards != Some('\n') {
                    corrupted = true;
                    continue;
                },
                '\t' => {},
                n => {
                    if n.is_ascii_control() {
                        corrupted = true;
                        continue;
                    }
                }
            }

            last_character = Some(n);
            filtered_string.push(n);
        }

        let result = VerifiedString {
            data: Arc::new(filtered_string),
            corrupt: corrupted,
        };
        *lock = Some(result.clone());
        result
    }
}

impl FromStr for UTF16String {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UTF16String::from_str(s))
    }
}

impl Clone for UTF16String {
    fn clone(&self) -> Self {
        Self {
            cache: Mutex::new(None),
            data: self.data.clone()
        }
    }
}

impl Default for UTF16String {
    fn default() -> Self {
        Self {
            cache: Mutex::new(Some(VerifiedString { data: Arc::new(String::new()), corrupt: false })),
            data: vec![0,0]
        }
    }
}

impl PartialEq for UTF16String {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Display for UTF16String {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.get_string_lossy() {
            Ok(n) => f.write_str(n.as_str()),
            Err(n) => f.write_str(n.as_str())
        }
    }
}

impl Debug for UTF16String {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fn cleanup_ctrl_chars(string: &str) -> String {
            string.to_string()
                .replace("\t", "\\t")
                .replace("\r", "\\r")
                .replace("\n", "\\n")
        }
        match self.get_string_lossy() {
            Ok(n) => write!(f, "`{}`", cleanup_ctrl_chars(n.as_str())),
            Err(n) => write!(f, "`{}` (CORRUPT)", cleanup_ctrl_chars(n.as_str()))
        }
    }
}

impl TagDataDefaults for UTF16String {}

impl DataData for UTF16String {
    fn from_bytes(bytes: &[u8]) -> RinghopperResult<Self> {
        Ok(Self { data: bytes.to_owned(), cache: Mutex::new(None) })
    }

    fn get_bytes(&self) -> &[u8] {
        self.data.as_ref()
    }
}

make_tag_data_implementation_for_datadata!(UTF16String);

impl DynamicTagData for UTF16String {
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
        DynamicTagDataType::UTF16String
    }
}

#[cfg(test)]
mod test;
