use primitives::dynamic::{DynamicTagData, DynamicTagDataType, SimplePrimitiveType};
use primitives::primitive::{Angle, ColorARGBFloat, ColorRGBFloat, Euler2D, Euler3D, Matrix3x3, Plane2D, Plane3D, Quaternion, Vector2D, Vector3D};
use primitives::tag::{for_each_field_mut, PrimaryTagStructDyn};

pub fn fix_bad_floats(tag: &mut dyn PrimaryTagStructDyn) {
    fn check_field(field: &mut dyn DynamicTagData) {
        match field.data_type() {
            DynamicTagDataType::SimplePrimitive(t) => match t {
                SimplePrimitiveType::F32 => {
                    let f: &mut f32 = field.as_any_mut().downcast_mut().unwrap();
                    if f.is_nan() {
                        *f = 0.0;
                    }
                }
                SimplePrimitiveType::Angle => {
                    let f: &mut Angle = field.as_any_mut().downcast_mut().unwrap();
                    check_field(&mut f.angle);
                }
                SimplePrimitiveType::Euler2D => {
                    let f: &mut Euler2D = field.as_any_mut().downcast_mut().unwrap();
                    check_field(&mut f.yaw);
                    check_field(&mut f.pitch);
                }
                SimplePrimitiveType::Euler3D => {
                    let f: &mut Euler3D = field.as_any_mut().downcast_mut().unwrap();
                    check_field(&mut f.yaw);
                    check_field(&mut f.pitch);
                    check_field(&mut f.roll);
                }
                SimplePrimitiveType::Matrix3x3 => {
                    let f: &mut Matrix3x3 = field.as_any_mut().downcast_mut().unwrap();
                    for i in &mut f.vectors {
                        check_field(&mut i.x);
                        check_field(&mut i.y);
                        check_field(&mut i.z);
                    }
                }
                SimplePrimitiveType::Plane2D => {
                    let f: &mut Plane2D = field.as_any_mut().downcast_mut().unwrap();
                    check_field(&mut f.d);
                    check_field(&mut f.vector);
                }
                SimplePrimitiveType::Plane3D => {
                    let f: &mut Plane3D = field.as_any_mut().downcast_mut().unwrap();
                    check_field(&mut f.d);
                    check_field(&mut f.vector);
                }
                SimplePrimitiveType::Quaternion => {
                    let f: &mut Quaternion = field.as_any_mut().downcast_mut().unwrap();
                    check_field(&mut f.w);
                    check_field(&mut f.x);
                    check_field(&mut f.y);
                    check_field(&mut f.z);
                }
                SimplePrimitiveType::Vector2D => {
                    let f: &mut Vector2D = field.as_any_mut().downcast_mut().unwrap();
                    check_field(&mut f.x);
                    check_field(&mut f.y);
                }
                SimplePrimitiveType::Vector3D => {
                    let f: &mut Vector3D = field.as_any_mut().downcast_mut().unwrap();
                    check_field(&mut f.x);
                    check_field(&mut f.y);
                    check_field(&mut f.z);
                }
                SimplePrimitiveType::ColorARGBFloat => {
                    let f: &mut ColorARGBFloat = field.as_any_mut().downcast_mut().unwrap();
                    check_field(&mut f.alpha);
                    check_field(&mut f.red);
                    check_field(&mut f.green);
                    check_field(&mut f.blue);

                    f.alpha = f.alpha.clamp(0.0, 1.0);
                    f.red = f.red.clamp(0.0, 1.0);
                    f.green = f.green.clamp(0.0, 1.0);
                    f.blue = f.blue.clamp(0.0, 1.0);
                }
                SimplePrimitiveType::ColorRGBFloat => {
                    let f: &mut ColorRGBFloat = field.as_any_mut().downcast_mut().unwrap();
                    check_field(&mut f.red);
                    check_field(&mut f.green);
                    check_field(&mut f.blue);

                    f.red = f.red.clamp(0.0, 1.0);
                    f.green = f.green.clamp(0.0, 1.0);
                    f.blue = f.blue.clamp(0.0, 1.0);
                }
                _ => return
            },
            _ => return
        }
    }

    for_each_field_mut(tag.as_mut_dynamic(), true, |_, field| {
        check_field(field);
    });
}