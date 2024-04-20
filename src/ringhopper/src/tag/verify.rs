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
use std::num::NonZeroUsize;
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

pub struct VerifyContext<T: TagTree + Send + Sync> {
    scenario: Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>,
    hud_globals: Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>,
    scenario_type: ScenarioType,
    engine: &'static Engine,
    tag_tree: Arc<T>,
    results: Mutex<HashMap<TagPath, VerifyResult>>
}

#[derive(PartialEq)]
enum VerifyStatus {
    Unverified,
    VerifiedError,
    VerifiedOK
}

impl<T: TagTree + Send + Sync + 'static> VerifyContext<T> {
    pub fn verify(scenario: &TagPath, tag_tree: T, engine: &'static Engine, threads: NonZeroUsize) -> RinghopperResult<HashMap<TagPath, VerifyResult>> {
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
            tag_tree: Arc::new(tag_tree),
            results: Mutex::new(HashMap::new())
        };

        let all_dependencies = recursively_get_dependencies_for_map(scenario, context.tag_tree.as_ref(), engine)
            .map_err(|e| Error::Other(format!("Failed to query dependencies: {e}")))?;
        context.scenario_type = context.scenario.lock().unwrap().as_any().downcast_ref::<Scenario>().unwrap()._type;

        let mut v = Vec::with_capacity(all_dependencies.len());
        v.extend(all_dependencies.into_iter());
        let all_dependencies = Arc::new(v);

        let context = Arc::new(context);
        let thread_count = threads.get();
        let mut threads = Vec::with_capacity(thread_count);

        for _ in 0..thread_count {
            let all_dependencies = all_dependencies.clone();
            let context = context.clone();

            threads.push(std::thread::spawn(move || {
                for path in all_dependencies.iter() {
                    context.verify_tag(&path, true);
                }
            }));
        }

        for t in threads {
            t.join().expect("thread should be finished successfully");
        }

        Ok(Arc::into_inner(context).unwrap().results.into_inner().unwrap())
    }

    fn reserve_tag_to_verify(&self, path: &TagPath) -> VerifyStatus {
        let mut results = self.results.lock().unwrap();
        match results.get(path) {
            Some(n) => {
                if n.is_ok() {
                    VerifyStatus::VerifiedOK
                }
                else {
                    VerifyStatus::VerifiedError
                }
            },
            None => {
                results.insert(path.to_owned(), Default::default());
                VerifyStatus::Unverified
            }
        }
    }

    /// Verify the tag
    ///
    /// If `skip_if_locked` is true, then this will return `None` if the tag is currently open in another thread.
    ///
    /// Otherwise, return `true` if the tag is OK, and `false` if not.
    fn verify_tag(&self, path: &TagPath, skip_if_locked: bool) -> Option<bool> {
        let mut result = VerifyResult::default();
        match self.tag_tree.open_tag_shared(&path) {
            Ok(tag) => {
                // We want to acquire the tag ASAP in case something tries to read this tag before we verify it
                let lock = if skip_if_locked {
                    tag.try_lock().ok()?
                }
                else {
                    tag.lock().unwrap()
                };

                match self.reserve_tag_to_verify(path) {
                    VerifyStatus::Unverified => (),
                    VerifyStatus::VerifiedError => return Some(false),
                    VerifyStatus::VerifiedOK => return Some(true)
                };

                let tag = lock.as_ref();
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
            },
            Err(e) => {
                if self.reserve_tag_to_verify(path) != VerifyStatus::Unverified {
                    return Some(false);
                }
                result.errors.push(format!("Failed to open tag: {e}"))
            }
        }

        let is_ok = result.is_ok();
        let path = path.to_owned();
        self.results.lock().unwrap().insert(path, result);

        Some(is_ok)
    }

    fn open_tag_reference_maybe(&self, tag_reference: &TagReference, result: &mut VerifyResult, must_be_set_error: Option<&'static str>) -> Option<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
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

    fn open_tag_maybe(&self, tag_path: &TagPath, result: &mut VerifyResult) -> Option<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        self.open_tag_unverified(tag_path, result).and_then(|n| {
            if self.verify_tag(tag_path, false).unwrap() {
                Some(n)
            }
            else {
                result.errors.push(format!("{tag_path} has errors and could not be opened"));
                None
            }
        })
    }

    fn open_tag_unverified(&self, tag_path: &TagPath, result: &mut VerifyResult) -> Option<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
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
