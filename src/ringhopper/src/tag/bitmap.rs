#[cfg(test)]
mod test;

use std::iter::FusedIterator;
use std::num::NonZeroUsize;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MipmapMetadata {
    /// Current mipmap index.
    ///
    /// 0 = base map, 1 onwards = mipmap number
    pub mipmap_index: usize,

    /// Current face index.
    ///
    /// For cubemaps, this is 0-5, for 3D textures this is 0-<depth>, and for 2D textures, this is always 0.
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
    fn fixup_real_dims(&mut self, block_size: NonZeroUsize) {
        let block_size = block_size.get();
        self.block_width = (self.width + (block_size - 1)) / block_size;
        self.block_height = (self.height + (block_size - 1)) / block_size;
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
        block_size: NonZeroUsize,
        mipmap_count: Option<NonZeroUsize>
    ) -> Self {
        Self { inner: MipmapFaceIterator::new(width, height, bitmap_type, block_size, mipmap_count) }
    }
}

/// Iterates bitmap by faces.
///
/// Unlike [`MipmapTextureIterator`], this will iterate through each individual face of a mipmap.
///
/// For example, a cubemap will yield one face per iteration (this 6 iterations to go through the full mipmap), and a
/// 3D texture will contain only one level of depth per iteration.
#[derive(Copy, Clone)]
struct MipmapFaceIterator {
    next: Option<MipmapMetadata>,
    block_size: NonZeroUsize,
    bitmap_type: MipmapType,
    mipmap_count: Option<NonZeroUsize>,
    face_count: usize
}

impl MipmapFaceIterator {
    pub fn new(
        width: NonZeroUsize,
        height: NonZeroUsize,
        bitmap_type: MipmapType,
        block_size: NonZeroUsize,
        mipmap_count: Option<NonZeroUsize>,
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

        face.fixup_real_dims(block_size);

        let mut result = Self {
            next: Some(face),
            block_size,
            bitmap_type,
            mipmap_count,
            face_count: 0
        };
        result.fixup_face_count();
        result
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
        if (next.width == 1 && next.height == 1 && next.depth == 1) || self.mipmap_count.is_some_and(|c| c.get() == next.mipmap_index) {
            self.next = None;
            return current;
        }

        // halve dimensions
        next.width = (next.width / 2).max(1);
        next.height = (next.height / 2).max(1);
        next.depth = (next.depth / 2).max(1);
        next.fixup_real_dims(self.block_size);
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
