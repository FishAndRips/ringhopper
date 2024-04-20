use primitives::{error::OverflowCheck, primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::{Bitmap, BitmapType};
use crate::{primitives::dynamic::DynamicEnumImpl, tag::{bitmap::{bytes_per_block, MipmapFaceIterator}, tree::TagTree}};

use super::{VerifyContext, VerifyResult};

pub enum SequenceType {
    Any,
    Sprite,
    Bitmap
}

pub fn verify_bitmap<T: TagTree + Send + Sync>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &VerifyContext<T>, result: &mut VerifyResult) {
    let bitmap = tag.as_any().downcast_ref::<Bitmap>().unwrap();

    // Verify that each bitmap data doesn't overflow the bitmap data
    for (i, data) in ziperator!(bitmap.bitmap_data) {
        let iterator = match MipmapFaceIterator::new_from_bitmap_data(data) {
            Ok(n) => n,
            Err(e) => {
                result.errors.push(format!("Unable to check mipmaps for bitmap #{i}: {e}"));
                continue;
            }
        };

        let total_block_length = iterator
            .map(|i| i.block_count)
            .reduce(|a, b| a + b)
            .expect("should be able to get at least one mipmap when verifying bitmap data");

        let block_size = bytes_per_block(data.format);
        let offset = data.pixel_data_offset as usize;
        let range = total_block_length.mul_overflow_checked(block_size.get())
            .and_then(|byte_size| offset.add_overflow_checked(byte_size))
            .ok()
            .and_then(|end| bitmap.processed_pixel_data.bytes.get(offset..end));

        if range.is_none() {
            result.errors.push(format!("Bitmap data #{i} has out-of-bounds pixel data ({total_block_length} {block_size}-byte block(s) at offset {offset} out of {} total bytes)", bitmap.processed_pixel_data.bytes.len()));
        }
    }
}

pub fn verify_bitmap_sequence_index(
    bitmap: &mut Bitmap,
    sequence_index: Option<u16>,
    minimum: usize,
    sequence_type: SequenceType
) -> Result<(), String> {
    match sequence_type {
        SequenceType::Any => (),
        SequenceType::Sprite => if bitmap._type != BitmapType::Sprites {
            return Err(format!("expected sprites, but bitmap is actually a {}", bitmap._type.to_str()))
        },
        SequenceType::Bitmap => if bitmap._type != BitmapType::_2dTextures {
            return Err(format!("expected 2D textures, but bitmap is actually a {}", bitmap._type.to_str()))
        }
    }

    let s = match sequence_index {
        Some(n) => n as usize,
        None => return Ok(())
    };

    let seq = match bitmap.bitmap_group_sequence.items.get(s) {
        Some(n) => n,
        None => return Err(format!("sequence index #{s} is out-of-bounds (bitmap only has {} sequence(s))", bitmap.bitmap_group_sequence.items.len()))
    };

    let actual_sequence_type = match sequence_type {
        SequenceType::Any => if seq.sprites.items.is_empty() { SequenceType::Bitmap } else { SequenceType::Sprite }
        n => n
    };

    match actual_sequence_type {
        SequenceType::Bitmap => if (seq.bitmap_count as usize) < minimum {
            return Err(format!("expected sequence index #{s} to have at least {minimum} bitmap(s), found only {}", seq.bitmap_count))
        }
        SequenceType::Sprite => if (seq.sprites.items.len() as usize) < minimum {
            return Err(format!("expected sequence index #{s} to have at least {minimum} sprite(s), found only {}", seq.sprites.items.len()))
        },
        SequenceType::Any => unreachable!()
    }

    Ok(())
}
