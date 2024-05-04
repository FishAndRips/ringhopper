use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::UnicodeStringList;
use crate::tag::unicode_string_list::*;

use super::BludgeonResult;

pub fn repair_unicode_string_list(tag: &mut dyn PrimaryTagStructDyn) -> BludgeonResult {
    let unicode_string_list: &mut UnicodeStringList = tag.as_any_mut().downcast_mut().unwrap();

    for string in unicode_string_list.strings.items.iter_mut().map(|c| &mut c.string.bytes) {
        // Bad string length - do nothing
        if string.len() % 2 == 1 {
            return BludgeonResult::CannotRepair
        }

        let mut characters: Vec<u16> = string.chunks(2).map(|c| u16::from_le_bytes(c.try_into().unwrap())).collect();
        if characters.is_empty() {
            *string = vec![0u8; 2];
            continue;
        }

        if !characters.ends_with(&[0]) {
            characters.push(0);
        }

        let mut last_character = None;
        let mut i = 0usize;

        while i < characters.len() {
            let character = characters[i];
            let next_character = characters.get(i + 1).map(|c| *c);

            if character == CR && next_character != Some(LF) {
                characters.remove(i);
                continue;
            }

            if character == LF && last_character != Some(CR) {
                characters.insert(i, CR);
            }

            last_character = Some(character);
            i += 1;
        }
    }

    BludgeonResult::Done
}
