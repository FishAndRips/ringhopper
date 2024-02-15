use std::convert::TryInto;
use constants::{TICK_RATE, TICK_RATE_RECIPROCOL};
use definitions::{ActorVariant, Bitmap, BitmapData, BitmapDataFormat, ContinuousDamageEffect, DamageEffect, GBXModel, Light, Model, ModelAnimations, PointPhysics, Projectile, Scenario, ScenarioType, Weapon};
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::map::{DomainType, Map, ResourceMapType};
use primitives::primitive::{Angle, Bounds, TagPath};
use tag::model::ModelFunctions;
use tag::nudge::nudge_tag;

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

pub fn fix_extracted_actor_variant_tag(actor_variant: &mut ActorVariant) {
    actor_variant.grenades.grenade_velocity /= TICK_RATE_RECIPROCOL;
    nudge_tag(actor_variant);
}

pub fn fix_continuous_damage_effect_tag(continuous_damage_effect: &mut ContinuousDamageEffect) {
    continuous_damage_effect.camera_shaking.wobble_period /= TICK_RATE;
    nudge_tag(continuous_damage_effect);
}

pub fn fix_damage_effect_tag(damage_effect: &mut DamageEffect) {
    damage_effect.camera_shaking.wobble_period /= TICK_RATE;
    nudge_tag(damage_effect);
}

pub fn fix_point_physics_tag(point_physics: &mut PointPhysics) {
    point_physics.air_friction /= 10000.0;
    point_physics.water_friction /= 10000.0;
    nudge_tag(point_physics);
}

pub fn fix_projectile_tag(projectile: &mut Projectile) {
    projectile.minimum_velocity /= TICK_RATE_RECIPROCOL;
    projectile.initial_velocity /= TICK_RATE_RECIPROCOL;
    projectile.final_velocity /= TICK_RATE_RECIPROCOL;

    for i in &mut projectile.material_response.items {
        i.potential_and.upper /= TICK_RATE_RECIPROCOL;
        i.potential_and.lower /= TICK_RATE_RECIPROCOL;
    }

    nudge_tag(projectile);
}

pub fn fix_light_tag(light: &mut Light) {
    light.effect_parameters.duration /= TICK_RATE;
    nudge_tag(light);
}

pub fn fix_model_tag_normal<M: Map>(model: &mut Model, map: &M) -> RinghopperResult<()> {
    model.fix_runtime_markers()?;
    todo!("get vertices/triangles back into the model")
}

pub fn fix_gbxmodel_tag<M: Map>(gbxmodel: &mut GBXModel, map: &M) -> RinghopperResult<()> {
    gbxmodel.fix_runtime_markers()?;
    todo!("get vertices/triangles back into the model")
}

pub fn fix_scenario_tag(scenario: &mut Scenario) -> RinghopperResult<()> {
    Err(Error::TagGroupUnimplemented)
}

pub fn fix_model_animations_tag(model_animations: &mut ModelAnimations) -> RinghopperResult<()> {
    Err(Error::TagGroupUnimplemented)
}

fn fix_bitmap_tag<M: Map, F>(tag: &mut Bitmap, map: &M, mut compressed_data_handler: Option<F>) -> RinghopperResult<()>
    where F: FnMut(&mut BitmapData, &mut Vec<u8>) -> RinghopperResult<()> {

    let total_bitmap_data = tag
        .bitmap_data
        .items
        .iter()
        .map(|b| Ok(b.pixel_data_size as usize))
        .reduce(|a, b| {
            a.and_then(|size| b.and_then(|size2| size.add_overflow_checked(size2)))
        });
    let new_data = match total_bitmap_data {
        Some(n) => n?,
        None => return Ok(())
    };
    let mut processed_data: Vec<u8> = Vec::with_capacity(new_data);
    for i in &mut tag.bitmap_data.items {
        let offset = i.pixel_data_offset as usize;
        let length = i.pixel_data_size as usize;

        let bitmap_data = if i.flags.external {
            map.get_data_at_address(offset, &DomainType::ResourceMapFile(ResourceMapType::Bitmaps), length)
        }
        else {
            map.get_data_at_address(offset, &DomainType::MapData, length)
        }.ok_or_else(
            || Error::MapDataOutOfBounds(format!("Unable to extract bitmap data at offset 0x{offset:08X}"))
        )?;

        i.flags.external = false;
        i.pixel_data_offset = processed_data.len().try_into().map_err(|_| Error::SizeLimitExceeded)?;

        if i.flags.compressed ^ matches!(i.format, BitmapDataFormat::DXT1 | BitmapDataFormat::DXT3 | BitmapDataFormat::DXT5) {
            return Err(Error::InvalidTagData(format!("Bitmap is marked as compressed, but is formatted as {:?}", i.format)))
        }

        if let Some(callback) = &mut compressed_data_handler {
            if i.flags.compressed {
                let mut data = bitmap_data.to_vec();
                callback(i, &mut data)?;
                processed_data.append(&mut data);
            }
            else {
                processed_data.extend_from_slice(bitmap_data);
            }
        }
        else {
            processed_data.extend_from_slice(bitmap_data);
        }

    }

    Ok(())
}

pub fn fix_bitmap_tag_normal<M: Map>(tag: &mut Bitmap, map: &M) -> RinghopperResult<()> {
    fix_bitmap_tag(tag, map, None)
}
