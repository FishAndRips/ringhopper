use primitives::{primitive::{TagGroup, TagPath}, tag::PrimaryTagStructDyn};

mod sound;
mod model;
mod scenario;
mod unicode_string_list;
mod scenario_structure_bsp;
mod floats;

pub enum BludgeonResult {
    Done,
    CannotRepair
}

pub fn bludgeon_tag(tag: &mut dyn PrimaryTagStructDyn, path: &TagPath) -> BludgeonResult {
    floats::fix_bad_floats(tag);

    match tag.group() {
        TagGroup::Model | TagGroup::GBXModel => model::repair_model(tag),
        TagGroup::Sound => sound::repair_sound(tag),
        TagGroup::Scenario => scenario::repair_scenario(tag, path),
        TagGroup::UnicodeStringList => unicode_string_list::repair_unicode_string_list(tag),
        TagGroup::ScenarioStructureBSP => scenario_structure_bsp::repair_scenario_structure_bsp(tag),

        // TODO: indices
        // TODO: out-of-range clamping

        // VERIFY THAT THIS IS NOT JUST AUTOMATICALLY FIXED:
        // - TODO: uppercase tag references??

        _ => BludgeonResult::Done
    }
}
