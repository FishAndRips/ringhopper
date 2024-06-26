#[macro_use]
pub(crate) mod macros {
    macro_rules! generate_tag_data_simple_primitive_code_read {
        ($self: expr, $b: tt, $field_type: ty, $data: expr, $current_offset: tt, $struct_end: expr, $field: tt) => {
            $self.$field = <$field_type as SimpleTagData>::read::<$b>($data, $current_offset, $struct_end)?.into();
            $current_offset = $current_offset.add_overflow_checked(<$field_type>::simple_size())?;
        };
        ($self: expr, $b: tt, $field_type: ty, $data: expr, $current_offset: tt, $struct_end: expr, $field: tt, $($fields: tt), +) => {
            generate_tag_data_simple_primitive_code_read!($self, $b, $field_type, $data, $current_offset, $struct_end, $field);
            generate_tag_data_simple_primitive_code_read!($self, $b, $field_type, $data, $current_offset, $struct_end, $($fields), +);
        };
    }

    macro_rules! generate_tag_data_simple_primitive_code_write {
        ($self: expr, $b: tt, $field_type: ty, $data: expr, $current_offset: tt, $struct_end: expr, $field: tt) => {
            $self.$field.write::<$b>($data, $current_offset, $struct_end)?;
            $current_offset = $current_offset.add_overflow_checked(<$field_type>::simple_size())?;
        };
        ($self: expr, $b: tt, $field_type: ty, $data: expr, $current_offset: tt, $struct_end: expr, $field: tt, $($fields: tt), +) => {
            generate_tag_data_simple_primitive_code_write!($self, $b, $field_type, $data, $current_offset, $struct_end, $field);
            generate_tag_data_simple_primitive_code_write!($self, $b, $field_type, $data, $current_offset, $struct_end, $($fields), +);
        };
    }

    macro_rules! count_sizes {
        ($base_size: expr, $field: tt) => {
            $base_size
        };
        ($base_size: expr, $field: tt, $($fields: tt), +) => {
            count_sizes!($base_size, $field) + count_sizes!($base_size, $($fields), +)
        }
    }

    macro_rules! generate_display_simple_primitive_code {
        ($self: expr, $fmt: expr, $field: tt) => {
            $fmt.write_fmt(format_args!("{} = {}", stringify!($field), $self.$field))?;
        };
        ($self: expr, $fmt: expr, $field: tt, $($fields: tt), +) => {
            generate_display_simple_primitive_code!($self, $fmt, $field);
            $fmt.write_str(", ")?;
            generate_display_simple_primitive_code!($self, $fmt, $($fields), +);
        };
    }

    macro_rules! generate_tag_data_simple_primitive_code {
        ($what: tt, $field_type: ty, $($fields: tt),+) => {
            #[allow(unused_assignments)]
            impl SimpleTagData for $what {
                fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
                    let mut current_offset = at;
                    let mut r = Self::default();
                    generate_tag_data_simple_primitive_code_read!(r, B, $field_type, data, current_offset, struct_end, $($fields), +);
                    Ok(r)
                }
                fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
                    let mut current_offset = at;
                    generate_tag_data_simple_primitive_code_write!(self, B, $field_type, data, current_offset, struct_end, $($fields), +);
                    Ok(())
                }
                fn simple_size() -> usize {
                    count_sizes!(<$field_type>::simple_size(), $($fields), +)
                }
            }

            impl SimplePrimitive for $what {
                fn primitive_type() -> SimplePrimitiveType {
                    SimplePrimitiveType::$what
                }
            }

            impl std::fmt::Display for $what {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("{")?;
                    generate_display_simple_primitive_code!(self, f, $($fields), +);
                    f.write_str("}")?;
                    Ok(())
                }
            }
        };
    }
}

mod vector;
use std::convert::TryInto;

use crate::parse::SimpleTagData;
use crate::error::*;
use crate::parse::tag_data_fits;
use byteorder::*;

pub use self::vector::*;

mod group;
pub use self::group::*;

mod plane;
pub use self::plane::*;

mod path;
pub use self::path::*;

mod color;
pub use self::color::*;

mod data;
pub use self::data::*;

mod string;
pub use self::string::*;

mod array;
pub use self::array::*;

macro_rules! define_primitive_ref {
    ($name: tt, $($reference: tt), *) => {
        /// Defines a primitive reference.
        pub enum $name<'a> {
            TagGroup($($reference)* TagGroup),
            Plane2D($($reference)* Plane2D),
            Plane3D($($reference)* Plane3D),
            TagReference($($reference)* TagReference),
            ColorRGBFloat($($reference)* ColorRGBFloat),
            ColorARGBFloat($($reference)* ColorARGBFloat),
            ColorARGBInt($($reference)* ColorARGBInt),
            Data($($reference)* Data),
            Euler2D($($reference)* Euler2D),
            Euler3D($($reference)* Euler3D),
            Matrix3x3($($reference)* Matrix3x3),
            Address($($reference)* Address),
            Angle($($reference)* Angle),
            Vector2D($($reference)* Vector2D),
            Vector3D($($reference)* Vector3D),
            Quaternion($($reference)* Quaternion),
            String32($($reference)* String32),
        }
    };
}

define_primitive_ref!(PrimitiveRef, &, 'a);
define_primitive_ref!(PrimitiveRefMut, &, 'a, mut);
