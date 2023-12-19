use std::fmt::{Display, Debug};

use byteorder::ByteOrder;
use crate::error::*;

use super::*;

/// Max error tolerance for determinig if a vector is normalized.
const NONNORMAL_THRESHOLD: f32 = 0.00001;

/// General functionality for vector types.
pub trait Vector: Sized {
    /// Normalize the vector into a unit vector.
    fn normalize(&self) -> Self {
        let value = self.normalize_into(1.0);
        debug_assert!(self.is_unit_vector());
        value
    }

    /// Normalize the vector into the given unit.
    fn normalize_into(&self, unit: f32) -> Self {
        let magnitude_sq = self.magnitude_squared();
        if magnitude_sq == 0.0 {
            return Self::one();
        }

        let result = self.scale(unit / magnitude_sq.sqrt());
        debug_assert!(result.is_unit_vector());
        result
    }

    /// Return `true` if the vector is a unit vector.
    fn is_unit_vector(&self) -> bool {
        (1.0 - self.magnitude_squared()).abs() < NONNORMAL_THRESHOLD
    }

    /// Calculate the magnitude for the vector squared.
    ///
    /// This is the same as getting the distance from [`Vector::zero()`].
    fn magnitude_squared(&self) -> f32 {
        self.distance_squared(&Self::zero())
    }

    /// Return the dot product of this and another vector.
    fn dot(&self, with: &Self) -> f32;

    /// Return a vector where all components are equal to zero.
    fn zero() -> Self;

    /// Return a unit vector.
    fn one() -> Self;

    /// Scale the vector by `scaler`.
    fn scale(&self, scaler: f32) -> Self;

    /// Compute the sum of this vector and another vector.
    fn add(&self, of: &Self) -> Self;

    /// Subtract the components of this vector with the components of another.
    fn subtract(&self, with: &Self) -> Self;

    /// Multiply the components of this vector and another vector.
    fn multiply(&self, by: &Self) -> Self;

    /// Divide the components this vector by the components of the other vector.
    fn divide(&self, by: &Self) -> Self;

    /// Compute the distance the point lies from a plane.
    ///
    /// If less than 0, it's behind the plane. If greater than 0, it's in front of the plane. Otherwise, it's
    /// intersected, although floating point precision may result in an intersected point being very close.
    ///
    /// Note that, unlike [`distance_squared`](Vector::distance_squared), this value is not squared.
    ///
    /// # Examples (with [`Vector2D`])
    /// ```rust
    /// use ringhopper_primitives::primitive::{Vector, Vector2D, Plane2D};
    ///
    /// let point = Vector2D::zero();
    /// let plane = Plane2D { vector: Vector2D { x: 1.0, y: 0.0 }, d: 2.0 };
    /// let distance = point.distance_from_plane(&plane);
    /// assert!(distance == -2.0);
    /// ```
    fn distance_from_plane<P: Plane<VectorType = Self>>(&self, plane: &P) -> f32 {
        self.dot(&plane.vector()) - plane.d()
    }

    /// Compute the distance squared between the two vectors without computing a square root.
    ///
    /// This returns the distance squared. To get the real distance, use [`f32::sqrt`].
    ///
    /// # Examples (with [`Vector2D`])
    /// ```rust
    /// use ringhopper_primitives::primitive::{Vector, Vector2D};
    ///
    /// let a = Vector2D { x: 2.5, y: 0.0 };
    /// let b = Vector2D::zero();
    /// let distance = a.distance_squared(&b);
    /// assert!(distance > 2.0 * 2.0);
    /// assert!(distance < 3.0 * 3.0);
    /// ```
    fn distance_squared(&self, of: &Self) -> f32;
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Vector2D {
    pub x: f32,
    pub y: f32
}

impl Vector for Vector2D {
    fn scale(&self, scaler: f32) -> Self {
        Self {
            x: self.x * scaler,
            y: self.y * scaler
        }
    }

    fn add(&self, of: &Self) -> Self {
        Self {
            x: self.x + of.x,
            y: self.y + of.y,
        }
    }

    fn subtract(&self, with: &Self) -> Self {
        Self {
            x: self.x - with.x,
            y: self.y - with.y,
        }
    }

    fn multiply(&self, of: &Self) -> Self {
        Self {
            x: self.x * of.x,
            y: self.y * of.y,
        }
    }

    fn divide(&self, of: &Self) -> Self {
        Self {
            x: self.x / of.x,
            y: self.y / of.y,
        }
    }

    fn distance_squared(&self, of: &Self) -> f32 {
        let x = self.x - of.x;
        let y = self.y - of.y;
        x*x + y*y
    }

    fn zero() -> Self {
        Self::default()
    }

    fn one() -> Self {
        let o = Self {
            x: 0.0,
            y: 1.0,
        };

        debug_assert!(o.is_unit_vector());

        o
    }

    fn dot(&self, with: &Self) -> f32 {
        self.x * with.x + self.y * with.y
    }
}

generate_tag_data_simple_primitive_code!(Vector2D, f32, x, y);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Vector3D {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl Vector for Vector3D {
    fn scale(&self, scaler: f32) -> Self {
        Self {
            x: self.x * scaler,
            y: self.y * scaler,
            z: self.z * scaler,
        }
    }

    fn add(&self, of: &Self) -> Self {
        Self {
            x: self.x + of.x,
            y: self.y + of.y,
            z: self.z + of.z,
        }
    }

    fn subtract(&self, with: &Self) -> Self {
        Self {
            x: self.x - with.x,
            y: self.y - with.y,
            z: self.z - with.z,
        }
    }

    fn multiply(&self, of: &Self) -> Self {
        Self {
            x: self.x * of.x,
            y: self.y * of.y,
            z: self.z * of.z,
        }
    }

    fn divide(&self, of: &Self) -> Self {
        Self {
            x: self.x / of.x,
            y: self.y / of.y,
            z: self.z / of.z,
        }
    }

    fn distance_squared(&self, of: &Self) -> f32 {
        let x = self.x - of.x;
        let y = self.y - of.y;
        let z = self.z - of.z;
        x*x + y*y + z*z
    }

    fn zero() -> Self {
        Self::default()
    }

    fn one() -> Self {
        let o = Self {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };

        debug_assert!(o.is_unit_vector());

        o
    }

    fn dot(&self, with: &Self) -> f32 {
        self.x * with.x + self.y * with.y + self.z * with.z
    }
}

generate_tag_data_simple_primitive_code!(Vector3D, f32, x, y, z);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

generate_tag_data_simple_primitive_code!(Quaternion, f32, x, y, z, w);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Euler2D {
    pub yaw: Angle,
    pub pitch: Angle
}

generate_tag_data_simple_primitive_code!(Euler2D, Angle, yaw, pitch);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Euler3D {
    pub yaw: Angle,
    pub pitch: Angle,
    pub roll: Angle
}

generate_tag_data_simple_primitive_code!(Euler3D, Angle, yaw, pitch, roll);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Matrix3x3 {
    pub vectors: [Vector3D; 3]
}

impl TagDataSimplePrimitive for Matrix3x3 {
    fn size() -> usize {
        <Vector3D as TagDataSimplePrimitive>::size() * 3
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        let at1 = at;
        let at2 = at1.add_overflow_checked(<Vector3D as TagDataSimplePrimitive>::size())?;
        let at3 = at2.add_overflow_checked(<Vector3D as TagDataSimplePrimitive>::size())?;

        Ok(Matrix3x3 {
            vectors: [
                Vector3D::read::<B>(data, at1, struct_end)?,
                Vector3D::read::<B>(data, at2, struct_end)?,
                Vector3D::read::<B>(data, at3, struct_end)?,
            ]
        })
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        let at1 = at;
        let at2 = at1.add_overflow_checked(<Vector3D as TagDataSimplePrimitive>::size())?;
        let at3 = at2.add_overflow_checked(<Vector3D as TagDataSimplePrimitive>::size())?;

        self.vectors[0].write::<B>(data, at1, struct_end)?;
        self.vectors[1].write::<B>(data, at2, struct_end)?;
        self.vectors[2].write::<B>(data, at3, struct_end)?;

        Ok(())
    }
}

/// Represents an angle.
///
/// The value is internally represented as radians.
#[derive(Clone, Copy, Default, PartialEq)]
#[repr(transparent)]
pub struct Angle {
    /// Angle value, represented in radians.
    pub angle: f32
}

impl Angle {
    /// Create an Angle from degrees.
    pub fn from_degrees(deg: f32) -> Self {
        Angle { angle: deg.to_radians() }
    }

    /// Convert an Angle to degrees.
    pub fn to_degrees(self) -> f32 {
        self.angle.to_degrees()
    }

    /// Create an Angle from radians.
    pub fn from_radians(rad: f32) -> Self {
        rad.into()
    }

    /// Convert an Angle to radians.
    pub fn to_radians(self) -> f32 {
        self.angle
    }
}

impl Display for Angle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.05}°", self.to_degrees())
    }
}

impl Debug for Angle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl From<f32> for Angle {
    fn from(value: f32) -> Self {
        Angle { angle: value }
    }
}

impl From<Angle> for f32 {
    fn from(value: Angle) -> Self {
        value.angle
    }
}

generate_tag_data_simple_primitive_code!(Angle, f32, angle);
