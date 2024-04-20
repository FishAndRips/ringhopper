macro_rules! ziperator {
    ($reflexive:expr) => {{
        (0..$reflexive.items.len()).zip($reflexive.items.iter())
    }};
}

mod object;
mod effect;
mod bitmap;
mod globals;
mod hud_interface;
mod model;
mod dependencies;
mod unicode_string_list;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::{TagGroup, TagPath, TagReference};
use primitives::tag::PrimaryTagStructDyn;
use ringhopper_engines::Engine;
use ringhopper_structs::{Globals, Scenario, ScenarioType};
use crate::tag::object::is_object;
use crate::tag::tree::TagTree;
use self::bitmap::verify_bitmap;
use self::object::*;
use self::effect::*;
use self::globals::*;
use self::model::*;
use self::hud_interface::*;
use self::dependencies::*;
use self::unicode_string_list::*;
use super::dependency::recursively_get_dependencies_for_map;

#[derive(Clone, Default)]
pub struct VerifyResult {
    pub pedantic_warnings: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl VerifyResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn combine(&mut self, mut other: VerifyResult) {
        self.pedantic_warnings.append(&mut other.pedantic_warnings);
        self.warnings.append(&mut other.warnings);
        self.errors.append(&mut other.errors);
    }
}

pub struct VerifyContext<'a, 'b, T: TagTree> {
    scenario: Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>,
    hud_globals: Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>,
    scenario_type: ScenarioType,
    engine: &'b Engine,
    tag_tree: &'a T,
    results: HashMap<TagPath, VerifyResult>
}

impl<'a, 'b, T: TagTree> VerifyContext<'a, 'b, T> {
    pub fn verify(scenario: &TagPath, tag_tree: &'a T, engine: &'b Engine) -> RinghopperResult<HashMap<TagPath, VerifyResult>> {
        let globals_path = TagPath::new("globals\\globals", TagGroup::Globals).unwrap();

        let scenario_ref = tag_tree.open_tag_shared(scenario)?;
        let globals_ref = tag_tree.open_tag_shared(&globals_path)?;

        let globals_tag_lock = globals_ref.lock().unwrap();
        let globals_tag = globals_tag_lock.as_any().downcast_ref::<Globals>().unwrap();
        let hud_globals_ref;

        if let Some(n) = globals_tag.interface_bitmaps.items.first() {
            let hud_globals_path = n.hud_globals
                .path()
                .ok_or_else(|| Error::InvalidTagData(format!("{globals_path} does not have HUD globals set for interface bitmaps #0")))?;
            hud_globals_ref = tag_tree.open_tag_shared(hud_globals_path)?;
        }
        else {
            return Err(Error::InvalidTagData(format!("{globals_path} is missing an interface bitmaps reflexive")))
        }

        drop(globals_tag_lock);

        let mut context = VerifyContext {
            scenario: scenario_ref,
            hud_globals: hud_globals_ref,
            scenario_type: ScenarioType::Multiplayer,
            engine,
            tag_tree,
            results: HashMap::new()
        };

        let all_dependencies = recursively_get_dependencies_for_map(scenario, tag_tree, engine)
            .map_err(|e| Error::Other(format!("Failed to query dependencies: {e}")))?;
        context.scenario_type = context.scenario.lock().unwrap().as_any().downcast_ref::<Scenario>().unwrap()._type;

        for path in all_dependencies {
            if context.results.contains_key(&path) {
                continue
            }

            let tag = tag_tree
                .open_tag_shared(&path)
                .map_err(|e| Error::Other(format!("Could not open {path}: {e}")))?;

            context.verify_tag(&path, tag.lock().unwrap().as_mut());
        }

        Ok(context.results)
    }

    fn verify_tag(&mut self, path: &TagPath, tag: &mut dyn PrimaryTagStructDyn) -> bool {
        // Do we need to re-verify?
        if self.results.contains_key(path) {
            return true
        }

        let mut result = VerifyResult::default();
        let group = path.group();

        // TODO: Verify indices

        verify_dependencies(tag, path, self, &mut result);

        // Verify supergroups
        if is_object(group) {
            verify_object(tag, path, self, &mut result);
        }

        match group {
            TagGroup::Weapon => verify_weapon(tag, path, self, &mut result),
            TagGroup::Effect => verify_effect(tag, path, self, &mut result),
            TagGroup::HUDGlobals => verify_hud_globals(tag, path, self, &mut result),
            TagGroup::WeaponHUDInterface => verify_weapon_hud_interface(tag, path, self, &mut result),
            TagGroup::UnitHUDInterface => verify_unit_hud_interface(tag, path, self, &mut result),
            TagGroup::GrenadeHUDInterface => verify_grenade_hud_interface(tag, path, self, &mut result),
            TagGroup::Globals => verify_globals(tag, path, self, &mut result),
            TagGroup::Model => verify_model(tag, path, self, &mut result),
            TagGroup::GBXModel => verify_gbxmodel(tag, path, self, &mut result),
            TagGroup::UnicodeStringList => verify_unicode_string_list(tag, path, self, &mut result),
            TagGroup::Bitmap => verify_bitmap(tag, path, self, &mut result),
            _ => ()
        }

        let is_ok = result.is_ok();
        self.results.insert(path.to_owned(), result);

        is_ok
    }

    fn open_tag_reference_maybe(&mut self, tag_reference: &TagReference, result: &mut VerifyResult, must_be_set_error: Option<&'static str>) -> Option<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        match tag_reference {
            TagReference::Set(tp) => self.open_tag_maybe(tp, result),
            TagReference::Null(_) => {
                if let Some(n) = must_be_set_error {
                    result.errors.push(n.to_string());
                }
                None
            }
        }
    }

    fn open_tag_maybe(&mut self, tag_path: &TagPath, result: &mut VerifyResult) -> Option<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        self.open_tag_unverified(tag_path, result).and_then(|n| {
            if self.verify_tag(tag_path, n.lock().unwrap().as_mut()) {
                Some(n)
            }
            else {
                result.errors.push(format!("{tag_path} has errors and could not be opened"));
                None
            }
        })
    }

    fn open_tag_unverified(&mut self, tag_path: &TagPath, result: &mut VerifyResult) -> Option<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        match self.tag_tree.open_tag_shared(tag_path) {
            Ok(n) => Some(n),
            Err(e) => {
                result.errors.push(format!("Unable to open {tag_path}: {e}"));
                None
            }
        }
    }
}

pub type VerifyFn<T> = fn(tag: &dyn PrimaryTagStructDyn, path: &TagPath, context: &mut VerifyContext<T>) -> VerifyResult;
