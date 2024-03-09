use std::fmt::Display;
use crc64::crc64;
use primitives::dynamic::{DynamicEnum, DynamicTagData, DynamicTagDataArray, DynamicTagDataType, SimplePrimitiveType};
use primitives::primitive::{Address, Angle, ColorARGBFloat, ColorARGBInt, ColorARGBIntBytes, ColorRGBFloat, CompressedFloat, CompressedVector2D, CompressedVector3D, Data, Euler2D, Euler3D, FileData, ID, Index, Matrix3x3, Plane2D, Plane3D, Quaternion, Rectangle, ScenarioScriptNodeValue, String32, TagGroup, TagReference, Vector2D, Vector2DInt, Vector3D};
use primitives::tag::PrimaryTagStructDyn;

#[derive(Clone)]
pub struct TagComparisonDifference {
    pub depth: usize,
    pub path: String,
    pub difference: String
}

/// Compare two tags.
///
/// # Panics
///
/// The groups and internal structure must be the same, or else this function may panic or output bad results.
pub fn compare_tags(first: &dyn PrimaryTagStructDyn, second: &dyn PrimaryTagStructDyn) -> Vec<TagComparisonDifference> {
    assert_eq!(first.group(), second.group());

    let mut path = String::with_capacity(1024);
    let mut comparison = Vec::new();
    compare_tag_data_blocks(first, second, &mut path, &mut comparison, 0);

    comparison
}

fn compare_tag_data<T: DynamicTagData + ?Sized>(first: &T, second: &T, path: &mut String, comparison: &mut Vec<TagComparisonDifference>, depth: usize) {
    let data_type = first.data_type();
    debug_assert_eq!(data_type, second.data_type());

    match data_type {
        DynamicTagDataType::Reflexive | DynamicTagDataType::Array => {
            let first = first.as_array().unwrap();
            let second = second.as_array().unwrap();
            return compare_array(first, second, path, comparison, depth);
        },

        DynamicTagDataType::Enum => {
            let first = first.as_enum().unwrap();
            let second = second.as_enum().unwrap();
            return compare_enums(first, second, path, comparison, depth);
        },

        DynamicTagDataType::TagReference => {
            let first = first.as_any().downcast_ref::<TagReference>().unwrap();
            let second = second.as_any().downcast_ref::<TagReference>().unwrap();
            return compare_tag_references(first, second, path, comparison, depth);
        },

        DynamicTagDataType::Block => {
            return compare_tag_data_blocks(first, second, path, comparison, depth);
        },

        DynamicTagDataType::Data => {
            let first = first.as_any().downcast_ref::<Data>().unwrap();
            let second = second.as_any().downcast_ref::<Data>().unwrap();
            return compare_data(&first.bytes, &second.bytes, path, comparison, depth);
        },

        DynamicTagDataType::FileData => {
            let first = first.as_any().downcast_ref::<FileData>().unwrap();
            let second = second.as_any().downcast_ref::<FileData>().unwrap();
            return compare_data(&first.bytes, &second.bytes, path, comparison, depth);
        },

        DynamicTagDataType::SimplePrimitive(primitive_type) => {
            macro_rules! do_compare {
                ($prim:tt) => {
                    compare_primitive::<$prim>(
                        first.as_any().downcast_ref().unwrap(),
                        second.as_any().downcast_ref().unwrap(),
                        path,
                        comparison,
                        depth
                    )
                };
            }

            match primitive_type {
                SimplePrimitiveType::Bool => do_compare!(bool),
                SimplePrimitiveType::String32 => compare_string32(first.as_any().downcast_ref().unwrap(), second.as_any().downcast_ref().unwrap(), path, comparison, depth),
                SimplePrimitiveType::I8 => do_compare!(i8),
                SimplePrimitiveType::U8 => do_compare!(u8),
                SimplePrimitiveType::I16 => do_compare!(i16),
                SimplePrimitiveType::U16 => do_compare!(u16),
                SimplePrimitiveType::I32 => do_compare!(i32),
                SimplePrimitiveType::U32 => do_compare!(u32),
                SimplePrimitiveType::F32 => do_compare!(f32),
                SimplePrimitiveType::Size => do_compare!(usize),
                SimplePrimitiveType::Angle => do_compare!(Angle),
                SimplePrimitiveType::Vector2D => do_compare!(Vector2D),
                SimplePrimitiveType::Vector3D => do_compare!(Vector3D),
                SimplePrimitiveType::Plane2D => do_compare!(Plane2D),
                SimplePrimitiveType::Plane3D => do_compare!(Plane3D),
                SimplePrimitiveType::Euler2D => do_compare!(Euler2D),
                SimplePrimitiveType::Euler3D => do_compare!(Euler3D),
                SimplePrimitiveType::Quaternion => do_compare!(Quaternion),
                SimplePrimitiveType::Matrix3x3 => do_compare!(Matrix3x3),
                SimplePrimitiveType::Vector2DInt => do_compare!(Vector2DInt),
                SimplePrimitiveType::Rectangle => do_compare!(Rectangle),
                SimplePrimitiveType::ColorRGBFloat => do_compare!(ColorRGBFloat),
                SimplePrimitiveType::ColorARGBInt => do_compare!(ColorARGBInt),
                SimplePrimitiveType::ColorARGBIntBytes => do_compare!(ColorARGBIntBytes),
                SimplePrimitiveType::ColorARGBFloat => do_compare!(ColorARGBFloat),
                SimplePrimitiveType::Index => compare_index(first.as_any().downcast_ref().unwrap(), second.as_any().downcast_ref().unwrap(), path, comparison, depth),
                SimplePrimitiveType::ID => do_compare!(ID),
                SimplePrimitiveType::TagGroup => do_compare!(TagGroup),
                SimplePrimitiveType::Address => do_compare!(Address),
                SimplePrimitiveType::ScenarioScriptNodeValue => do_compare!(ScenarioScriptNodeValue),
                SimplePrimitiveType::CompressedVector3D => do_compare!(CompressedVector3D),
                SimplePrimitiveType::CompressedVector2D => do_compare!(CompressedVector2D),
                SimplePrimitiveType::CompressedFloat => do_compare!(CompressedFloat)
            }
        }
    }
}

fn compare_primitive<T: PartialEq + Display>(first: &T, second: &T, path: &mut String, comparison: &mut Vec<TagComparisonDifference>, depth: usize) {
    if first != second {
        comparison.push(TagComparisonDifference {
            depth,
            path: path[1..].to_owned(),
            difference: format!("value is different ({first} != {second})")
        });
    }
}

fn compare_index(first: &Index, second: &Index, path: &mut String, comparison: &mut Vec<TagComparisonDifference>, depth: usize) {
    if first != second {
        comparison.push(TagComparisonDifference {
            depth,
            path: path[1..].to_owned(),
            difference: format!("value is different ({first:?} != {second:?})")
        });
    }
}

fn compare_string32(first: &String32, second: &String32, path: &mut String, comparison: &mut Vec<TagComparisonDifference>, depth: usize) {
    if first != second {
        comparison.push(TagComparisonDifference {
            depth,
            path: path[1..].to_owned(),
            difference: format!("value is different (`{first}` != `{second}`)")
        });
    }
}

fn compare_tag_data_blocks<T: DynamicTagData + ?Sized>(first: &T, second: &T, path: &mut String, comparison: &mut Vec<TagComparisonDifference>, depth: usize) {
    let length_before = path.len();
    for i in first.fields() {
        *path += ".";
        *path += i;
        compare_tag_data(first.get_field(i).unwrap(), second.get_field(i).unwrap(), path, comparison, depth + 1);
        path.truncate(length_before);
    }
    return;
}

fn compare_data(first: &[u8], second: &[u8], path: &mut String, comparison: &mut Vec<TagComparisonDifference>, depth: usize) {
    let flength = first.len();
    let slength = second.len();

    if flength != slength {
        comparison.push(TagComparisonDifference {
            depth,
            path: path[1..].to_owned(),
            difference: format!("length is different ({flength} != {slength})")
        });
        return;
    }

    if first != second {
        let first = crc64(0, first);
        let second = crc64(0, second);
        let op = if first != second { "!=" } else { "~= (forged??)" };
        comparison.push(TagComparisonDifference {
            depth,
            path: path[1..].to_owned(),
            difference: format!("data is different (CRC64: {first:08X} {op} {second:08X})")
        });
        return;
    }
}

fn compare_enums(first: &dyn DynamicEnum, second: &dyn DynamicEnum, path: &mut String, comparison: &mut Vec<TagComparisonDifference>, depth: usize) {
    let first = first.get_enum_string_value();
    let second = second.get_enum_string_value();

    if first != second {
        comparison.push(TagComparisonDifference {
            depth,
            path: path[1..].to_owned(),
            difference: format!("enum is different (`{first}` != `{second}`)")
        });
    }
}

fn compare_array(first: &dyn DynamicTagDataArray, second: &dyn DynamicTagDataArray, path: &mut String, comparison: &mut Vec<TagComparisonDifference>, depth: usize) {
    let flength = first.len();
    let slength = second.len();

    // Cannot compare the arrays if the lengths are different
    if flength != slength {
        comparison.push(TagComparisonDifference {
            depth,
            path: path[1..].to_owned(),
            difference: format!("length is different ({flength} != {slength})")
        });
        return;
    }

    // Now compare!
    let length_before = path.len();
    for i in 0..flength {
        *path += &format!("[{i}]");
        compare_tag_data(first.get_at_index(i).unwrap(), second.get_at_index(i).unwrap(), path, comparison, depth + 1);
        path.truncate(length_before);
    }
}

fn compare_tag_references(first: &TagReference, second: &TagReference, path: &mut String, comparison: &mut Vec<TagComparisonDifference>, depth: usize) {
    if first == second {
        return
    }
    comparison.push(TagComparisonDifference {
        depth,
        path: path[1..].to_owned(),
        difference: format!("reference is different (`{first}` != `{second}`)")
    })
}
