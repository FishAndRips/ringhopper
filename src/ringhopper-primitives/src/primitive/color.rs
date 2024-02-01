use std::fmt::{Debug, Display, Formatter};
use crate::primitive::Vector3D;
use crate::parse::*;
use crate::error::*;
use byteorder::*;
use crate::dynamic::SimplePrimitiveType;

/// General functionality for color types.
pub trait Color: Sized {
    /// Calculate the apparent brightness of the color.
    ///
    /// Alpha is not considered by this operation.
    ///
    /// # Examples (with [`ColorARGBFloat`])
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{Color, ColorARGBFloat};
    ///
    /// // luma = 0.25
    /// let color = ColorARGBFloat { alpha: 1.0, red: 0.25, green: 0.25, blue: 0.25 };
    ///
    /// // luma = ~0.28
    /// let brighter_color = ColorARGBFloat { alpha: 1.0, red: 0.0, green: 0.4, blue: 0.0 };
    ///
    /// // luma = ~0.085
    /// let darker_color = ColorARGBFloat { alpha: 1.0, red: 0.4, green: 0.0, blue: 0.0 };
    ///
    /// assert_eq!(color.luma(), 0.25);
    /// assert!(color.luma() < brighter_color.luma());
    /// assert!(color.luma() > darker_color.luma());
    /// ```
    fn luma(&self) -> f32 {
        let color = self.rgb_float().clamp();

        // Already monochrome
        if color.red == color.green && color.red == color.blue {
            return color.red
        }

        color.red * 0.2126 + color.green * 0.7152 + color.blue * 0.0722
    }

    /// Get the floating point RGB components of the color.
    ///
    /// # Examples (with [`ColorARGBFloat`])
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{Color, ColorARGBFloat, ColorRGBFloat};
    ///
    /// let color = ColorARGBFloat { alpha: 1.0, red: 0.125, green: 0.25, blue: 0.5 };
    /// assert_eq!(color.rgb_float(), ColorRGBFloat { red: 0.125, green: 0.25, blue: 0.5 });
    /// ```
    fn rgb_float(&self) -> ColorRGBFloat;

    /// Get the floating point alpha component of the color.
    ///
    /// # Examples (with [`ColorARGBFloat`])
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{Color, ColorARGBFloat};
    ///
    /// let color = ColorARGBFloat { alpha: 1.0, red: 0.25, green: 0.5, blue: 0.75 };
    /// assert_eq!(color.alpha_float(), 1.0);
    /// ```
    fn alpha_float(&self) -> f32;

    /// Convert from a [`ColorARGBFloat`] type.
    ///
    /// # Examples (with [`ColorARGBFloat`])
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{Color, ColorARGBFloat, ColorARGBIntBytes};
    ///
    /// let color = ColorARGBFloat { alpha: 1.0, red: 0.5, green: 0.25, blue: 0.125 };
    /// let expected = ColorARGBIntBytes { alpha: 255, red: 127, green: 63, blue: 31 };
    /// assert_eq!(ColorARGBIntBytes::from_argb_float(&color), expected);
    /// ```
    fn from_argb_float(color: &ColorARGBFloat) -> Self;

    /// Return a copy, normalizing RGB as a vector for vector maps.
    ///
    /// Vector maps place (0.5,0.5,0.5) as the origin.
    fn vector_normalize(&self) -> Self {
        let rgb = self.rgb_float().vector_normalize();
        let a = self.alpha_float();
        Color::from_argb_float(&ColorARGBFloat::combine(a, &rgb))
    }

    /// Compress for gamma correction.
    ///
    /// Alpha is not affected by this operation.
    ///
    /// # Examples (with [`ColorARGBFloat`])
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{Color, ColorARGBFloat};
    ///
    /// let color = ColorARGBFloat { alpha: 1.0, red: 0.25, green: 0.25, blue: 0.25 };
    /// let expected = ColorARGBFloat { alpha: 1.0, red: 0.5, green: 0.5, blue: 0.5 };
    ///
    /// assert_eq!(color.gamma_compress(), expected);
    /// ```
    fn gamma_compress(&self) -> Self {
        let rgb = self.rgb_float().clamp().gamma_compress();
        let a = self.alpha_float();
        Color::from_argb_float(&ColorARGBFloat::combine(a, &rgb))
    }

    /// Decompress for gamma correction.
    ///
    /// Alpha is not affected by this operation.
    ///
    /// # Examples (with [`ColorARGBFloat`])
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{Color, ColorARGBFloat};
    ///
    /// let color = ColorARGBFloat { alpha: 1.0, red: 0.5, green: 0.5, blue: 0.5 };
    /// let expected = ColorARGBFloat { alpha: 1.0, red: 0.25, green: 0.25, blue: 0.25 };
    ///
    /// assert_eq!(color.gamma_decompress(), expected);
    /// ```
    fn gamma_decompress(&self) -> Self {
        let rgb = self.rgb_float().clamp().gamma_decompress();
        let a = self.alpha_float();
        Color::from_argb_float(&ColorARGBFloat::combine(a, &rgb))
    }

    /// Alpha blend this color with a source.
    ///
    /// # Examples (with [`ColorARGBFloat`])
    ///
    /// ```
    /// use ringhopper_primitives::primitive::{Color, ColorARGBFloat};
    ///
    /// let color = ColorARGBFloat { alpha: 1.0, red: 1.0, green: 1.0, blue: 1.0 };
    /// let source = ColorARGBFloat { alpha: 0.25, red: 0.0, green: 0.0, blue: 1.0 };
    /// let expected = ColorARGBFloat { alpha: 1.0, red: 0.75, green: 0.75, blue: 1.0 };
    ///
    /// assert_eq!(color.alpha_blend(&source), expected);
    /// ```
    fn alpha_blend<C: Color>(&self, source: &C) -> Self {
        let rgb_this = self.rgb_float().clamp();
        let rgb_source = source.rgb_float().clamp();

        let alpha_this = self.alpha_float();
        let alpha_source = source.alpha_float();

        let blend = alpha_this * (1.0 - alpha_source);

        let alpha = alpha_source + blend;
        let red = rgb_source.red * alpha_source + rgb_this.red * blend;
        let green = rgb_source.green * alpha_source + rgb_this.green * blend;
        let blue = rgb_source.blue * alpha_source + rgb_this.blue * blend;

        Color::from_argb_float(&ColorARGBFloat { alpha, red, green, blue })
    }

    /// Clamp the color within the range of 0.0 to 1.0.
    fn clamp(&self) -> Self {
        let rgb = self.rgb_float().clamp();
        let alpha = self.alpha_float().clamp(0.0, 1.0);

        Color::from_argb_float(&ColorARGBFloat::combine(alpha, &rgb))
    }
}

/// Refers to a color composed of floats with an alpha channel.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct ColorARGBFloat {
    pub alpha: f32,
    pub red: f32,
    pub green: f32,
    pub blue: f32
}

generate_tag_data_simple_primitive_code!(ColorARGBFloat, f32, alpha, red, green, blue);

impl ColorARGBFloat {
    const fn combine(alpha: f32, rgb: &ColorRGBFloat) -> Self {
        ColorARGBFloat { alpha, red: rgb.red, green: rgb.green, blue: rgb.blue }
    }
}

impl Color for ColorARGBFloat {
    fn rgb_float(&self) -> ColorRGBFloat {
        ColorRGBFloat { red: self.red, green: self.green, blue: self.blue }
    }

    fn alpha_float(&self) -> f32 {
        self.alpha
    }

    fn from_argb_float(color: &ColorARGBFloat) -> Self {
        *color
    }
}

/// Refers to a color composed of floats without an alpha channel.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct ColorRGBFloat {
    pub red: f32,
    pub green: f32,
    pub blue: f32
}

impl Color for ColorRGBFloat {
    fn rgb_float(&self) -> ColorRGBFloat {
        *self
    }

    fn alpha_float(&self) -> f32 {
        1.0
    }

    fn from_argb_float(color: &ColorARGBFloat) -> Self {
        color.rgb_float()
    }

    fn vector_normalize(&self) -> Self {
        use super::Vector;

        const HALF: f32 = 0.5;
        const HALF_VECTOR: Vector3D = Vector3D { x: HALF, y: HALF, z: HALF };

        let clamped = self.clamp();

        let vector = Vector3D { x: clamped.red, y: clamped.green, z: clamped.blue }
            .sub(&HALF_VECTOR)
            .normalize_into(HALF)
            .add(&HALF_VECTOR);

        ColorRGBFloat { red: vector.x, green: vector.y, blue: vector.z }
    }

    fn gamma_compress(&self) -> Self {
        // This is approximate but fairly close, as the exponent is slightly higher than 2.
        ColorRGBFloat { red: self.red.sqrt(), green: self.green.sqrt(), blue: self.blue.sqrt() }
    }

    fn gamma_decompress(&self) -> Self {
        // This is approximate but fairly close, as the exponent is slightly higher than 2.
        ColorRGBFloat { red: self.red*self.red, green: self.green*self.green, blue: self.blue*self.blue }
    }

    fn clamp(&self) -> Self {
        ColorRGBFloat {
            red: self.red.clamp(0.0, 1.0),
            green: self.green.clamp(0.0, 1.0),
            blue: self.blue.clamp(0.0, 1.0)
        }
    }
}

generate_tag_data_simple_primitive_code!(ColorRGBFloat, f32, red, green, blue);

/// Refers to a color composed of 8-bit integer color with alpha stored in integer form.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(transparent)]
pub struct ColorARGBInt {
    pub color: u32
}

impl Color for ColorARGBInt {
    fn rgb_float(&self) -> ColorRGBFloat {
        let color: ColorARGBIntBytes = (*self).into();
        color.rgb_float()
    }

    fn alpha_float(&self) -> f32 {
        let color: ColorARGBIntBytes = (*self).into();
        color.alpha_float()
    }

    fn from_argb_float(color: &ColorARGBFloat) -> Self {
        ColorARGBIntBytes::from_argb_float(color).into()
    }

    fn clamp(&self) -> Self {
        let color: ColorARGBIntBytes = (*self).into();
        color.clamp().into()
    }
}

impl TagDataSimplePrimitive for ColorARGBInt {
    fn size() -> usize {
        <u32 as TagDataSimplePrimitive>::size()
    }

    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        Ok(Self { color: u32::read::<B>(data, at, struct_end)? })
    }

    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        self.color.write::<B>(data, at, struct_end)
    }

    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::ColorARGBInt
    }
}

impl Display for ColorARGBInt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let bytes: ColorARGBIntBytes = (*self).into();
        Display::fmt(&bytes, f)
    }
}

impl Display for ColorARGBIntBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{{ alpha = {}, red = {}, green = {}, blue = {} }}", self.alpha, self.red, self.green, self.blue ))
    }
}


/// Refers to a color composed of 8-bit integer color with alpha.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
#[repr(C)]
pub struct ColorARGBIntBytes {
    pub alpha: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8
}

impl Color for ColorARGBIntBytes {
    fn rgb_float(&self) -> ColorRGBFloat {
        ColorRGBFloat { red: self.red as f32 / 255.0, green: self.green as f32 / 255.0, blue: self.blue as f32 / 255.0 }
    }

    fn alpha_float(&self) -> f32 {
        self.alpha as f32 / 255.0
    }

    fn from_argb_float(color: &ColorARGBFloat) -> Self {
        let color = color.clamp();
        Self {
            alpha: (color.alpha * 255.0) as u8,
            red: (color.red * 255.0) as u8,
            green: (color.green * 255.0) as u8,
            blue: (color.blue * 255.0) as u8
        }
    }

    fn clamp(&self) -> Self {
        // No need to clamp
        *self
    }
}

impl From<u32> for ColorARGBIntBytes {
    fn from(value: u32) -> Self {
        Self {
            alpha: ((value & 0xFF000000) >> 24) as u8,
            red:   ((value & 0x00FF0000) >> 16) as u8,
            green: ((value & 0x0000FF00) >>  8) as u8,
            blue:  ((value & 0x000000FF) >>  0) as u8,
        }
    }
}

impl From<ColorARGBInt> for u32 {
    fn from(value: ColorARGBInt) -> Self {
        value.color
    }
}

impl From<u32> for ColorARGBInt {
    fn from(value: u32) -> Self {
        Self {
            color: value
        }
    }
}

impl From<ColorARGBIntBytes> for u32 {
    fn from(value: ColorARGBIntBytes) -> Self {
        ((value.alpha as u32)   << 24)
        | ((value.red as u32)   << 16)
        | ((value.green as u32) <<  8)
        | ((value.blue as u32)  <<  0)
    }
}

impl From<ColorARGBIntBytes> for ColorARGBInt {
    fn from(value: ColorARGBIntBytes) -> Self {
        Self { color: value.into() }
    }
}

impl From<ColorARGBInt> for ColorARGBIntBytes {
    fn from(value: ColorARGBInt) -> Self {
        value.color.into()
    }
}

impl TagDataSimplePrimitive for ColorARGBIntBytes {
    fn size() -> usize {
        std::mem::size_of::<u32>()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        u32::read::<B>(data, at, struct_end).map(|f| f.into())
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        let v: u32 = (*self).into();
        v.write::<B>(data, at, struct_end)
    }

    fn primitive_type() -> SimplePrimitiveType where Self: Sized {
        SimplePrimitiveType::ColorARGBIntBytes
    }
}
