use primitives::{primitive::TagGroup, tag::PrimaryTagStructDyn};

mod sound;
mod model;

pub enum BludgeonResult {
    Done,
    CannotRepair
}

pub fn bludgeon_tag(tag: &mut dyn PrimaryTagStructDyn) -> BludgeonResult {
    match tag.group() {
        TagGroup::Model | TagGroup::GBXModel => model::repair_model(tag),
        TagGroup::Sound => sound::repair_sound(tag),
        _ => BludgeonResult::Done
    }
}
