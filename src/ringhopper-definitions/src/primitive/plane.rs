use super::*;

/// General functionality for planar types.
pub trait Plane {
    type VectorType: Vector;

    fn vector(&self) -> Self::VectorType;
    fn d(&self) -> f32;
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Plane2D {
    pub vector: Vector2D,
    pub d: f32
}

impl Plane for Plane2D {
    type VectorType = Vector2D;
    fn vector(&self) -> Vector2D {
        self.vector
    }
    fn d(&self) -> f32 {
        self.d
    }
}

impl TagDataSimplePrimitive for Plane2D {
    fn size() -> usize {
        std::mem::size_of::<Self>()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        let vector = Vector2D::read::<B>(data, at, struct_end)?;
        let d = f32::read::<B>(data, at + 0x8, struct_end)?;
        Ok(Self {
            vector, d
        })
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.vector.write::<B>(data, at, struct_end)?;
        self.d.write::<B>(data, at + 0x8, struct_end)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Plane3D {
    pub vector: Vector3D,
    pub d: f32
}

impl Plane for Plane3D {
    type VectorType = Vector3D;
    fn vector(&self) -> Vector3D {
        self.vector
    }
    fn d(&self) -> f32 {
        self.d
    }
}

impl TagDataSimplePrimitive for Plane3D {
    fn size() -> usize {
        std::mem::size_of::<Self>()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        let vector = Vector3D::read::<B>(data, at, struct_end)?;
        let d = f32::read::<B>(data, at + 0xC, struct_end)?;
        Ok(Self {
            vector, d
        })
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.vector.write::<B>(data, at, struct_end)?;
        self.d.write::<B>(data, at + 0xC, struct_end)?;
        Ok(())
    }
}
