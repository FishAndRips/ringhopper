use std::convert::TryInto;
use definitions::{BitmapType, ModelTriangleStripData};
use crate::tag::model::ModelPartGet;
use crate::constants::{TICK_RATE, TICK_RATE_RECIPROCOL};
use crate::definitions::{ActorVariant, Bitmap, BitmapDataFormat, ContinuousDamageEffect, DamageEffect, GBXModel, Light, Model, ModelAnimations, ModelAnimationsAnimation, ModelVertexUncompressed, Object, PointPhysics, Projectile, Scenario, ScenarioType, Sound, SoundFormat, Weapon};
use crate::primitives::byteorder::{BigEndian, LittleEndian};
use crate::primitives::dynamic::DynamicTagDataArray;
use crate::primitives::error::{Error, OverflowCheck, RinghopperResult};
use crate::primitives::map::{DomainType, Map, ResourceMapType};
use crate::primitives::parse::{TagData, SimpleTagData};
use crate::primitives::primitive::{Angle, Bounds, Index, TagPath};
use crate::tag::model::ModelFunctions;
use crate::tag::model_animations::{flip_endianness_for_model_animations_animation, FrameDataIterator};
use crate::tag::nudge::nudge_tag;
use crate::tag::scenario::{decompile_scripts, flip_scenario_script_endianness};

pub fn fix_weapon_tag(tag: &mut Weapon, tag_path: &TagPath, scenario_tag: &Scenario) {
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

pub fn fix_actor_variant_tag(actor_variant: &mut ActorVariant) {
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

macro_rules! extract_uncompressed_model_vertices {
    ($model:expr, $map:expr) => {{
        for geo in &mut $model.geometries.items {
            for part in &mut geo.parts.items {
                let part = part.get_model_part_mut();

                let vertex_count = part.vertex_count as usize;
                let triangle_count = part.triangle_count as usize;

                if vertex_count > 0xFFFF {
                    return Err(Error::InvalidTagData(format!("Model data is invalid: vertex count is too high (0x{vertex_count:X} > 0xFFFF)")))
                }
                let max_triangles = 0xFFFE*3;
                if triangle_count > max_triangles {
                    return Err(Error::InvalidTagData(format!("Model data is invalid: triangle count is too high (0x{triangle_count:X} > 0x{max_triangles:X})")))
                }

                let index_count = triangle_count + 2;
                part.uncompressed_vertices.items.reserve_exact(vertex_count);

                let vertex_size = ModelVertexUncompressed::size();
                let vertex_offset = part.vertex_pointer.into();
                let vertex_end = vertex_offset + (vertex_size * vertex_count);
                for v in (vertex_offset..vertex_end).step_by(vertex_size) {
                    let vertex = ModelVertexUncompressed::read_from_map($map, v, &DomainType::ModelVertexData)?;
                    part.uncompressed_vertices.items.push(vertex);
                }

                let mut indices: Vec<Index> = Vec::with_capacity(index_count + 2);
                let triangle_size = Index::simple_size();
                let triangle_offset = part.triangle_pointer.into();
                let triangle_end = triangle_offset + (index_count * triangle_size);
                for t in (triangle_offset..triangle_end).step_by(triangle_size) {
                    let index = Index::read_from_map($map, t, &DomainType::ModelTriangleData)?;
                    indices.push(index);
                }

                // Pad out the triangles
                indices.push(None);
                indices.push(None);

                let iterator = (0..indices.len())
                    .step_by(3)
                    .map(|index| ModelTriangleStripData {
                        indices: unsafe {[
                            // Fine because we checked above
                            *indices.get_unchecked(index),
                            *indices.get_unchecked(index + 1),
                            *indices.get_unchecked(index + 2)
                        ]}
                    });

                part.triangle_data.items.extend(iterator);
            }
        }
        $model.fix_compressed_vertices();
    }};
}

macro_rules! fix_uncompressed_model {
    ($model:expr, $map:expr) => {{
        $model.flip_lod_cutoffs();
        $model.fix_runtime_markers()?;
        extract_uncompressed_model_vertices!($model, $map);
        Ok(())
    }}
}

pub fn fix_model_tag_uncompressed<M: Map>(model: &mut Model, map: &M) -> RinghopperResult<()> {
    fix_uncompressed_model!(model, map)
}

pub fn fix_gbxmodel_tag<M: Map>(gbxmodel: &mut GBXModel, map: &M) -> RinghopperResult<()> {
    fix_uncompressed_model!(gbxmodel, map)
}

pub fn fix_scenario_tag(scenario: &mut Scenario, scenario_name: &str) -> RinghopperResult<()> {
    flip_scenario_script_endianness::<LittleEndian, BigEndian>(scenario)?;
    decompile_scripts(scenario, scenario_name)?;
    Ok(())
}

pub fn fix_object_tag(object: &mut Object) -> RinghopperResult<()> {
    for cc in &mut object.change_colors.items {
        match cc.permutations.len() {
            0 => (),
            1 => cc.permutations.items[0].weight = 1.0,

            // Weights aren't actually weights in a cache file but partial weights from 0.0 - 1.0
            permutation_count => {
                // Check that the weights are valid
                let mut last_weight = 0.0;
                for i in &cc.permutations.items {
                    if i.weight < last_weight || i.weight > 1.0 {
                        return Err(Error::InvalidTagData("change colors has invalid weights".to_owned()))
                    }
                    last_weight = i.weight;
                }

                // Apply the weights
                for i in 1..permutation_count {
                    let last = cc.permutations.items[i - 1].weight;
                    let this = &mut cc.permutations.items[i].weight;
                    *this = *this - last;
                }
            }
        }
    }

    Ok(())
}

pub fn fix_model_animations_tag(model_animations: &mut ModelAnimations) -> RinghopperResult<()> {
    for i in &mut model_animations.animations.items {
        fix_model_animations_animation(i)?;
    }
    Ok(())
}

fn fix_model_animations_animation(animation: &mut ModelAnimationsAnimation) -> RinghopperResult<()> {
    if animation.flags.compressed_data {
        // TODO: decompress animations
        let offset = FrameDataIterator::for_animation(animation).to_size();
        let expected_final_data = offset.add_overflow_checked(animation.frame_data.bytes.len())?;
        if expected_final_data > u32::MAX as usize {
            return Err(Error::ArrayLimitExceeded)
        }

        let mut new_frame_data = Vec::with_capacity(offset.add_overflow_checked(animation.frame_data.bytes.len())?);
        new_frame_data.resize(offset, 0);
        new_frame_data.extend_from_slice(&animation.frame_data.bytes);
        animation.frame_data.bytes = new_frame_data;
        animation.offset_to_compressed_data = offset as u32;

        animation.default_data.bytes = vec![0; FrameDataIterator::for_animation_inverted(animation).to_size()];
    }

    flip_endianness_for_model_animations_animation::<LittleEndian, BigEndian>(animation)
}

fn fix_bitmap_tag<M: Map>(tag: &mut Bitmap, map: &M, xbox_map: bool) -> RinghopperResult<()> {
    // Fix bitmap indices for sprites; for some horrible reason, these are zeroed out when put in maps
    if tag._type == BitmapType::Sprites {
        for sequence in &mut tag.bitmap_group_sequence.items {
            sequence.bitmap_count = if sequence.sprites.items.len() == 1 { 1 } else { 0 };

            let mut lowest_index = None;
            for sprite in &mut sequence.sprites.items {
                let this_index = if let Some(n) = sprite.bitmap_index { n } else { continue };
                match lowest_index {
                    Some(n) if n < this_index => (),
                    _ => lowest_index = Some(this_index)
                }
            }

            sequence.first_bitmap_index = lowest_index;
        }
    }

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
        let domain = if i.flags.external { DomainType::ResourceMapFile(ResourceMapType::Bitmaps) } else { DomainType::MapData };

        let bitmap_data = map.get_data_at_address(offset, &domain, length)
            .ok_or_else(|| Error::MapDataOutOfBounds(format!("Unable to extract bitmap data at offset 0x{offset:08X} from {domain:?}")))?;

        i.pixel_data_offset = processed_data.len().try_into().map_err(|_| Error::SizeLimitExceeded)?;

        let expected_compressed_flag = matches!(i.format, BitmapDataFormat::DXT1 | BitmapDataFormat::DXT3 | BitmapDataFormat::DXT5 | BitmapDataFormat::BC7);
        if i.flags.compressed ^ expected_compressed_flag {
            return Err(Error::InvalidTagData(format!("Bitmap is formatted as {:?}, but the compressed flag is wrong (should be {expected_compressed_flag}).", i.format)))
        }

        if xbox_map {
            todo!("handle xbox")
        }
        else {
            processed_data.extend_from_slice(bitmap_data);
        }
    }
    tag.processed_pixel_data.bytes = processed_data;

    Ok(())
}

pub fn fix_bitmap_tag_normal<M: Map>(tag: &mut Bitmap, map: &M) -> RinghopperResult<()> {
    fix_bitmap_tag(tag, map, false)
}

pub fn fix_sound_tag(tag: &mut Sound) -> RinghopperResult<()> {
    let permutations = tag.pitch_ranges.items.iter_mut().flat_map(|p| p.permutations.items.iter_mut());
    for permutation in permutations {
        if permutation.format != SoundFormat::PCM {
            continue;
        }
        if permutation.samples.bytes.len() % 2 == 1 {
            return Err(Error::InvalidTagData("Sound data is 16-bit PCM, but one or more permutations have an odd number of bytes".to_owned()));
        }

        // swap endian of 16-bit PCM
        let swapped: Vec<u8> = permutation.samples.bytes.chunks(2).map(|b| [b[1], b[0]]).flatten().collect();
        permutation.samples.bytes = swapped;
    }

    Ok(())
}
