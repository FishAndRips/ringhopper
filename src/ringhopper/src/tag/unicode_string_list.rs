use std::fmt::Formatter;
use definitions::{UnicodeStringList, UnicodeStringListString};
use primitives::dynamic::DynamicTagDataArray;
use primitives::primitive::{Data, Reflexive};

#[derive(Debug)]
pub enum UnicodeStringListError {
    InvalidStringData,
    MissingEndString
}

impl std::fmt::Display for UnicodeStringListError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEndString => f.write_str("an ###END-STRING### is missing at the end"),
            Self::InvalidStringData => f.write_str("invalid string data")
        }
    }
}

fn parse_string(string: &[u8]) -> Result<String, UnicodeStringListError> {
    if string.is_empty() {
        return Ok(String::new())
    }

    if string[0] > 0x80 {
        if string.len() % 2 != 0 {
            return Err(UnicodeStringListError::InvalidStringData)
        }

        let mut iterator = string.chunks(2).map(|c| [c[0], c[1]]);
        let bom = iterator.next().unwrap();

        let utf16_data = if u16::from_le_bytes(bom) == 0xFEFF {
            iterator.map(u16::from_le_bytes).collect::<Vec<u16>>()
        }
        else if u16::from_be_bytes(bom) == 0xFEFF {
            iterator.map(u16::from_be_bytes).collect::<Vec<u16>>()
        }
        else {
            return Err(UnicodeStringListError::InvalidStringData)
        };

        String::from_utf16(&utf16_data).ok()
    }
    else {
        String::from_utf8(string.to_vec()).ok()
    }.ok_or(UnicodeStringListError::InvalidStringData)
}

/// Helper methods for [`UnicodeStringList`] tags.
pub trait UnicodeStringListFunctions {
    /// Generate a string list tag from data input.
    ///
    /// `data` can be either a sequence of UTF-8 characters or UTF-16 in little/big endian with a BOM.
    ///
    /// Both CRLF (`\r\n`) and LF (`\n`) line endings are accepted. Control characters besides these line endings are not, however.
    ///
    /// An error of type [`UnicodeStringListError`] will be returned if this function fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper::tag::unicode_string_list::*;
    /// use ringhopper::definitions::UnicodeStringList;
    ///
    /// let result = UnicodeStringList::from_text_data(
    ///     "This is my string!\n###END-STRING###\nThis is another string!\nWow!\r\n###END-STRING###".as_bytes()
    /// ).expect("should have worked");
    ///
    /// assert_eq!(result.string_count(), 2);
    /// assert_eq!(result.read_string_data(0).expect("should be valid"), "This is my string!");
    /// assert_eq!(result.read_string_data(1).expect("should be valid"), "This is another string!\nWow!");
    /// ```
    fn from_text_data(data: &[u8]) -> Result<Self, UnicodeStringListError> where Self: Sized;

    /// Get the number of strings in the string list.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper::tag::unicode_string_list::*;
    /// use ringhopper::definitions::UnicodeStringList;
    ///
    /// let result = UnicodeStringList::default();
    /// assert_eq!(result.string_count(), 0);
    /// ```
    fn string_count(&self) -> usize;

    /// Access and parse a string at an index.
    ///
    /// An error of type [`UnicodeStringListError`] will be returned if the string at the index is invalid.
    ///
    /// Note that all line endings will be LF (`\n`).
    ///
    /// # Panics
    ///
    /// Panics if `index` >= `self.string_count()`
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper::tag::unicode_string_list::*;
    /// use ringhopper::definitions::{UnicodeStringList, UnicodeStringListString};
    /// use ringhopper::primitives::primitive::{Data, Reflexive};
    ///
    /// let result = UnicodeStringList {
    ///     strings: Reflexive::new(vec![
    ///         UnicodeStringListString {
    ///             string: Data::new(vec!['h' as u8, 0u8, 'e' as u8, 0u8, 'l' as u8, 0u8, 'l' as u8, 0u8, 'o' as u8, 0u8, 0u8, 0u8])
    ///         }
    ///     ]),
    ///     ..Default::default()
    /// };
    ///
    /// assert_eq!(result.read_string_data(0).expect("should be valid"), "hello");
    /// ```
    fn read_string_data(&self, index: usize) -> Result<String, UnicodeStringListError>;
}

impl UnicodeStringListFunctions for UnicodeStringList {
    fn from_text_data(data: &[u8]) -> Result<Self, UnicodeStringListError> {
        let parsed_string = parse_string(data)?;
        let mut string_data = parsed_string.lines().collect::<Vec<&str>>();

        while string_data.last().is_some_and(|l| l.is_empty()) {
            string_data.pop();
        }

        if string_data.is_empty() {
            return Ok(UnicodeStringList::default());
        }

        if string_data.pop().unwrap() != "###END-STRING###" {
            return Err(UnicodeStringListError::MissingEndString)
        }

        // Find any bad characters
        let illegal_character = string_data
            .iter()
            .map(|s| s.chars())
            .flatten()
            .any(|c| c.is_ascii_control());

        if illegal_character {
            return Err(UnicodeStringListError::InvalidStringData)
        }

        let strings = string_data
            .split(|line| *line == "###END-STRING###")
            .map(|lines| {
                let (last_line, other_lines) = match lines.split_last() {
                    Some(n) => (*n.0, n.1),
                    None => ("", &[] as &[&str])
                };

                other_lines.iter()
                    // intersperse is not stabilized so we have to do this manually
                    // (see https://github.com/rust-lang/rust/issues/79524)
                    .map(|&string| [string, "\r\n"])
                    .flatten()
                    .chain(std::iter::once(last_line))

                    // now convert into a u16 iterator
                    .map(|u| u.encode_utf16())
                    .flatten()

                    // add a null byte at the end
                    .chain(std::iter::once(0u16))

                    // now convert to u16 little endian bytes
                    .map(|c| c.to_le_bytes())
                    .flatten()
                    .collect::<Data>()
            })
            .map(|string| UnicodeStringListString { string })
            .collect::<Reflexive<UnicodeStringListString>>();

        Ok(UnicodeStringList { strings, ..Default::default() })
    }

    fn string_count(&self) -> usize {
        self.strings.len()
    }
    
    fn read_string_data(&self, index: usize) -> Result<String, UnicodeStringListError> {
        let bytes = &self.strings.items[index].string.bytes;
        if bytes.len() % 2 != 0 || bytes.is_empty() {
            return Err(UnicodeStringListError::InvalidStringData)
        }

        let mut utf16 = bytes
            .chunks(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .filter(|c| *c != '\r' as u16)
            .collect::<Vec<u16>>();

        // Must be null terminated!
        if utf16.pop() != Some(0) {
            return Err(UnicodeStringListError::InvalidStringData)
        }

        String::from_utf16(utf16.as_slice())
            .ok()
            .ok_or(UnicodeStringListError::InvalidStringData)
    }
}
