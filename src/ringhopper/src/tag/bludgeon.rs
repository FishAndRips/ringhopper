use primitives::{primitive::TagGroup, tag::PrimaryTagStructDyn};

mod sound;

pub enum BludgeonResult {
    Done,
    CannotRepair
}

pub fn bludgeon_tag(tag: &mut dyn PrimaryTagStructDyn) -> BludgeonResult {
    match tag.group() {
        TagGroup::Sound => sound::repair_sound(tag),
        _ => BludgeonResult::Done
    }
}
