macro_rules! ziperator {
    ($reflexive:expr) => {{
        (0..$reflexive.items.len()).zip($reflexive.items.iter())
    }};
}

pub mod unicode_string_list;
pub mod tree;
pub mod dependency;
pub mod tag_collection;
pub mod nudge;
pub mod compare;
pub mod convert;
pub mod model;
pub mod model_animations;
pub mod scenario;
pub mod object;
pub mod scenario_structure_bsp;
pub mod bitmap;
pub mod archive;
pub mod recover;
pub mod verify;
pub mod sound;
pub mod default;
pub mod bludgeon;
