use definitions::*;
use primitives::primitive::TagGroup;
use primitives::tag::PrimaryTagStructDyn;

macro_rules! get_base_object_tag_memes {
    ($tag:expr, $as_any:tt, $downcast:tt, $map:expr, $unit:tt, $item:tt, $device:tt, $basic:tt) => {{
        let group = $tag.group();
        match group {
            TagGroup::Object => $tag.$as_any().$downcast(),

            TagGroup::Unit
            | TagGroup::Biped
            | TagGroup::Vehicle => $unit($tag).map($map),

            TagGroup::Item
            | TagGroup::Weapon
            | TagGroup::Garbage
            | TagGroup::Equipment => $item($tag).map($map),

            TagGroup::Device
            | TagGroup::DeviceMachine
            | TagGroup::DeviceControl
            | TagGroup::DeviceLightFixture => $device($tag).map($map),

            TagGroup::Projectile => $tag.$as_any().$downcast::<Projectile>().map($map),

            TagGroup::Scenery
            | TagGroup::Placeholder
            | TagGroup::SoundScenery => $basic($tag).map($map),

            _ => None
        }
    }};
}

/// Get a reference to the base object struct of the tag if the tag is an object tag.
pub fn downcast_base_object(tag: &dyn PrimaryTagStructDyn) -> Option<&Object> {
    get_base_object_tag_memes!(tag, as_any, downcast_ref, |o| &o.object, downcast_base_unit, downcast_base_item, downcast_base_device, downcast_basic_object)
}

/// Get a mutable reference to the base object struct of the tag if the tag is an object tag.
pub fn downcast_base_object_mut(tag: &mut dyn PrimaryTagStructDyn) -> Option<&mut Object> {
    get_base_object_tag_memes!(tag, as_any_mut, downcast_mut, |o| &mut o.object, downcast_base_unit_mut, downcast_base_item_mut, downcast_base_device_mut, downcast_basic_object_mut)
}


macro_rules! get_base_unit_tag_memes {
    ($tag:expr, $as_any:tt, $downcast:tt, $map:expr) => {{
        let group = $tag.group();
        let any = $tag.$as_any();
        match group {
            TagGroup::Biped => any.$downcast::<Biped>().map($map),
            TagGroup::Vehicle => any.$downcast::<Vehicle>().map($map),
            TagGroup::Unit => any.$downcast(),
            _ => None
        }
    }};
}

/// Get a reference to the base unit struct of the tag if the tag is a unit tag.
pub fn downcast_base_unit(tag: &dyn PrimaryTagStructDyn) -> Option<&Unit> {
    get_base_unit_tag_memes!(tag, as_any, downcast_ref, |o| &o.unit)
}

/// Get a mutable reference to the base unit struct of the tag if the tag is a unit tag.
pub fn downcast_base_unit_mut(tag: &mut dyn PrimaryTagStructDyn) -> Option<&mut Unit> {
    get_base_unit_tag_memes!(tag, as_any_mut, downcast_mut, |o| &mut o.unit)
}

macro_rules! get_basic_object_tag_memes {
    ($tag:expr, $as_any:tt, $downcast:tt, $map:expr) => {{
        let group = $tag.group();
        let any = $tag.$as_any();
        match group {
            TagGroup::Scenery => any.$downcast::<Scenery>().map($map),
            TagGroup::SoundScenery => any.$downcast::<SoundScenery>().map($map),
            TagGroup::Placeholder => any.$downcast::<Placeholder>().map($map),
            _ => None
        }
    }};
}

/// Get a reference to the basic object struct of the tag if the tag is a basic object tag.
pub fn downcast_basic_object(tag: &dyn PrimaryTagStructDyn) -> Option<&BasicObject> {
    get_basic_object_tag_memes!(tag, as_any, downcast_ref, |o| &o.basic_object)
}

/// Get a mutable reference to the basic object struct of the tag if the tag is a basic object tag.
pub fn downcast_basic_object_mut(tag: &mut dyn PrimaryTagStructDyn) -> Option<&mut BasicObject> {
    get_basic_object_tag_memes!(tag, as_any_mut, downcast_mut, |o| &mut o.basic_object)
}

macro_rules! get_base_item_tag_memes {
    ($tag:expr, $as_any:tt, $downcast:tt, $map:expr) => {{
        let group = $tag.group();
        let any = $tag.$as_any();
        match group {
            TagGroup::Weapon => any.$downcast::<Weapon>().map($map),
            TagGroup::Equipment => any.$downcast::<Equipment>().map($map),
            TagGroup::Garbage => any.$downcast::<Garbage>().map($map),
            TagGroup::Item => any.$downcast(),
            _ => None
        }
    }};
}

/// Get a reference to the base item struct of the tag if the tag is an item tag.
pub fn downcast_base_item(tag: &dyn PrimaryTagStructDyn) -> Option<&Item> {
    get_base_item_tag_memes!(tag, as_any, downcast_ref, |o| &o.item)
}

/// Get a reference to the base item struct of the tag if the tag is an item tag.
pub fn downcast_base_item_mut(tag: &mut dyn PrimaryTagStructDyn) -> Option<&mut Item> {
    get_base_item_tag_memes!(tag, as_any_mut, downcast_mut, |o| &mut o.item)
}

macro_rules! get_base_device_tag_memes {
    ($tag:expr, $as_any:tt, $downcast:tt, $map:expr) => {{
        let group = $tag.group();
        let any = $tag.$as_any();
        match group {
            TagGroup::DeviceMachine => any.$downcast::<DeviceMachine>().map($map),
            TagGroup::DeviceControl => any.$downcast::<DeviceControl>().map($map),
            TagGroup::DeviceLightFixture => any.$downcast::<DeviceLightFixture>().map($map),
            TagGroup::Device => any.$downcast(),
            _ => None
        }
    }};
}

pub fn downcast_base_device(tag: &dyn PrimaryTagStructDyn) -> Option<&Device> {
    get_base_device_tag_memes!(tag, as_any, downcast_ref, |o| &o.device)
}

pub fn downcast_base_device_mut(tag: &mut dyn PrimaryTagStructDyn) -> Option<&mut Device> {
    get_base_device_tag_memes!(tag, as_any_mut, downcast_mut, |o| &mut o.device)
}
