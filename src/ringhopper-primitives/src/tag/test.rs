use crate::parse::*;
use super::*;

#[derive(Default)]
struct UnicodeStringList {
    pub strings: Reflexive<String>
}

#[derive(Default)]
struct String {
    pub string: Data
}

// TODO: Replace this whole implementation with generated code when that's finished
impl TagDataAccessor for UnicodeStringList {
    fn access(&self, matcher: &str) -> Vec<AccessorResult> {
        if matcher == "" {
            return vec![AccessorResult::Accessor(self)]
        }
        if matcher == ".strings" {
            return vec![AccessorResult::Accessor(&self.strings)]
        }
        assert!(matcher.starts_with(".strings"));
        self.strings.access(&matcher[".strings".len()..])
    }
    fn access_mut(&mut self, _matcher: &str) -> Vec<AccessorResultMut> {
        unimplemented!()
    }
    fn all_fields(&self) -> &'static [&'static str] {
        &["strings"]
    }
    fn get_type(&self) -> TagDataAccessorType {
        TagDataAccessorType::Block
    }
}

impl TagDataAccessor for String {
    fn access(&self, matcher: &str) -> Vec<AccessorResult> {
        if matcher == "" {
            return vec![AccessorResult::Accessor(self)]
        }
        assert!(matcher.starts_with(".string"));
        return vec![AccessorResult::Primitive(PrimitiveRef::Data(&self.string))]
    }
    fn access_mut(&mut self, _matcher: &str) -> Vec<AccessorResultMut> {
        unimplemented!()
    }
    fn all_fields(&self) -> &'static [&'static str] {
        &["string"]
    }
    fn get_type(&self) -> TagDataAccessorType {
        TagDataAccessorType::Block
    }
}

impl TagData for String {
    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> crate::error::RinghopperResult<Self> {
        Ok(Self { string: Data::read_from_tag_file(data, at, struct_end, extra_data_cursor)? })
    }
    fn size() -> usize {
        Data::size()
    }
    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> crate::error::RinghopperResult<()> {
        self.string.write_to_tag_file(data, at, struct_end)
    }
}

impl TagData for UnicodeStringList {
    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> crate::error::RinghopperResult<Self> {
        Ok(Self { strings: Reflexive::<String>::read_from_tag_file(data, at, struct_end, extra_data_cursor)? })
    }
    fn size() -> usize {
        Reflexive::<String>::size()
    }
    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> crate::error::RinghopperResult<()> {
        self.strings.write_to_tag_file(data, at, struct_end)
    }
}

impl PrimaryTagStruct for UnicodeStringList {
    fn group() -> TagGroup where Self: Sized {
        TagGroup::UnicodeStringList
    }
    fn version() -> u16 {
        1
    }
}

fn read_test_unicode_string_list() -> (&'static [u8], UnicodeStringList) {
    let data = include_bytes!("test.unicode_string_list");
    (data, TagFile::read_tag_from_file_buffer::<UnicodeStringList>(data, ParseStrictness::Strict).expect("should be valid"))
}

#[test]
fn parse_unicode_string_list() {
    let (data, string_list) = read_test_unicode_string_list();
    let string_list_accessor = string_list.as_accessor();

    let utf16_matches = |string_index: usize, expected: &str| {
        let mut data: Vec<u8> = expected.encode_utf16().map(|f| f.to_le_bytes()).flatten().collect();
        data.push(0);
        data.push(0);

        assert_eq!(string_list.strings.items[string_index].string.bytes.as_ref(), data);

        // Also try with the accessor
        let v = string_list_accessor.access(format!(".strings[{string_index}].string").as_str());
        let result = match &v[0] {
            AccessorResult::Primitive(PrimitiveRef::Data(data)) => *data,
            AccessorResult::Error(e) => panic!("Error: {}", e.as_str()),
            _ => panic!("could not access it!")
        };
        assert_eq!(result.bytes.as_slice(), data.as_slice());
    };

    utf16_matches(0, "This is a test string.");
    utf16_matches(1, "This is another test string.\r\nAnd it has multiple lines!");
    utf16_matches(2, "And this is one final test string.");
    utf16_matches(3, "");
    utf16_matches(4, "Okay, this is the actual test string. I wanted to add an empty one, too.");

    // If we convert it back to a tag file, it should match.
    let reparsed = TagFile::to_tag_file(&string_list).unwrap();
    assert_eq!(data, reparsed.as_slice());
}
