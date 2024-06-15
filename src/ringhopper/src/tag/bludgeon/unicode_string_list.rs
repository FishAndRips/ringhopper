use primitives::primitive::UTF16String;
use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::UnicodeStringList;

use super::BludgeonResult;

pub fn repair_unicode_string_list(tag: &mut dyn PrimaryTagStructDyn) -> BludgeonResult {
    let unicode_string_list: &mut UnicodeStringList = tag.as_any_mut().downcast_mut().unwrap();
    for string in unicode_string_list.strings.items.iter_mut() {
        let string = &mut string.string;
        if let Err(attempted_string) = string.get_string_lossy() {
            *string = UTF16String::from_str(attempted_string.as_str());
        }
    }

    BludgeonResult::Done
}
