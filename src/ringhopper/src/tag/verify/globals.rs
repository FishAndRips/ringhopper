use primitives::{dynamic::DynamicTagDataArray, primitive::TagPath};
use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::{Globals, ScenarioType};
use crate::tag::tree::TagTree;
use super::{VerifyContext, VerifyResult};

pub fn verify_globals<T: TagTree>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, context: &mut VerifyContext<T>, result: &mut VerifyResult) {
    let globals: &Globals = tag.as_any().downcast_ref().unwrap();

    let scenario_type = context.scenario_type;
    let engine_name = context.engine.name;

    if scenario_type == ScenarioType::Multiplayer {
        if globals.multiplayer_information.items.is_empty() {
            result.errors.push(format!("Multiplayer information reflexive is empty (must have at least 1 for {scenario_type} scenarios)"));
        }
        let required_weapons = 16; // TODO: TO BE FILLED BY O.E.M.
        if globals.weapon_list.items.len() < required_weapons {
            result.errors.push(format!("Weapons list is empty (must have at least {required_weapons} for {scenario_type} scenarios on {engine_name} engines)"));
        }
        for (i, mp) in ziperator!(globals.multiplayer_information) {
            let required_sounds = 43; // TODO: TO BE FILLED BY O.E.M.
            if mp.sounds.len() < required_sounds {
                result.warnings.push(format!("Sounds reflexive for MP information #{i} has only {} / {required_sounds} sounds for {engine_name} engines", mp.sounds.len()));
            }
        }
    }

    if scenario_type != ScenarioType::UserInterface {
        if globals.falling_damage.items.is_empty() {
            result.errors.push(format!("Falling damage reflexive is empty (must have at least 1 for {scenario_type} scenarios)"));
        }
        if globals.materials.items.len() < 32 {
            result.errors.push(format!("Materials reflexive is empty (must have at least 32 for {scenario_type} scenarios)"));
        }
    }

    // Check Gearbox netcode limitiations

    let uses_gearbox_netcode_limitations = true; // TODO: TO BE FILLED BY O.E.M.
    if uses_gearbox_netcode_limitations && scenario_type == ScenarioType::Multiplayer {
        pub const MAX_GRENADE_TYPE_MP: usize = 2;
        pub const MAX_GRENADE_COUNT_MP: u16 = 7;

        for (i, grenade) in ziperator!(globals.grenades) {
            if grenade.maximum_count > MAX_GRENADE_COUNT_MP {
                result.errors.push(format!("Grenade #{i}'s maximum count ({}) exceeds the maximum number of grenades ({MAX_GRENADE_COUNT_MP}) supported by the netcode.", grenade.maximum_count));
            }
            if grenade.mp_spawn_default > MAX_GRENADE_COUNT_MP {
                result.errors.push(format!("Grenade #{i}'s MP spawn count ({}) exceeds the maximum number of grenades ({MAX_GRENADE_COUNT_MP}) supported by the netcode.", grenade.mp_spawn_default));
            }
        }

        if globals.grenades.items.len() > MAX_GRENADE_TYPE_MP {
            result.errors.push(format!("Grenade reflexive size ({}) exceeds the maximum number of grenades types ({MAX_GRENADE_TYPE_MP}) supported by the netcode.", globals.grenades.items.len()));
        }
    }
}
