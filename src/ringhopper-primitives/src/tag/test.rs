use crate::map::{DomainType, Map};
use crate::parse::*;
use super::*;

#[derive(Default)]
struct UnicodeStringList {
    pub metadata: PrimaryTagStructMetadata,
    pub strings: Reflexive<String>
}

#[derive(Default)]
struct String {
    pub string: Data
}

impl TagData for String {
    fn size() -> usize {
        Data::size()
    }
    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        Ok(Self { string: Data::read_from_tag_file(data, at, struct_end, extra_data_cursor)? })
    }
    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.string.write_to_tag_file(data, at, struct_end)
    }

    fn read_from_map<M: Map>(_map: &M, _address: usize, _domain_type: &DomainType) -> RinghopperResult<Self> where Self: Sized {
        unimplemented!()
    }
}

impl TagData for UnicodeStringList {
    fn size() -> usize {
        Reflexive::<String>::size()
    }
    fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {
        Ok(Self { metadata: Default::default(), strings: Reflexive::<String>::read_from_tag_file(data, at, struct_end, extra_data_cursor)? })
    }
    fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.strings.write_to_tag_file(data, at, struct_end)
    }

    fn read_from_map<M: Map>(_map: &M, _address: usize, _domain_type: &DomainType) -> RinghopperResult<Self> where Self: Sized {
        unimplemented!()
    }
}

impl TagDataDefaults for String {}
impl TagDataDefaults for UnicodeStringList {}

impl DynamicTagData for UnicodeStringList {
    fn get_field(&self, _field: &str) -> Option<&dyn DynamicTagData> {
        todo!()
    }

    fn get_field_mut(&mut self, _field: &str) -> Option<&mut dyn DynamicTagData> {
        todo!()
    }

    fn get_metadata_for_field(&self, _field: &str) -> Option<TagFieldMetadata> {
        todo!()
    }

    fn fields(&self) -> &'static [&'static str] {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        todo!()
    }

    fn data_type(&self) -> DynamicTagDataType {
        DynamicTagDataType::Block
    }
}

impl PrimaryTagStruct for UnicodeStringList {
    fn group() -> TagGroup where Self: Sized {
        TagGroup::UnicodeStringList
    }

    fn version() -> u16 where Self: Sized {
        1
    }

    fn metadata(&self) -> &PrimaryTagStructMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut PrimaryTagStructMetadata {
        &mut self.metadata
    }
}

fn read_test_unicode_string_list() -> (&'static [u8], UnicodeStringList) {
    let data = include_bytes!("test.unicode_string_list");
    (data, TagFile::read_tag_from_file_buffer::<UnicodeStringList>(data, ParseStrictness::Strict).expect("should be valid"))
}

#[test]
fn parse_unicode_string_list() {
    let (data, string_list) = read_test_unicode_string_list();

    let utf16_matches = |string_index: usize, expected: &str| {
        let mut data: Vec<u8> = expected.encode_utf16().map(|f| f.to_le_bytes()).flatten().collect();
        data.push(0);
        data.push(0);

        assert_eq!(string_list.strings.items[string_index].string.bytes.as_ref(), data);
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
