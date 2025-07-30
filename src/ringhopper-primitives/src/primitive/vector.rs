use std::fmt::{Display, Debug, Formatter};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use byteorder::ByteOrder;
use crate::dynamic::SimplePrimitiveType;
use crate::parse::SimplePrimitive;
use crate::error::*;

use super::*;

/// Max error tolerance for determining if a vector is normalized.
const NONNORMAL_THRESHOLD: f32 = 0.00001;

trait VectorMathOps {
    /// Compute the sum of this vector and another vector.
    fn add(&self, of: &Self) -> Self;

    /// Subtract the components of this vector with the components of another.
    fn sub(&self, with: &Self) -> Self;

    /// Multiply the components of this vector and another vector.
    fn mul(&self, by: &Self) -> Self;

    /// Divide the components this vector by the components of the other vector.
    fn div(&self, by: &Self) -> Self;
}

/// General functionality for vector types.
pub trait Vector: Sized + Copy + Clone {
    /// Normalize the vector into a unit vector.
    fn normalize(&self) -> Self {
        if self.is_unit_vector() {
            return *self
        }
        let value = self.normalize_into(1.0);
        debug_assert!(value.is_unit_vector());
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

    fn distance_squared(&self, of: &Self) -> f32 {
        let delta = *self - *of;
        delta.dot(&delta)
    }
}

impl VectorMathOps for Vector2D {
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

    fn distance_squared(&self, of: &Self) -> f32 {
        let delta = *self - *of;
        delta.dot(&delta)
    }
}

impl VectorMathOps for Vector3D {
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

impl Vector for Quaternion {
    fn dot(&self, with: &Self) -> f32 {
        self.x * with.x + self.y * with.y + self.z * with.z + self.w * with.w
    }

    fn zero() -> Self {
        Self::default()
    }

    fn one() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }

    fn scale(&self, by: f32) -> Self {
        Self {
            x: self.x * by,
            y: self.y * by,
            z: self.z * by,
            w: self.w * by,
        }
    }

    fn distance_squared(&self, of: &Self) -> f32 {
        let delta = *self - *of;
        delta.dot(&delta)
    }
}

impl VectorMathOps for Quaternion {
    fn add(&self, of: &Self) -> Self {
        Self {
            x: self.x + of.x,
            y: self.y + of.y,
            z: self.z + of.z,
            w: self.w + of.w,
        }
    }

    fn sub(&self, with: &Self) -> Self {
        Self {
            x: self.x - with.x,
            y: self.y - with.y,
            z: self.z - with.z,
            w: self.w - with.w,
        }
    }

    fn mul(&self, by: &Self) -> Self {
        Self {
            x: self.x * by.x,
            y: self.y * by.y,
            z: self.z * by.z,
            w: self.w * by.w,
        }
    }

    fn div(&self, by: &Self) -> Self {
        Self {
            x: self.x / by.x,
            y: self.y / by.y,
            z: self.z / by.z,
            w: self.w / by.w,
        }
    }
}

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

impl SimpleTagData for Matrix3x3 {
    fn simple_size() -> usize {
        Vector3D::simple_size() * 3
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        let at1 = at;
        let at2 = at1.add_overflow_checked(Vector3D::simple_size())?;
        let at3 = at2.add_overflow_checked(Vector3D::simple_size())?;

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
        let at2 = at1.add_overflow_checked(Vector3D::simple_size())?;
        let at3 = at2.add_overflow_checked(Vector3D::simple_size())?;

        self.vectors[0].write::<B>(data, at1, struct_end)?;
        self.vectors[1].write::<B>(data, at2, struct_end)?;
        self.vectors[2].write::<B>(data, at3, struct_end)?;

        Ok(())
    }
}

impl SimplePrimitive for Matrix3x3 {
    fn primitive_type() -> SimplePrimitiveType {
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
    pub const fn from_radians(rad: f32) -> Self {
        Self { angle: rad }
    }

    /// Convert an Angle to radians.
    pub const fn to_radians(self) -> f32 {
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

impl SimpleTagData for Angle {
    fn simple_size() -> usize {
        f32::simple_size()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(Self {
            angle: f32::read::<B>(data, at, struct_end)?
        })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.angle.to_bits().write::<B>(data, at, struct_end)
    }
}

impl SimplePrimitive for Angle {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::Angle
    }
}

macro_rules! define_ops_for_vector {
    ($vector:ty) => {
        impl Add for $vector {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                VectorMathOps::add(&self, &rhs)
            }
        }
        impl AddAssign for $vector {
            fn add_assign(&mut self, rhs: Self) {
                *self = VectorMathOps::add(&self, &rhs)
            }
        }
        impl Sub for $vector {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                VectorMathOps::sub(&self, &rhs)
            }
        }
        impl SubAssign for $vector {
            fn sub_assign(&mut self, rhs: Self) {
                *self = VectorMathOps::sub(&self, &rhs)
            }
        }
        impl Mul for $vector {
            type Output = Self;
            fn mul(self, rhs: Self) -> Self {
                VectorMathOps::mul(&self, &rhs)
            }
        }
        impl MulAssign for $vector {
            fn mul_assign(&mut self, rhs: Self) {
                *self = VectorMathOps::mul(&self, &rhs)
            }
        }
        impl Div for $vector {
            type Output = Self;
            fn div(self, rhs: Self) -> Self {
                VectorMathOps::div(&self, &rhs)
            }
        }
        impl DivAssign for $vector {
            fn div_assign(&mut self, rhs: Self) {
                *self = VectorMathOps::div(&self, &rhs)
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
define_ops_for_vector!(Quaternion);

/// Denotes a float compressed into a 16-bit integer.
#[derive(Clone, Copy, Default, PartialEq)]
#[repr(transparent)]
pub struct CompressedFloat {
    pub data: u16
}

impl From<f32> for CompressedFloat {
    fn from(value: f32) -> Self {
        Self { data: compress_float::<16>(value) as u16 }
    }
}

impl From<CompressedFloat> for f32 {
    fn from(value: CompressedFloat) -> Self {
        decompress_float::<16>(value.data as u32)
    }
}

impl SimpleTagData for CompressedFloat {
    fn simple_size() -> usize {
        u16::simple_size()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        u16::read::<B>(data, at, struct_end).map(|data| Self { data })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.data.write::<B>(data, at, struct_end)
    }
}

impl SimplePrimitive for CompressedFloat {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::CompressedFloat
    }
}

impl Debug for CompressedFloat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("compressed<0x{:04X} = {:?}>", self.data, f32::from(*self)))
    }
}

impl Display for CompressedFloat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("compressed<0x{:04X} = {}>", self.data, f32::from(*self)))
    }
}

/// Denotes a 3D vector compressed into a 32-bit integer.
#[derive(Clone, Copy, Default, PartialEq)]
#[repr(transparent)]
pub struct CompressedVector3D {
    pub data: u32
}

impl From<Vector3D> for CompressedVector3D {
    fn from(value: Vector3D) -> Self {
        Self { data: compress_vector3d(&value) }
    }
}

impl From<CompressedVector3D> for Vector3D {
    fn from(value: CompressedVector3D) -> Self {
        decompress_vector3d(value.data)
    }
}

impl Debug for CompressedVector3D {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("compressed<0x{:08X} = {:?}>", self.data, Vector3D::from(*self)))
    }
}

impl Display for CompressedVector3D {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("compressed<0x{:08X} = {}>", self.data, Vector3D::from(*self)))
    }
}

impl SimpleTagData for CompressedVector3D {
    fn simple_size() -> usize {
        u32::simple_size()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        u32::read::<B>(data, at, struct_end).map(|data| Self { data })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.data.write::<B>(data, at, struct_end)
    }
}

impl SimplePrimitive for CompressedVector3D {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::CompressedVector3D
    }
}

/// Denotes a 2D vector compressed into a 32-bit integer.
#[derive(Clone, Copy, Default, PartialEq)]
#[repr(transparent)]
pub struct CompressedVector2D {
    pub data: u32
}

impl From<Vector2D> for CompressedVector2D {
    fn from(value: Vector2D) -> Self {
        Self { data: compress_vector2d(&value) }
    }
}

impl From<CompressedVector2D> for Vector2D {
    fn from(value: CompressedVector2D) -> Self {
        decompress_vector2d(value.data)
    }
}

impl Debug for CompressedVector2D {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("compressed<0x{:08X} = {:?}>", self.data, Vector2D::from(*self)))
    }
}

impl Display for CompressedVector2D {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("compressed<0x{:08X} = {}>", self.data, Vector2D::from(*self)))
    }
}

impl SimpleTagData for CompressedVector2D {
    fn simple_size() -> usize {
        u32::simple_size()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        u32::read::<B>(data, at, struct_end).map(|data| Self { data })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.data.write::<B>(data, at, struct_end)
    }
}

impl SimplePrimitive for CompressedVector2D {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::CompressedVector2D
    }
}

fn compress_vector3d(vector: &Vector3D) -> u32 {
    let x = compress_float::<11>(vector.x);
    let y = compress_float::<11>(vector.y) << 11;
    let z = compress_float::<10>(vector.z) << 22;

    x | y | z
}

fn decompress_vector3d(vector: u32) -> Vector3D {
    let x = decompress_float::<11>(vector);
    let y = decompress_float::<11>(vector >> 11);
    let z = decompress_float::<10>(vector >> 22);

    Vector3D { x, y, z }
}

fn compress_vector2d(vector: &Vector2D) -> u32 {
    let x = compress_float::<16>(vector.x);
    let y = compress_float::<16>(vector.y) << 16;

    x | y
}

fn decompress_vector2d(vector: u32) -> Vector2D {
    let x = decompress_float::<16>(vector);
    let y = decompress_float::<16>(vector >> 16);

    Vector2D { x, y }
}

fn decompress_float<const BITS: usize>(value: u32) -> f32 {
    let is_negative = (value & (1 << (BITS - 1))) != 0;
    let mut max = ((1 << (BITS - 1)) - 1) as u32;
    let mut value = value & max;

    if is_negative {
        max += 1;
        value = max - value;
    }

    let mut value = value as f64;
    value /= max as f64;

    let float = value as f32;
    if is_negative {
        -float
    }
    else {
        float
    }
}

fn compress_float<const BITS: usize>(value: f32) -> u32 {
    let value = value.clamp(-1.0, 1.0);
    let mut positive = value.abs();
    let is_negative = positive != value;
    let mut max = ((1 << (BITS - 1)) - 1) as u32;

    if is_negative {
        positive = 1.0 - positive;
        max += 1;
    }

    let value = (positive * (max as f32)).round();
    let mut integer = value as u32;
    if is_negative {
        integer = integer | 1 << (BITS - 1);
    }

    integer
}

#[cfg(test)]
mod test;
