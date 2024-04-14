use std::borrow::Cow;
use std::num::NonZeroUsize;
use definitions::{BitmapType, ModelTriangleStripData, ScenarioStructureBSP};
use primitives::parse::TagData;
use primitives::primitive::{calculate_padding_for_alignment, ColorARGBInt, TagGroup};
use crate::tag::model::ModelPartGet;
use crate::constants::{TICK_RATE, TICK_RATE_RECIPROCOL};
use crate::definitions::*;
use crate::primitives::byteorder::{BigEndian, LittleEndian};
use crate::primitives::dynamic::DynamicTagDataArray;
use crate::primitives::error::{Error, OverflowCheck, RinghopperResult};
use crate::primitives::map::{DomainType, Map, ResourceMapType};
use crate::primitives::parse::{SimpleTagData};
use crate::primitives::primitive::{Angle, Bounds, Index, TagPath};
use crate::tag::bitmap::{bytes_per_block, COMPRESSED_BITMAP_DATA_FORMATS, MipmapFaceIterator, MipmapMetadata, MipmapTextureIterator, MipmapType, pixels_per_block_length, Swizzlable, swizzle};
use crate::tag::model::ModelFunctions;
use crate::tag::model_animations::{flip_endianness_for_model_animations_animation, FrameDataIterator};
use crate::tag::nudge::nudge_tag;
use crate::tag::scenario::{decompile_scripts, flip_scenario_script_endianness};
use crate::tag::scenario_structure_bsp::recompress_scenario_structure_bsp_vertices;

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

pub fn fix_damage_effect_tag(damage_effect: &mut DamageEffect, tag_path: &TagPath, scenario_tag: &Scenario) {
    damage_effect.camera_shaking.wobble_period /= TICK_RATE;
    nudge_tag(damage_effect);

    if scenario_tag._type == ScenarioType::Singleplayer
        && !scenario_tag.flags.do_not_apply_bungie_campaign_tag_patches
        && tag_path.path() == "weapons\\pistol\\bullet" {
        damage_effect.damage.modifiers.elite_energy_shield = 1.0;
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

pub fn fix_point_physics_tag(point_physics: &mut PointPhysics) {
    point_physics.air_friction /= 10000.0;
    point_physics.water_friction /= 10000.0;
    nudge_tag(point_physics);
}

pub fn fix_projectile_tag(projectile: &mut Projectile) {
    projectile.minimum_velocity /= TICK_RATE_RECIPROCOL;
    projectile.initial_velocity /= TICK_RATE_RECIPROCOL;
    projectile.final_velocity /= TICK_RATE_RECIPROCOL;

    for i in &mut projectile.material_response {
        i.potential_and.upper /= TICK_RATE_RECIPROCOL;
        i.potential_and.lower /= TICK_RATE_RECIPROCOL;
    }

    nudge_tag(projectile);
}

pub fn fix_light_tag(light: &mut Light) {
    light.effect_parameters.duration /= TICK_RATE;
    nudge_tag(light);
}

macro_rules! extract_vertices {
    ($model:expr, $map:expr) => {{
        for geo in &mut $model.geometries {
            for part in &mut geo.parts {
                let part = part.get_model_part_mut();

                let vertex_count = part.vertices.vertex_count as usize;
                let triangle_count = part.triangle_count as usize;

                if vertex_count > 0xFFFF {
                    return Err(Error::InvalidTagData(format!("Model data is invalid: vertex count is too high (0x{vertex_count:X} > 0xFFFF)")))
                }
                let max_triangles = 0xFFFE*3;
                if triangle_count > max_triangles {
                    return Err(Error::InvalidTagData(format!("Model data is invalid: triangle count is too high (0x{triangle_count:X} > 0x{max_triangles:X})")))
                }

                if $map.get_engine().compressed_models {
                    // Load all vertices
                    part.compressed_vertices.items.reserve_exact(vertex_count);
                    let vertex_pointer = CacheFileModelDataPointer::read_from_map($map, part.vertices.vertex_pointer.into(), &DomainType::TagData)?;
                    part.compressed_vertices.items.extend(ModelVertexCompressed::read_chunks_from_map_to_iterator(
                        $map,
                        vertex_count,
                        vertex_pointer.data.into(),
                        &DomainType::TagData
                    )?.into_infallible());

                    // Now all indices
                    part.triangle_data.items = load_all_indices($map, triangle_count, part.triangle_pointer.into(), &DomainType::TagData)?;
                }
                else {
                    // Load all vertices
                    part.uncompressed_vertices.items.reserve_exact(vertex_count);
                    part.uncompressed_vertices.items.extend(ModelVertexUncompressed::read_chunks_from_map_to_iterator(
                        $map,
                        vertex_count,
                        part.vertices.vertex_pointer.into(),
                        &DomainType::ModelVertexData
                    )?.into_infallible());

                    // Now all indices
                    part.triangle_data.items = load_all_indices($map, triangle_count, part.triangle_pointer.into(), &DomainType::ModelTriangleData)?;
                }
            }
        }
        if $map.get_engine().compressed_models {
            $model.fix_uncompressed_vertices();
        }
        else {
            $model.fix_compressed_vertices();
        }
    }};
}

fn load_all_indices<M: Map>(map: &M, triangle_count: usize, address: usize, domain: &DomainType) -> RinghopperResult<Vec<ModelTriangleStripData>> {
    // Now all indices
    let index_count = if triangle_count > 0 { triangle_count + 2 } else { 0 };
    let mut indices = Index::read_chunks_from_map_to_iterator(
        map,
        index_count,
        address,
        domain
    )?.into_infallible();
    let iterator = (0..indices.len())
        .step_by(3)
        .map(|_| ModelTriangleStripData {
            indices: [
                indices.next().expect("should be a triangle index here..."),
                indices.next().unwrap_or(None),
                indices.next().unwrap_or(None),
            ]
        });
    Ok(iterator.collect())
}

macro_rules! fix_model {
    ($model:expr, $map:expr) => {{
        $model.flags.blend_shared_normals = false;
        $model.flip_lod_cutoffs();
        $model.fix_runtime_markers()?;
        extract_vertices!($model, $map);
        Ok(())
    }}
}

pub fn fix_model_tag<M: Map>(model: &mut Model, map: &M) -> RinghopperResult<()> {
    fix_model!(model, map)
}

pub fn fix_gbxmodel_tag<M: Map>(gbxmodel: &mut GBXModel, map: &M) -> RinghopperResult<()> {
    fix_model!(gbxmodel, map)
}

pub fn fix_scenario_tag(scenario: &mut Scenario, scenario_name: &str) -> RinghopperResult<()> {
    flip_scenario_script_endianness::<LittleEndian, BigEndian>(scenario)?;
    decompile_scripts(scenario, scenario_name)?;

    for i in &mut scenario.cutscene_titles {
        i.up_time -= i.fade_in_time;
        i.fade_in_time *= TICK_RATE_RECIPROCOL;
        i.fade_out_time *= TICK_RATE_RECIPROCOL;
        i.up_time *= TICK_RATE_RECIPROCOL;
    }

    nudge_tag(scenario);

    Ok(())
}

pub fn fix_scenario_structure_bsp_tag(bsp: &mut ScenarioStructureBSP) -> RinghopperResult<()> {
    recompress_scenario_structure_bsp_vertices(bsp).map(|_| ())
}
pub fn fix_object_tag(object: &mut Object) -> RinghopperResult<()> {
    for cc in &mut object.change_colors {
        match cc.permutations.len() {
            0 => (),
            1 => cc.permutations.items[0].weight = 1.0,

            // Weights aren't actually weights in a cache file but partial weights from 0.0 - 1.0
            permutation_count => {
                // Check that the weights are valid
                let mut last_weight = 0.0;
                for i in &cc.permutations {
                    if i.weight < last_weight || i.weight > 1.0 {
                        return Err(Error::InvalidTagData("change colors has invalid weights".to_owned()))
                    }
                    last_weight = i.weight;
                }

                // Apply the weights
                for i in (1..permutation_count).rev() {
                    let last = cc.permutations.items[i - 1].weight;
                    let this = &mut cc.permutations.items[i].weight;
                    *this = *this - last;
                }
            }
        }
    }

    nudge_tag(object);

    Ok(())
}

pub fn fix_model_animations_tag(model_animations: &mut ModelAnimations) -> RinghopperResult<()> {
    for i in &mut model_animations.animations {
        fix_model_animations_animation(i)?;
    }
    Ok(())
}

fn fix_model_animations_animation(animation: &mut ModelAnimationsAnimation) -> RinghopperResult<()> {
    if animation.flags.compressed_data {
        // TODO: decompress animations
        let offset = FrameDataIterator::for_animation(animation).to_size().mul_overflow_checked(animation.frame_count as usize)?;
        let expected_final_data = offset.add_overflow_checked(animation.frame_data.bytes.len())?;
        if expected_final_data > u32::MAX as usize {
            return Err(Error::ArrayLimitExceeded)
        }

        let mut new_frame_data = Vec::with_capacity(offset.add_overflow_checked(animation.frame_data.bytes.len())?);
        new_frame_data.resize(offset, 0);
        new_frame_data.extend_from_slice(&animation.frame_data.bytes);
        animation.frame_data.bytes = new_frame_data;
        animation.offset_to_compressed_data = offset as u32; // we are required to do this or tool.exe will error
        animation.default_data.bytes = vec![0u8; FrameDataIterator::for_animation_inverted(animation).to_size()]; // compressed animations have no default data for some reason
    }

    flip_endianness_for_model_animations_animation::<LittleEndian, BigEndian>(animation)
}

pub fn fix_unicode_string_list_tag<M: Map>(_tag: &mut UnicodeStringList, map: &M) -> RinghopperResult<()> {
    // TODO: Figure out how to decode these
    for i in map.get_all_tags().iter().filter(|c| c.group() == TagGroup::Font) {
        let tag = map.extract_tag(&i)?;
        let tag: &Font = &tag.as_any().downcast_ref().unwrap();
        if tag.encoding_type == FontEncodingType::Extended {
            return Err(Error::Other("Extended unicode_string_list tags are unsupported".to_string()))
        }
    }

    Ok(())
}

pub fn fix_bitmap_tag<M: Map>(tag: &mut Bitmap, map: &M) -> RinghopperResult<()> {
    // Fix bitmap indices for sprites; for some horrible reason, these are zeroed out when put in maps
    if tag._type == BitmapType::Sprites {
        for sequence in &mut tag.bitmap_group_sequence {
            sequence.bitmap_count = if sequence.sprites.items.len() == 1 { 1 } else { 0 };

            let mut lowest_index = None;
            for sprite in &mut sequence.sprites {
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

    let engine = map.get_engine();
    let engine_name = engine.name;
    let must_modulo_block_size = engine.bitmap_options.texture_dimension_must_modulo_block_size;

    for i in &mut tag.bitmap_data {
        let is_compressed_format = COMPRESSED_BITMAP_DATA_FORMATS.contains(&i.format);

        // All of these are non-zero.
        let block_size = pixels_per_block_length(i.format);
        let bytes_per_block = bytes_per_block(i.format);

        // Do some basic checks here.
        if i.flags.compressed != is_compressed_format {
            return Err(Error::InvalidTagData(format!("Compressed flag is {} when it should be {is_compressed_format}.", i.flags.compressed)));
        }
        if i.flags.swizzled {
            if !engine.bitmap_options.swizzled {
                return Err(Error::InvalidTagData(format!("Bitmap is marked as swizzled, but this is not allowed for `{engine_name}` maps.")));
            }
            if i.flags.compressed {
                return Err(Error::InvalidTagData("Bitmap is marked as swizzled and compressed which is not allowed.".to_string()));
            }
        }

        let width = i.width as usize;
        let height = i.height as usize;
        let depth = i.depth as usize;

        if width == 0 || height == 0 || depth == 0 {
            return Err(Error::InvalidTagData(format!("Bitmap is {width}x{height}x{depth} which has 0 on one dimension.")));
        }

        let width = unsafe { NonZeroUsize::new_unchecked(width) };
        let height = unsafe { NonZeroUsize::new_unchecked(height) };
        let depth = unsafe { NonZeroUsize::new_unchecked(depth) };

        let mipmap_format = MipmapType::get_mipmap_type(&i)?;

        let alignment = engine.bitmap_options.alignment;
        if depth.get() != 1 && i._type != BitmapDataType::_3dTexture {
            return Err(Error::InvalidTagData(format!("Bitmap is has a depth of {depth}, but it is not a 3D texture.")));
        }
        if !depth.is_power_of_two() {
            return Err(Error::InvalidTagData(format!("Bitmap is has a depth of {depth} which is non-power-of-two.")));
        }
        if must_modulo_block_size && (width.get() % block_size != 0 || height.get() % block_size != 0) {
            return Err(Error::InvalidTagData(format!("Bitmap is {width}x{height} which is not divisible by {block_size}, which is required for {engine_name}.")));
        }

        let offset = i.pixel_data_offset as usize;
        let length = i.pixel_data_size as usize;
        let domain = if i.flags.external { DomainType::ResourceMapFile(ResourceMapType::Bitmaps) } else { DomainType::MapData };

        let bitmap_data = map.get_data_at_address(offset, &domain, length)
            .ok_or_else(|| Error::MapDataOutOfBounds(format!("Unable to extract bitmap data at offset 0x{offset:08X} from {domain:?}")))?;

        if length % alignment != 0 {
            return Err(Error::InvalidTagData(format!("Bitmap is {length} bytes, which is not divisible by {alignment} which is required for {engine_name}.")));
        }

        // Get and fix actual mipmap count stored
        let reported_mipmap_count = i.mipmap_count as usize;
        let physical_mipmap_count = if must_modulo_block_size {
            let actual_mipmap_count = MipmapTextureIterator::new(width, height, mipmap_format, block_size, Some(reported_mipmap_count))
                .take_while(|f| f.width % block_size.get() == 0 && f.height % block_size.get() == 0)
                .count() - 1;
            i.mipmap_count = actual_mipmap_count as u16;
            actual_mipmap_count
        }
        else {
            reported_mipmap_count
        };

        // Handle cubemaps
        let mut cow;
        if engine.bitmap_options.cubemap_faces_stored_separately && i._type == BitmapDataType::CubeMap {
            let bitmap_length = MipmapFaceIterator::new(width, height, MipmapType::TwoDimensional, block_size, Some(physical_mipmap_count))
                .map(|m| m.block_count * bytes_per_block.get())
                .reduce(|i, j| i + j)
                .unwrap();

            let bitmap_length_padded = bitmap_length + calculate_padding_for_alignment(bitmap_length, alignment);
            let input_bitmap_length = bitmap_length_padded * 6;
            if length != input_bitmap_length {
                return Err(Error::InvalidTagData(format!("Bitmap is {length} bytes, expected it to be {input_bitmap_length} ({bitmap_length_padded}*6) bytes...")));
            }

            let mut all = Vec::with_capacity(bitmap_length * 6);

            for mipmap in MipmapFaceIterator::new(width, height, MipmapType::TwoDimensional, block_size, Some(physical_mipmap_count)) {
                for i in [0, 2, 1, 3, 4, 5] {
                    let start = i*bitmap_length_padded + mipmap.block_offset * bytes_per_block.get();
                    let end = start + mipmap.block_count * bytes_per_block.get();
                    let range = start .. end;
                    all.extend_from_slice(&bitmap_data[range]);
                }
            }

            cow = Cow::Owned(all);
        }
        else {
            let actual_data = MipmapTextureIterator::new(width, height, mipmap_format, block_size, Some(physical_mipmap_count))
                .map(|m| m.block_count * bytes_per_block.get())
                .reduce(|i, j| i + j)
                .unwrap();

            let padding = calculate_padding_for_alignment(actual_data, alignment);
            let expected_length = actual_data + padding;

            // Mipmaps where a length is not divisible by block size are not included here.
            //
            // For Xbox maps, there appears to be garbage/null bytes after `expected_length`, thus it cannot be trusted.
            if must_modulo_block_size {
                if bitmap_data.len() < expected_length {
                    return Err(Error::InvalidTagData(format!("Bitmap is {length} bytes, expected it to be at most {expected_length} ({actual_data} + {padding}) bytes...")));
                }
            }
            else {
                if bitmap_data.len() != expected_length {
                    return Err(Error::InvalidTagData(format!("Bitmap is {length} bytes, expected it to be at exactly {expected_length} ({actual_data} + {padding}) bytes...")));
                }
            }

            cow = Cow::Borrowed(&bitmap_data[..actual_data])
        }

        if i.flags.swizzled {
            let mut data = Vec::with_capacity(cow.len());

            let mut do_thing_to_mipmap_metadata = |i: MipmapMetadata| -> RinghopperResult<()> {
                let start = i.block_offset * bytes_per_block.get();
                let size = i.block_count.mul_overflow_checked(bytes_per_block.get())?;
                let end = start.add_overflow_checked(size)?;

                let input = &cow[start..end];

                fn handle_things_from_here<T: SimpleTagData + Swizzlable>(metadata: MipmapMetadata, input: &[u8], output: &mut Vec<u8>) -> RinghopperResult<()> {
                    let mut data: Vec<T> = Vec::with_capacity(metadata.block_count);
                    data.extend(T::read_chunks_to_iterator::<LittleEndian>(input).unwrap().into_infallible());

                    let mut deswizzled: Vec<T> = vec![Default::default(); metadata.block_count];
                    swizzle(&data, &mut deswizzled, metadata.width, metadata.height, metadata.depth, true)?;

                    for i in deswizzled {
                        output.extend_from_slice(i.as_bytes::<LittleEndian>().unwrap().bytes());
                    }

                    Ok(())
                }

                match bytes_per_block.get() {
                    1 => handle_things_from_here::<u8>(i, input, &mut data)?,
                    2 => handle_things_from_here::<u16>(i, input, &mut data)?,
                    4 => handle_things_from_here::<ColorARGBInt>(i, input, &mut data)?,
                    n => unreachable!("cannot deswizzle {n} len", n=n)
                }

                Ok(())
            };

            if matches!(mipmap_format, MipmapType::Cubemap) {
                for i in MipmapFaceIterator::new(width, height, mipmap_format, block_size, Some(reported_mipmap_count)) {
                    do_thing_to_mipmap_metadata(i)?;
                }
            }
            else {
                for i in MipmapTextureIterator::new(width, height, mipmap_format, block_size, Some(reported_mipmap_count)) {
                    do_thing_to_mipmap_metadata(i)?;
                }
            }

            debug_assert_eq!(cow.len(), data.len());

            cow = Cow::Owned(data);
        }

        i.pixel_data_offset = processed_data.len() as u32;
        processed_data.extend_from_slice(&cow);
    }
    tag.processed_pixel_data.bytes = processed_data;

    Ok(())
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

    tag.maximum_bend_rate = tag.maximum_bend_rate.powf(TICK_RATE);
    nudge_tag(tag);

    Ok(())
}
