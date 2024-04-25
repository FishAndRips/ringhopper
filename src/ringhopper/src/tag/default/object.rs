use primitives::tag::PrimaryTagStructDyn;

use crate::tag::object::downcast_base_object_mut;

pub fn unset_defaults_for_object(tag: &mut dyn PrimaryTagStructDyn) {
    let object = downcast_base_object_mut(tag).unwrap();
    if object.render_bounding_radius == object.bounding_radius {
        object.render_bounding_radius = 0.0
    }
}

pub fn set_defaults_for_object(tag: &mut dyn PrimaryTagStructDyn) {
    let object = downcast_base_object_mut(tag).unwrap();
    if object.render_bounding_radius == 0.0 {
        object.render_bounding_radius = object.bounding_radius
    }
}
