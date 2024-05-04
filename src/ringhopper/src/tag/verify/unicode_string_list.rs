use primitives::{primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::UnicodeStringList;
use crate::tag::tree::TagTree;
use crate::tag::unicode_string_list::*;
use super::{VerifyContext, VerifyResult};

pub fn verify_unicode_string_list<T: TagTree + Send + Sync>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &VerifyContext<T>, result: &mut VerifyResult) {
    let list: &UnicodeStringList = tag.as_any().downcast_ref().unwrap();

    for (i, string) in ziperator!(list.strings) {
        let bytes = string.string.bytes.as_slice();

        if (bytes.len() % 2) != 0 {
            result.errors.push(format!("String #{i} is not a 16-bit string (byte count is not divisible by 2)"));
            continue;
        }

        if bytes.is_empty() {
            result.errors.push(format!("String #{i} has no data"));
            continue;
        }

        let iterate_through_chars = || {
            bytes.chunks(2).map(|a: &[u8]| {
                (a[0] as u16) | ((a[1] as u16) << 8)
            })
        };

        let mut nulls = 0;
        let mut last_character = None;
        for i in iterate_through_chars() {
            if i == 0 {
                nulls += 1;
                if nulls > 1 {
                    result.errors.push(format!("String #{i} has multiple null bytes"));
                    break;
                }
            }

            if i == LF {
                if last_character != Some(CR) {
                    result.errors.push(format!("String #{i} has LF line endings with no matching CR"));
                    break;
                }
            }
            else if last_character == Some(CR) {
                result.errors.push(format!("String #{i} has CR line endings with no matching LF"));
                break;
            }

            last_character = Some(i);
        }

        if last_character != Some(0) {
            result.errors.push(format!("String #{i} is not null-terminated"));
        }
    }
}
