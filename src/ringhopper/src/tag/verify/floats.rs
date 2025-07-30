use primitives::dynamic::{DynamicTagData, DynamicTagDataType, SimplePrimitiveType};
use primitives::primitive::{Angle, ColorARGBFloat, ColorRGBFloat, Euler2D, Euler3D, Matrix3x3, Plane2D, Plane3D, Quaternion, Vector2D, Vector3D};
use primitives::tag::{for_each_field, PrimaryTagStructDyn};
use crate::tag::result::TagResult;

pub fn check_bad_floats(tag: &dyn PrimaryTagStructDyn, result: &mut TagResult) {
    fn check_field(field: &dyn DynamicTagData, result: &mut TagResult) {
        let zero_to_one = 0.0f32..=1.0f32;

        match field.data_type() {
            DynamicTagDataType::SimplePrimitive(t) => match t {
                SimplePrimitiveType::Float => {
                    let f: &f32 = field.as_any().downcast_ref().unwrap();
                    if f.is_nan() {
                        result.errors.push("NaN float(s) detected. This can be automatically fixed.".to_string());
                    }
                }
                SimplePrimitiveType::Angle => {
                    let f: &Angle = field.as_any().downcast_ref().unwrap();
                    check_field(&f.angle, result);
                }
                SimplePrimitiveType::Euler2D => {
                    let f: &Euler2D = field.as_any().downcast_ref().unwrap();
                    check_field(&f.yaw, result);
                    check_field(&f.pitch, result);
                }
                SimplePrimitiveType::Euler3D => {
                    let f: &Euler3D = field.as_any().downcast_ref().unwrap();
                    check_field(&f.yaw, result);
                    check_field(&f.pitch, result);
                    check_field(&f.roll, result);
                }
                SimplePrimitiveType::Matrix3x3 => {
                    let f: &Matrix3x3 = field.as_any().downcast_ref().unwrap();
                    for i in f.vectors {
                        check_field(&i.x, result);
                        check_field(&i.y, result);
                        check_field(&i.z, result);
                    }
                }
                SimplePrimitiveType::Plane2D => {
                    let f: &Plane2D = field.as_any().downcast_ref().unwrap();
                    check_field(&f.d, result);
                    check_field(&f.vector, result);
                }
                SimplePrimitiveType::Plane3D => {
                    let f: &Plane3D = field.as_any().downcast_ref().unwrap();
                    check_field(&f.d, result);
                    check_field(&f.vector, result);
                }
                SimplePrimitiveType::Quaternion => {
                    let f: &Quaternion = field.as_any().downcast_ref().unwrap();
                    check_field(&f.w, result);
                    check_field(&f.x, result);
                    check_field(&f.y, result);
                    check_field(&f.z, result);
                }
                SimplePrimitiveType::Vector2D => {
                    let f: &Vector2D = field.as_any().downcast_ref().unwrap();
                    check_field(&f.x, result);
                    check_field(&f.y, result);
                }
                SimplePrimitiveType::Vector3D => {
                    let f: &Vector3D = field.as_any().downcast_ref().unwrap();
                    check_field(&f.x, result);
                    check_field(&f.y, result);
                    check_field(&f.z, result);
                }
                SimplePrimitiveType::ColorARGBFloat => {
                    let f: &ColorARGBFloat = field.as_any().downcast_ref().unwrap();
                    check_field(&f.alpha, result);
                    check_field(&f.red, result);
                    check_field(&f.green, result);
                    check_field(&f.blue, result);

                    if !zero_to_one.contains(&f.alpha) || !zero_to_one.contains(&f.red) || !zero_to_one.contains(&f.green) || !zero_to_one.contains(&f.blue) {
                        result.errors.push(format!("Color components ({f}) detect outside of 0-1 range. This can be automatically fixed."));
                    }
                }
                SimplePrimitiveType::ColorRGBFloat => {
                    let f: &ColorRGBFloat = field.as_any().downcast_ref().unwrap();
                    check_field(&f.red, result);
                    check_field(&f.green, result);
                    check_field(&f.blue, result);

                    if !zero_to_one.contains(&f.red) || !zero_to_one.contains(&f.green) || !zero_to_one.contains(&f.blue) {
                        result.errors.push(format!("Color components ({f}) detect outside of 0-1 range. This can be automatically fixed."));
                    }
                }
                _ => return
            },
            _ => return
        }
    }

    for_each_field(tag.as_dynamic(), |_, field| {
        check_field(field, result);
    });
}
