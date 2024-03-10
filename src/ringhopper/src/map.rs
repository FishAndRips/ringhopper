use std::borrow::Cow;
use std::ops::Range;

use definitions::{Bitmap, Model, read_any_tag_from_map, Scenario, ScenarioType};
use primitives::error::RinghopperResult;
use primitives::map::Map;
use primitives::primitive::{TagGroup, TagPath};
use primitives::tag::PrimaryTagStructDyn;

use crate::map::extract::*;
use crate::tag::object::downcast_base_object_mut;
use crate::tag::tree::{TagFilter, TagTree, TagTreeItem, TagTreeItemType};

mod extract;
pub mod resource;

pub mod header;
pub mod gearbox;
mod util;

type SizeRange = Range<usize>;

#[derive(Clone)]
struct BSPDomain {
    path: Option<TagPath>,
    range: SizeRange,
    base_address: usize,
    tag_address: usize
}

fn extract_tag_from_map<M: Map>(
    map: &M,
    path: &TagPath,
    scenario_tag: &Scenario,

    bitmap_extraction_fn: fn(tag: &mut Bitmap, map: &M) -> RinghopperResult<()>,
    model_extraction_fn: fn(tag: &mut Model, map: &M) -> RinghopperResult<()>,
) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
    let group = path.group();
    let mut tag = read_any_tag_from_map(path, map)?;

    match group {
        TagGroup::ActorVariant => fix_actor_variant_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::Bitmap => bitmap_extraction_fn(tag.as_any_mut().downcast_mut().unwrap(), map)?,
        TagGroup::ContinuousDamageEffect => fix_continuous_damage_effect_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::DamageEffect => fix_damage_effect_tag(tag.as_any_mut().downcast_mut().unwrap(), path, &scenario_tag),
        TagGroup::GBXModel => fix_gbxmodel_tag(tag.as_any_mut().downcast_mut().unwrap(), map)?,
        TagGroup::Light => fix_light_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::ModelAnimations => fix_model_animations_tag(tag.as_any_mut().downcast_mut().unwrap())?,
        TagGroup::Model => { model_extraction_fn(tag.as_any_mut().downcast_mut().unwrap(), map)?; },
        TagGroup::PointPhysics => fix_point_physics_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::Projectile => fix_projectile_tag(tag.as_any_mut().downcast_mut().unwrap()),
        TagGroup::Scenario => fix_scenario_tag(tag.as_any_mut().downcast_mut().unwrap(), path.base_name())?,
        TagGroup::ScenarioStructureBSP => fix_scenario_structure_bsp_tag(tag.as_any_mut().downcast_mut().unwrap())?,
        TagGroup::Sound => fix_sound_tag(tag.as_any_mut().downcast_mut().unwrap())?,
        TagGroup::Weapon => fix_weapon_tag(tag.as_any_mut().downcast_mut().unwrap(), path, &scenario_tag),
        _ => ()
    };

    if let Some(n) = downcast_base_object_mut(tag.as_mut()) {
        fix_object_tag(n)?;
    }

    Ok(tag)
}

pub trait MapTagTree: Map {
    /// Get the scenario type for the map.
    fn get_scenario_type(&self) -> ScenarioType;
}
impl<M: MapTagTree> TagTree for M {
    fn open_tag_copy(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        self.extract_tag(path)
    }

    fn files_in_path(&self, _path: &str) -> Option<Vec<TagTreeItem>> {
        todo!("files_in_path not yet implemented for MapTagTree")
    }

    fn write_tag(&mut self, _path: &TagPath, _tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<bool> {
        unimplemented!("write_tag not implemented for MapTagTree")
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn contains(&self, path: &TagPath) -> bool {
        self.get_tag(path).is_some()
    }

    fn root(&self) -> TagTreeItem {
        TagTreeItem::new(TagTreeItemType::Directory, Cow::default(), None, self)
    }

    fn get_all_tags_with_filter(&self, filter: Option<&TagFilter>) -> Vec<TagPath> {
        let all_tags = self.get_all_tags();
        if let Some(n) = filter {
            all_tags.into_iter().filter(|t| n.passes(t)).collect()
        }
        else {
            all_tags
        }
    }
}
