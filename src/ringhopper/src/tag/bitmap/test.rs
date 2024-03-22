use std::num::NonZeroUsize;
use super::*;

#[test]
fn iterates_depth_by_faces() {
    let mut d = MipmapFaceIterator::new(
        NonZeroUsize::new(8).unwrap(),
        NonZeroUsize::new(8).unwrap(),
        MipmapType::ThreeDimensional(NonZeroUsize::new(4).unwrap()),
        NonZeroUsize::new(1).unwrap(),
        None
    );

    let order = [
        MipmapMetadata { mipmap_index: 0, face_index: 0, block_offset: 0, width: 8, height: 8, depth: 4, block_width: 8, block_height: 8, block_count: 64 },
        MipmapMetadata { mipmap_index: 0, face_index: 1, block_offset: 64, width: 8, height: 8, depth: 4, block_width: 8, block_height: 8, block_count: 64 },
        MipmapMetadata { mipmap_index: 0, face_index: 2, block_offset: 128, width: 8, height: 8, depth: 4, block_width: 8, block_height: 8, block_count: 64 },
        MipmapMetadata { mipmap_index: 0, face_index: 3, block_offset: 192, width: 8, height: 8, depth: 4, block_width: 8, block_height: 8, block_count: 64 },
        MipmapMetadata { mipmap_index: 1, face_index: 0, block_offset: 256, width: 4, height: 4, depth: 2, block_width: 4, block_height: 4, block_count: 16 },
        MipmapMetadata { mipmap_index: 1, face_index: 1, block_offset: 272, width: 4, height: 4, depth: 2, block_width: 4, block_height: 4, block_count: 16 },
        MipmapMetadata { mipmap_index: 2, face_index: 0, block_offset: 288, width: 2, height: 2, depth: 1, block_width: 2, block_height: 2, block_count: 4 },
        MipmapMetadata { mipmap_index: 3, face_index: 0, block_offset: 292, width: 1, height: 1, depth: 1, block_width: 1, block_height: 1, block_count: 1 },
    ];

    for i in order {
        let next = d.next().unwrap();
        assert_eq!(i, next);
    }

    assert!(d.next().is_none());
}

#[test]
fn iterates_depth_by_textures() {
    let mut d = MipmapTextureIterator::new(
        NonZeroUsize::new(8).unwrap(),
        NonZeroUsize::new(8).unwrap(),
        MipmapType::ThreeDimensional(NonZeroUsize::new(4).unwrap()),
        NonZeroUsize::new(1).unwrap(),
        None
    );

    let order = [
        MipmapMetadata { mipmap_index: 0, face_index: 0, block_offset: 0, width: 8, height: 8, depth: 4, block_width: 8, block_height: 8, block_count: 256 },
        MipmapMetadata { mipmap_index: 1, face_index: 0, block_offset: 256, width: 4, height: 4, depth: 2, block_width: 4, block_height: 4, block_count: 32 },
        MipmapMetadata { mipmap_index: 2, face_index: 0, block_offset: 288, width: 2, height: 2, depth: 1, block_width: 2, block_height: 2, block_count: 4 },
        MipmapMetadata { mipmap_index: 3, face_index: 0, block_offset: 292, width: 1, height: 1, depth: 1, block_width: 1, block_height: 1, block_count: 1 },
    ];

    for i in order {
        let next = d.next().unwrap();
        assert_eq!(i, next);
    }

    assert!(d.next().is_none());
}

#[test]
fn iterates_cubemaps_by_faces() {
    let mut d = MipmapFaceIterator::new(
        NonZeroUsize::new(8).unwrap(),
        NonZeroUsize::new(8).unwrap(),
        MipmapType::Cubemap,
        NonZeroUsize::new(1).unwrap(),
        None
    );

    let order = [
        MipmapMetadata { mipmap_index: 0, face_index: 0, block_offset: 0, width: 8, height: 8, depth: 1, block_width: 8, block_height: 8, block_count: 64 },
        MipmapMetadata { mipmap_index: 0, face_index: 1, block_offset: 64, width: 8, height: 8, depth: 1, block_width: 8, block_height: 8, block_count: 64 },
        MipmapMetadata { mipmap_index: 0, face_index: 2, block_offset: 128, width: 8, height: 8, depth: 1, block_width: 8, block_height: 8, block_count: 64 },
        MipmapMetadata { mipmap_index: 0, face_index: 3, block_offset: 192, width: 8, height: 8, depth: 1, block_width: 8, block_height: 8, block_count: 64 },
        MipmapMetadata { mipmap_index: 0, face_index: 4, block_offset: 256, width: 8, height: 8, depth: 1, block_width: 8, block_height: 8, block_count: 64 },
        MipmapMetadata { mipmap_index: 0, face_index: 5, block_offset: 320, width: 8, height: 8, depth: 1, block_width: 8, block_height: 8, block_count: 64 },
        MipmapMetadata { mipmap_index: 1, face_index: 0, block_offset: 384, width: 4, height: 4, depth: 1, block_width: 4, block_height: 4, block_count: 16 },
        MipmapMetadata { mipmap_index: 1, face_index: 1, block_offset: 400, width: 4, height: 4, depth: 1, block_width: 4, block_height: 4, block_count: 16 },
        MipmapMetadata { mipmap_index: 1, face_index: 2, block_offset: 416, width: 4, height: 4, depth: 1, block_width: 4, block_height: 4, block_count: 16 },
        MipmapMetadata { mipmap_index: 1, face_index: 3, block_offset: 432, width: 4, height: 4, depth: 1, block_width: 4, block_height: 4, block_count: 16 },
        MipmapMetadata { mipmap_index: 1, face_index: 4, block_offset: 448, width: 4, height: 4, depth: 1, block_width: 4, block_height: 4, block_count: 16 },
        MipmapMetadata { mipmap_index: 1, face_index: 5, block_offset: 464, width: 4, height: 4, depth: 1, block_width: 4, block_height: 4, block_count: 16 },
        MipmapMetadata { mipmap_index: 2, face_index: 0, block_offset: 480, width: 2, height: 2, depth: 1, block_width: 2, block_height: 2, block_count: 4 },
        MipmapMetadata { mipmap_index: 2, face_index: 1, block_offset: 484, width: 2, height: 2, depth: 1, block_width: 2, block_height: 2, block_count: 4 },
        MipmapMetadata { mipmap_index: 2, face_index: 2, block_offset: 488, width: 2, height: 2, depth: 1, block_width: 2, block_height: 2, block_count: 4 },
        MipmapMetadata { mipmap_index: 2, face_index: 3, block_offset: 492, width: 2, height: 2, depth: 1, block_width: 2, block_height: 2, block_count: 4 },
        MipmapMetadata { mipmap_index: 2, face_index: 4, block_offset: 496, width: 2, height: 2, depth: 1, block_width: 2, block_height: 2, block_count: 4 },
        MipmapMetadata { mipmap_index: 2, face_index: 5, block_offset: 500, width: 2, height: 2, depth: 1, block_width: 2, block_height: 2, block_count: 4 },
        MipmapMetadata { mipmap_index: 3, face_index: 0, block_offset: 504, width: 1, height: 1, depth: 1, block_width: 1, block_height: 1, block_count: 1 },
        MipmapMetadata { mipmap_index: 3, face_index: 1, block_offset: 505, width: 1, height: 1, depth: 1, block_width: 1, block_height: 1, block_count: 1 },
        MipmapMetadata { mipmap_index: 3, face_index: 2, block_offset: 506, width: 1, height: 1, depth: 1, block_width: 1, block_height: 1, block_count: 1 },
        MipmapMetadata { mipmap_index: 3, face_index: 3, block_offset: 507, width: 1, height: 1, depth: 1, block_width: 1, block_height: 1, block_count: 1 },
        MipmapMetadata { mipmap_index: 3, face_index: 4, block_offset: 508, width: 1, height: 1, depth: 1, block_width: 1, block_height: 1, block_count: 1 },
        MipmapMetadata { mipmap_index: 3, face_index: 5, block_offset: 509, width: 1, height: 1, depth: 1, block_width: 1, block_height: 1, block_count: 1 },
    ];

    for i in order {
        let next = d.next().unwrap();
        assert_eq!(i, next);
    }

    assert!(d.next().is_none());
}

#[test]
fn iterates_cubemaps_by_texture() {
    let mut d = MipmapTextureIterator::new(
        NonZeroUsize::new(8).unwrap(),
        NonZeroUsize::new(8).unwrap(),
        MipmapType::Cubemap,
        NonZeroUsize::new(1).unwrap(),
        None
    );

    let order = [
        MipmapMetadata { mipmap_index: 0, face_index: 0, block_offset: 0, width: 8, height: 8, depth: 1, block_width: 8, block_height: 8, block_count: 384 },
        MipmapMetadata { mipmap_index: 1, face_index: 0, block_offset: 384, width: 4, height: 4, depth: 1, block_width: 4, block_height: 4, block_count: 96 },
        MipmapMetadata { mipmap_index: 2, face_index: 0, block_offset: 480, width: 2, height: 2, depth: 1, block_width: 2, block_height: 2, block_count: 24 },
        MipmapMetadata { mipmap_index: 3, face_index: 0, block_offset: 504, width: 1, height: 1, depth: 1, block_width: 1, block_height: 1, block_count: 6 },
    ];

    for i in order {
        let next = d.next().unwrap();
        assert_eq!(i, next);
    }

    assert!(d.next().is_none());
}

#[test]
fn iterates_block_compression() {
    let mut d = MipmapTextureIterator::new(
        NonZeroUsize::new(8).unwrap(),
        NonZeroUsize::new(8).unwrap(),
        MipmapType::TwoDimensional,
        NonZeroUsize::new(4).unwrap(),
        None
    );

    let order = [
        MipmapMetadata { mipmap_index: 0, face_index: 0, block_offset: 0, width: 8, height: 8, depth: 1, block_width: 2, block_height: 2, block_count: 4 },
        MipmapMetadata { mipmap_index: 1, face_index: 0, block_offset: 4, width: 4, height: 4, depth: 1, block_width: 1, block_height: 1, block_count: 1 },
        MipmapMetadata { mipmap_index: 2, face_index: 0, block_offset: 5, width: 2, height: 2, depth: 1, block_width: 1, block_height: 1, block_count: 1 },
        MipmapMetadata { mipmap_index: 3, face_index: 0, block_offset: 6, width: 1, height: 1, depth: 1, block_width: 1, block_height: 1, block_count: 1 }
    ];

    for i in order {
        assert_eq!(i, d.next().unwrap());
    }

    assert!(d.next().is_none());
}
