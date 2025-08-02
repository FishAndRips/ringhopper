#[cfg(test)]
mod test;
mod swizzle;

pub use swizzle::*;

use std::iter::FusedIterator;
use std::num::NonZeroUsize;
use flate2::FlushDecompress;
use definitions::{Bitmap, BitmapData, BitmapDataFormat, BitmapDataType};
use primitives::byteorder::{BigEndian, LittleEndian};
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::parse::SimpleTagData;
use primitives::primitive::Pixel32;
use crate::data::bitmap::Image;

/// Get the number of pixels per block length for the given bitmap format.
///
/// Multiply this value by itself to get the total number of pixels per block.
pub fn pixels_per_block_length(format: BitmapDataFormat) -> NonZeroUsize {
    match format {
        BitmapDataFormat::DXT1
        | BitmapDataFormat::DXT3
        | BitmapDataFormat::DXT5
        | BitmapDataFormat::BC7 => unsafe { NonZeroUsize::new_unchecked(4) },

        BitmapDataFormat::A8
        | BitmapDataFormat::Y8
        | BitmapDataFormat::AY8
        | BitmapDataFormat::A8Y8
        | BitmapDataFormat::R5G6B5
        | BitmapDataFormat::A1R5G5B5
        | BitmapDataFormat::A4R4G4B4
        | BitmapDataFormat::A8R8G8B8
        | BitmapDataFormat::X8R8G8B8
        | BitmapDataFormat::P8 => unsafe { NonZeroUsize::new_unchecked(1) },
    }
}

/// Get the bits per pixel for the given bitmap format.
pub fn bits_per_pixel(format: BitmapDataFormat) -> NonZeroUsize {
    match format {
        BitmapDataFormat::DXT1 => unsafe { NonZeroUsize::new_unchecked(4) }

        BitmapDataFormat::DXT3
        | BitmapDataFormat::DXT5
        | BitmapDataFormat::BC7 => unsafe { NonZeroUsize::new_unchecked(8) },

        BitmapDataFormat::P8
        | BitmapDataFormat::A8
        | BitmapDataFormat::Y8
        | BitmapDataFormat::AY8 => unsafe { NonZeroUsize::new_unchecked(8) },

        | BitmapDataFormat::A8Y8
        | BitmapDataFormat::R5G6B5
        | BitmapDataFormat::A1R5G5B5
        | BitmapDataFormat::A4R4G4B4 => unsafe { NonZeroUsize::new_unchecked(16) },

        BitmapDataFormat::A8R8G8B8
        | BitmapDataFormat::X8R8G8B8 => unsafe { NonZeroUsize::new_unchecked(32) },
    }
}

/// Get the total number of bytes per block for the given format.
pub fn bytes_per_block(format: BitmapDataFormat) -> NonZeroUsize {
    let ppb = pixels_per_block_length(format).get();
    let bpp = bits_per_pixel(format).get();

    unsafe { NonZeroUsize::new_unchecked(ppb * ppb * bpp / 8) }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MipmapMetadata {
    /// Current mipmap index.
    ///
    /// 0 = base map, 1 onwards = mipmap number
    pub mipmap_index: usize,

    /// Current face index.
    ///
    /// For cubemaps, this is 0-5, for 3D textures this is 0-depth, and for 2D textures, this is always 0.
    pub face_index: usize,

    /// Width in pixels.
    ///
    /// This is the width of the face, itself.
    pub width: usize,

    /// Height in pixels.
    ///
    /// This is the height of the face, itself.
    pub height: usize,

    /// Depth in bitmaps.
    ///
    /// If iterating by faces, this is however many bitmaps are left for the current depth.
    ///
    /// Otherwise, this is the total depth of the current texture.
    pub depth: usize,

    /// Current offset in blocks.
    pub block_offset: usize,

    /// Current width in blocks.
    pub block_width: usize,

    /// Current height in blocks.
    pub block_height: usize,

    /// Number of blocks for the face or mipmap.
    pub block_count: usize,
}

impl MipmapMetadata {
    fn fixup_real_dims(&mut self, block_length: NonZeroUsize) {
        let block_length = block_length.get();
        self.block_width = (self.width + (block_length - 1)) / block_length;
        self.block_height = (self.height + (block_length - 1)) / block_length;
        self.block_count = self.block_width * self.block_height;
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum MipmapType {
    /// 2D
    TwoDimensional,

    /// 3D with depth
    ThreeDimensional(NonZeroUsize),

    /// Cubemap
    Cubemap
}

impl MipmapType {
    /// Get a mipmap type from a [`BitmapData`] object.
    pub fn get_mipmap_type(data: &BitmapData) -> RinghopperResult<MipmapType> {
        match data._type {
            BitmapDataType::CubeMap => Ok(MipmapType::Cubemap),
            BitmapDataType::_2dTexture => Ok(MipmapType::TwoDimensional),
            BitmapDataType::_3dTexture => NonZeroUsize::new(data.depth as usize)
                .map(|z| MipmapType::ThreeDimensional(z))
                .ok_or_else(|| Error::InvalidTagData("depth is 0".to_string())),
        }
    }
}

/// Iterates bitmap by textures.
///
/// Unlike [`MipmapFaceIterator`], this does not iterate through each individual face of a mipmap, but rather the whole
/// mipmap.
///
/// For example, a cubemap will contain all faces, and a 3D texture will contain the full depth.
#[derive(Copy, Clone)]
pub struct MipmapTextureIterator {
    inner: MipmapFaceIterator
}

impl MipmapTextureIterator {
    /// Instantiate a new iterator.
    pub fn new(
        width: NonZeroUsize,
        height: NonZeroUsize,
        bitmap_type: MipmapType,
        block_length: NonZeroUsize,
        mipmap_count: Option<usize>
    ) -> Self {
        Self { inner: MipmapFaceIterator::new(width, height, bitmap_type, block_length, mipmap_count) }
    }

    /// Initialize a new iterator from a `BitmapData` instance.
    ///
    /// Returns `Err` if the bitmap data is malformed.
    pub fn new_from_bitmap_data(data: &BitmapData) -> RinghopperResult<Self> {
        MipmapFaceIterator::new_from_bitmap_data(data).map(|inner| Self { inner })
    }
}

/// Iterates bitmap by faces.
///
/// Unlike [`MipmapTextureIterator`], this will iterate through each individual face of a mipmap.
///
/// For example, a cubemap will yield one face per iteration (this 6 iterations to go through the full mipmap), and a
/// 3D texture will contain only one level of depth per iteration.
#[derive(Copy, Clone)]
pub struct MipmapFaceIterator {
    next: Option<MipmapMetadata>,
    block_length: NonZeroUsize,
    bitmap_type: MipmapType,
    mipmap_count: Option<usize>,
    face_count: usize
}

impl MipmapFaceIterator {
    /// Initialize a new iterator.
    pub fn new(
        width: NonZeroUsize,
        height: NonZeroUsize,
        bitmap_type: MipmapType,
        block_length: NonZeroUsize,
        mipmap_count: Option<usize>,
    ) -> Self {
        let mut face = MipmapMetadata {
            width: width.get(),
            height: height.get(),
            depth: if let MipmapType::ThreeDimensional(depth) = bitmap_type {
                depth.get()
            }
            else {
                1
            },

            mipmap_index: 0,
            face_index: 0,
            block_offset: 0,

            // to be set below
            block_height: 0,
            block_width: 0,
            block_count: 0
        };

        face.fixup_real_dims(block_length);

        let mut result = Self {
            next: Some(face),
            block_length,
            bitmap_type,
            mipmap_count,
            face_count: 0
        };
        result.fixup_face_count();
        result
    }

    /// Initialize a new iterator from a `BitmapData` instance.
    ///
    /// Returns `Err` if the bitmap data is malformed.
    pub fn new_from_bitmap_data(data: &BitmapData) -> RinghopperResult<Self> {
        let width = NonZeroUsize::new(data.width as usize)
            .ok_or_else(|| Error::InvalidTagData("width is 0".to_owned()))?;
        let height = NonZeroUsize::new(data.height as usize)
            .ok_or_else(|| Error::InvalidTagData("height is 0".to_owned()))?;
        let bitmap_type = MipmapType::get_mipmap_type(data)?;
        let block_length = pixels_per_block_length(data.format);
        let mipmap_count = Some(data.mipmap_count as usize);
        Ok(Self::new(width, height, bitmap_type, block_length, mipmap_count))
    }

    fn fixup_face_count(&mut self) {
        self.face_count = if self.bitmap_type == MipmapType::Cubemap { 6 } else { self.next.unwrap().depth };
    }
}


impl Iterator for MipmapFaceIterator {
    type Item = MipmapMetadata;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next.as_mut()?;
        let current = Some(*next);

        // advance pixel offset
        next.block_offset += next.block_count;

        // end of face count? if not, just return current
        next.face_index = (next.face_index + 1) % self.face_count;
        if next.face_index != 0 {
            return current;
        }

        // last mipmap?
        next.mipmap_index += 1;
        if (next.width == 1 && next.height == 1 && next.depth == 1) || self.mipmap_count.is_some_and(|c| c < next.mipmap_index) {
            self.next = None;
            return current;
        }

        // halve dimensions
        next.width = (next.width / 2).max(1);
        next.height = (next.height / 2).max(1);
        next.depth = (next.depth / 2).max(1);
        next.fixup_real_dims(self.block_length);
        self.fixup_face_count();

        current
    }
}

impl Iterator for MipmapTextureIterator {
    type Item = MipmapMetadata;
    fn next(&mut self) -> Option<Self::Item> {
        let mut current = self.inner.next()?;

        match self.inner.bitmap_type {
            MipmapType::TwoDimensional => return Some(current),
            MipmapType::Cubemap => current.block_count *= 6,
            MipmapType::ThreeDimensional(_) => current.block_count *= current.depth,
        }

        while self.inner.next.is_some_and(|i| i.mipmap_index == current.mipmap_index) {
            self.inner.next();
        }

        Some(current)
    }
}

impl FusedIterator for MipmapFaceIterator {}
impl FusedIterator for MipmapTextureIterator {}

pub const COMPRESSED_BITMAP_DATA_FORMATS: &'static [BitmapDataFormat] = &[
    BitmapDataFormat::DXT1,
    BitmapDataFormat::DXT3,
    BitmapDataFormat::DXT5,
    BitmapDataFormat::BC7
];

/// Extract the color plate of the bitmap tag into an [`Image`] instace.
///
/// Returns `None` if the bitmap tag does not have a valid color plate.
pub fn extract_compressed_color_plate_data(bitmap: &Bitmap) -> RinghopperResult<Option<Image>> {
    let color_plate = &bitmap.color_plate;
    let bytes = color_plate.compressed_data.bytes.as_slice();
    let width = color_plate.width as usize;
    let height = color_plate.height as usize;

    if bytes.len() < u32::simple_size() || width == 0 || height == 0 {
        return Ok(None)
    }

    let (len_bytes, compressed_bytes) = bytes.split_at(u32::simple_size());

    let uncompressed_size = u32::read::<BigEndian>(len_bytes, 0, len_bytes.len()).unwrap() as usize;
    let pixel_count = height.mul_overflow_checked(width)?;
    let expected_uncompressed_size = pixel_count.mul_overflow_checked(Pixel32::simple_size())?;
    if uncompressed_size != expected_uncompressed_size {
        return Err(Error::InvalidTagData(format!("compressed color plate data size is wrong (expected 0x{expected_uncompressed_size:08X}, got 0x{uncompressed_size:08X} instead)")))
    }

    let mut compressed_data = vec![0u8; expected_uncompressed_size];
    let mut decompressor = flate2::Decompress::new(true);

    decompressor
        .decompress(&compressed_bytes, &mut compressed_data, FlushDecompress::Finish)
        .map_err(|e| Error::InvalidTagData(format!("zlib error: {e}")))?;

    let read_bytes = decompressor.total_out();
    if read_bytes as usize != compressed_data.len() {
        return Err(Error::InvalidTagData(format!("decompressed size is wrong (expected 0x{expected_uncompressed_size:08X}, got 0x{read_bytes:08X} instead)")))
    }

    let result = Pixel32::read_chunks_to_iterator::<LittleEndian>(&compressed_data)
        .unwrap()
        .into_infallible();

    let mut data = Vec::with_capacity(pixel_count);
    data.extend(result);

    Ok(Some(Image { width, height, data }))
}
