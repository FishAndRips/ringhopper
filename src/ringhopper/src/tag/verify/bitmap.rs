use ringhopper_structs::{Bitmap, BitmapType};
use crate::primitives::dynamic::DynamicEnumImpl;

pub enum SequenceType {
    Any,
    Sprite,
    Bitmap
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
