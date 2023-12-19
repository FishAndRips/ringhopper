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
    fn access(&self, _matcher: &str) -> Vec<AccessorResult> {
        todo!()
    }
    fn access_mut(&mut self, _matcher: &str) -> Vec<AccessorResultMut> {
        todo!()
    }
    fn all_fields(&self) -> &'static [&'static str] {
        todo!()
    }
    fn get_type(&self) -> TagDataAccessorType {
        TagDataAccessorType::Block
    }
}

impl TagDataAccessor for String {
    fn access(&self, _matcher: &str) -> Vec<AccessorResult> {
        todo!()
    }
    fn access_mut(&mut self, _matcher: &str) -> Vec<AccessorResultMut> {
        todo!()
    }
    fn all_fields(&self) -> &'static [&'static str] {
        todo!()
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

impl PrimaryTagStruct for UnicodeStringList {}

impl PrimaryTagStructGroup for UnicodeStringList {
    fn fourcc() -> FourCC {
        TagGroup::UnicodeStringList.as_fourcc()
    }
    fn version() -> u16 {
        1
    }
}

#[test]
fn parse_unicode_string_list() {
    let data = include_bytes!("test.unicode_string_list");
    let string_list: UnicodeStringList = TagFile::read_tag_file::<UnicodeStringList>(data, ParseStrictness::Strict).expect("should be valid").take();

    let utf16_matches = |string_index: usize, expected: &str| {
        let mut data: Vec<u8> = expected.encode_utf16().map(|f| [(f & 0xFF) as u8, ((f & 0xFF00) >> 8) as u8]).flatten().collect();
        data.push(0);
        data.push(0);
        assert_eq!(string_list.strings[string_index].string.as_ref(), data);
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
