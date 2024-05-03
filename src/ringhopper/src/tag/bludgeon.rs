use primitives::{primitive::{TagGroup, TagPath}, tag::PrimaryTagStructDyn};

mod sound;
mod model;
mod scenario;

pub enum BludgeonResult {
    Done,
    CannotRepair
}

pub fn bludgeon_tag(tag: &mut dyn PrimaryTagStructDyn, path: &TagPath) -> BludgeonResult {
    match tag.group() {
        TagGroup::Model | TagGroup::GBXModel => model::repair_model(tag),
        TagGroup::Sound => sound::repair_sound(tag),
        TagGroup::Scenario => scenario::repair_scenario(tag, path),

        // TODO: unicode_string_list null termination and line endings
        // TODO: non normal vectors
        // TODO: indices
        // TODO: out-of-range clamping

        // VERIFY THAT THIS IS NOT JUST AUTOMATICALLY FIXED:
        // - TODO: uppercase tag references??

        _ => BludgeonResult::Done
    }
}
