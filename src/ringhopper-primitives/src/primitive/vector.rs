use std::fmt::{Display, Debug, Formatter};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use byteorder::ByteOrder;
use crate::dynamic::SimplePrimitiveType;
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

    /// Scale the vector by `by`.
    fn scale(&self, by: f32) -> Self;

    /// Compute the sum of this vector and another vector.
    fn add(&self, of: &Self) -> Self;

    /// Subtract the components of this vector with the components of another.
    fn sub(&self, with: &Self) -> Self;

    /// Multiply the components of this vector and another vector.
    fn mul(&self, by: &Self) -> Self;

    /// Divide the components this vector by the components of the other vector.
    fn div(&self, by: &Self) -> Self;

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
    /// assert_eq!(-2.0, distance);
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
pub struct Vector2DInt {
    pub x: i16,
    pub y: i16
}

generate_tag_data_simple_primitive_code!(Vector2DInt, i16, x, y);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Rectangle {
    pub top: i16,
    pub left: i16,
    pub bottom: i16,
    pub right: i16
}

generate_tag_data_simple_primitive_code!(Rectangle, i16, top, left, bottom, right);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct Vector2D {
    pub x: f32,
    pub y: f32
}

impl Vector for Vector2D {
    fn dot(&self, with: &Self) -> f32 {
        self.x * with.x + self.y * with.y
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

    fn scale(&self, by: f32) -> Self {
        Self {
            x: self.x * by,
            y: self.y * by
        }
    }

    fn add(&self, of: &Self) -> Self {
        Self {
            x: self.x + of.x,
            y: self.y + of.y,
        }
    }

    fn sub(&self, with: &Self) -> Self {
        Self {
            x: self.x - with.x,
            y: self.y - with.y,
        }
    }

    fn mul(&self, of: &Self) -> Self {
        Self {
            x: self.x * of.x,
            y: self.y * of.y,
        }
    }

    fn div(&self, of: &Self) -> Self {
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
    fn dot(&self, with: &Self) -> f32 {
        self.x * with.x + self.y * with.y + self.z * with.z
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

    fn scale(&self, by: f32) -> Self {
        Self {
            x: self.x * by,
            y: self.y * by,
            z: self.z * by,
        }
    }

    fn add(&self, of: &Self) -> Self {
        Self {
            x: self.x + of.x,
            y: self.y + of.y,
            z: self.z + of.z,
        }
    }

    fn sub(&self, with: &Self) -> Self {
        Self {
            x: self.x - with.x,
            y: self.y - with.y,
            z: self.z - with.z,
        }
    }

    fn mul(&self, of: &Self) -> Self {
        Self {
            x: self.x * of.x,
            y: self.y * of.y,
            z: self.z * of.z,
        }
    }

    fn div(&self, of: &Self) -> Self {
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

impl Display for Matrix3x3 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{{ vectors[0] = {}; vectors[1] = {}; vectors[2] = {} }}", self.vectors[0], self.vectors[1], self.vectors[2]))
    }
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

    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::Matrix3x3
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}°]", self.angle, self.to_degrees())
    }
}

impl Debug for Angle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

impl Add for Angle {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Angle::from_radians(self.angle + rhs.angle)
    }
}
impl AddAssign for Angle {
    fn add_assign(&mut self, rhs: Self) {
        *self = Angle::from_radians(self.angle + rhs.angle)
    }
}
impl Sub for Angle {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Angle::from_radians(self.angle - rhs.angle)
    }
}
impl SubAssign for Angle {
    fn sub_assign(&mut self, rhs: Self) {
        *self = Angle::from_radians(self.angle - rhs.angle)
    }
}
impl Mul for Angle {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Angle::from_radians(self.angle * rhs.angle)
    }
}
impl MulAssign for Angle {
    fn mul_assign(&mut self, rhs: Self) {
        *self = Angle::from_radians(self.angle * rhs.angle)
    }
}
impl Div for Angle {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Angle::from_radians(self.angle / rhs.angle)
    }
}
impl DivAssign for Angle {
    fn div_assign(&mut self, rhs: Self) {
        *self = Angle::from_radians(self.angle / rhs.angle)
    }
}
impl Mul<f32> for Angle {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Angle::from_radians(self.angle * rhs)
    }
}
impl MulAssign<f32> for Angle {
    fn mul_assign(&mut self, rhs: f32) {
        *self = Angle::from_radians(self.angle * rhs)
    }
}
impl Neg for Angle {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Angle::from_radians(-self.angle)
    }
}

impl TagDataSimplePrimitive for Angle {
    fn size() -> usize {
        <f32 as TagDataSimplePrimitive>::size()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(Self {
            angle: f32::read::<B>(data, at, struct_end)?
        })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.angle.write::<B>(data, at, struct_end)
    }

    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::Angle
    }
}

macro_rules! define_ops_for_vector {
    ($vector:ty) => {
        impl Add for $vector {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                Vector::add(&self, &rhs)
            }
        }
        impl AddAssign for $vector {
            fn add_assign(&mut self, rhs: Self) {
                *self = Vector::add(&self, &rhs)
            }
        }
        impl Sub for $vector {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Vector::sub(&self, &rhs)
            }
        }
        impl SubAssign for $vector {
            fn sub_assign(&mut self, rhs: Self) {
                *self = Vector::sub(&self, &rhs)
            }
        }
        impl Mul for $vector {
            type Output = Self;
            fn mul(self, rhs: Self) -> Self {
                Vector::mul(&self, &rhs)
            }
        }
        impl MulAssign for $vector {
            fn mul_assign(&mut self, rhs: Self) {
                *self = Vector::mul(&self, &rhs)
            }
        }
        impl Div for $vector {
            type Output = Self;
            fn div(self, rhs: Self) -> Self {
                Vector::div(&self, &rhs)
            }
        }
        impl DivAssign for $vector {
            fn div_assign(&mut self, rhs: Self) {
                *self = Vector::div(&self, &rhs)
            }
        }
        impl Mul<f32> for $vector {
            type Output = Self;
            fn mul(self, rhs: f32) -> Self::Output {
                Vector::scale(&self, rhs)
            }
        }
        impl MulAssign<f32> for $vector {
            fn mul_assign(&mut self, rhs: f32) {
                *self = Vector::scale(&self, rhs)
            }
        }
        impl Neg for $vector {
            type Output = Self;
            fn neg(self) -> Self::Output {
                Vector::scale(&self, -1.0)
            }
        }
    };
}

define_ops_for_vector!(Vector2D);
define_ops_for_vector!(Vector3D);
