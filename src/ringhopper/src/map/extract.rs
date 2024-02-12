use definitions::{Scenario, ScenarioType, Weapon};
use primitives::primitive::{Angle, Bounds, TagPath};

pub fn fix_extracted_weapon_tag(tag: &mut Weapon, tag_path: &TagPath, scenario_tag: &Scenario) {
    if scenario_tag._type == ScenarioType::Singleplayer && !scenario_tag.flags.do_not_apply_bungie_campaign_tag_patches {
        match tag_path.path() {
            "weapons\\pistol\\pistol" => {
                if let Some(n) = tag.triggers.items.get_mut(0) {
                    n.minimum_error = Angle::from_degrees(0.0);
                    n.error_angle = Bounds {
                        lower: Angle::from_degrees(0.2),
                        upper: Angle::from_degrees(2.0)
                    };
                }
            }
            "weapons\\plasma rifle\\plasma rifle" => {
                if let Some(n) = tag.triggers.items.get_mut(0) {
                    n.error_angle = Bounds {
                        lower: Angle::from_degrees(0.5),
                        upper: Angle::from_degrees(5.0)
                    };
                }
            }
            _ => ()
        }
    }
}
