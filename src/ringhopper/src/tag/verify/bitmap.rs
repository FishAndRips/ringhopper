use definitions::{BitmapDataFormat, BitmapFormat, BitmapUsage};
use primitives::{error::OverflowCheck, primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::{Bitmap, BitmapType};
use crate::{primitives::dynamic::DynamicEnumImpl, tag::{bitmap::{bytes_per_block, MipmapFaceIterator}, tree::TagTree}};

use super::{ScenarioContext, ScenarioTreeTagResult};

pub enum SequenceType {
    Any,
    Sprite,
    Bitmap
}

pub fn verify_bitmap<T: TagTree + Send + Sync>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &ScenarioContext<T>, result: &mut ScenarioTreeTagResult) {
    let bitmap = tag.as_any().downcast_ref::<Bitmap>().unwrap();

    let error_count = result.errors.len();

    // Verify the data lines up with the format.
    for (i, data) in ziperator!(bitmap.bitmap_data) {
        if data.format == BitmapDataFormat::P8 {
            if bitmap.flags.disable_height_map_compression {
                result.errors.push(format!("Bitmap #{i} is {}, but the bitmap tag has height compression disabled", data.format));
            }
            continue;
        }
        else if data.format != BitmapDataFormat::P8 && !bitmap.flags.disable_height_map_compression {
            if bitmap.usage == BitmapUsage::HeightMap || bitmap.usage == BitmapUsage::VectorMap {
                result.errors.push(format!("Bitmap #{i} is {}, but the bitmap tag has height compression enabled", data.format));
                continue;
            }
        }

        let allowed_formats: &[BitmapDataFormat] = match bitmap.encoding_format {
            BitmapFormat::_16Bit => &[BitmapDataFormat::A1R5G5B5, BitmapDataFormat::A4R4G4B4, BitmapDataFormat::R5G6B5],
            BitmapFormat::_32Bit => &[BitmapDataFormat::A8R8G8B8, BitmapDataFormat::X8R8G8B8],
            BitmapFormat::DXT1 => &[BitmapDataFormat::DXT1],
            BitmapFormat::DXT3 => &[BitmapDataFormat::DXT1, BitmapDataFormat::DXT3],
            BitmapFormat::DXT5 => &[BitmapDataFormat::DXT1, BitmapDataFormat::DXT5],
            BitmapFormat::Monochrome => &[BitmapDataFormat::A8, BitmapDataFormat::Y8, BitmapDataFormat::AY8, BitmapDataFormat::A8Y8],
            BitmapFormat::BC7 => &[BitmapDataFormat::BC7]
        };

        if !allowed_formats.contains(&data.format) {
            result.errors.push(format!("Bitmap #{i} is {}, but the bitmap tag is set to {} which doesn't match", data.format, bitmap.encoding_format));
        }
    }

    if error_count != result.errors.len() {
        if bitmap.color_plate.compressed_data.bytes.is_empty() {
            result.errors.push("If the bitmap(s) are the correct format, you can change the encoding format of the bitmap. No source data is present in the tag, so it cannot be regenerated.".to_owned());
        }
        else {
            result.errors.push("This tag has source data; you can regenerate the bitmap(s). If they are already the correct format, you can instead change the encoding format of the bitmap tag to match.".to_owned());
        }
    }

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
    bitmap: &Bitmap,
    sequence_index: Option<u16>,
    minimum: usize,
    sequence_type: SequenceType
) -> Result<(), String> {
    match sequence_type {
        SequenceType::Any => (),
        SequenceType::Sprite => if bitmap._type != BitmapType::Sprites {
            return Err(format!("expected sprites, but bitmap is actually {}", bitmap._type.to_str()))
        },
        SequenceType::Bitmap => if bitmap._type != BitmapType::_2dTextures {
            return Err(format!("expected 2D textures, but bitmap is actually {}", bitmap._type.to_str()))
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
        SequenceType::Sprite => if seq.sprites.items.len() < minimum {
            return Err(format!("expected sequence index #{s} to have at least {minimum} sprite(s), found only {}", seq.sprites.items.len()))
        },
        SequenceType::Any => unreachable!()
    }

    Ok(())
}
