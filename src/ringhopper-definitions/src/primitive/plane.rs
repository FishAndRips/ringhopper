use super::*;

/// General functionality for planar types.
pub trait Plane {
    type VectorType: Vector;
    
    fn vector(&self) -> Self::VectorType;
    fn d(&self) -> f32;
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
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

#[derive(Clone, Copy, Default, Debug, PartialEq)]
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
