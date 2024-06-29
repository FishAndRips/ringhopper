use primitives::{byteorder::LittleEndian, error::{OverflowCheck, RinghopperResult}, parse::SimpleTagData, primitive::{TagPath, Vector}, tag::PrimaryTagStructDyn};
use ringhopper_structs::*;

use crate::tag::tree::TagTree;

use super::{ScenarioContext, TagResult};

pub fn verify_scenario_structure_bsp<T: TagTree + Send + Sync + 'static>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, context: &ScenarioContext<T>, result: &mut TagResult) {
    let scenario_structure_bsp: &ScenarioStructureBSP = tag.as_any().downcast_ref().unwrap();

    // Check this before we can proceed.
    if !check_scenario_structure_bsp_vertex_data_size_correct(scenario_structure_bsp) {
        result.errors.push("BSP material(s) contain bad lightmap/render size(s). This tag needs remade.".to_owned());
        return;
    }

    let mut complained_bitfield: u32 = 0;

    for obj in &scenario_structure_bsp.detail_objects {
        'next_cell: for (cell_i, cell) in ziperator!(obj.cells) {
            let start = cell.count_index as usize;
            let count = cell.valid_layers_flags.count_ones() as usize;
            let Ok(end) = count.add_overflow_checked(start) else {
                result.errors.push(format!("BSP detail object cell #{cell_i} has an invalid count index and range (out-of-bounds)"));
                continue 'next_cell;
            };

            let reference_vector_range = start..end;
            let Some(counts) = obj.counts.items.get(reference_vector_range.clone()) else {
                result.errors.push(format!("BSP detail object cell #{cell_i} has an invalid count index and range (can't get counts {reference_vector_range:?})"));
                continue 'next_cell;
            };
            let Some(_) = obj.z_reference_vectors.items.get(reference_vector_range.clone()) else {
                result.errors.push(format!("BSP detail object cell #{cell_i} has an invalid count index and range (can't get z references {reference_vector_range:?})"));
                continue 'next_cell;
            };

            // Make sure all counts are valid
            let mut start_index = cell.start_index as usize;
            for c in counts {
                let count = c.count as usize;
                let Ok(end) = start_index.add_overflow_checked(count) else {
                    result.errors.push(format!("BSP detail object cell #{cell_i} has an invalid count (out-of-bounds)"));
                    continue 'next_cell;
                };
                let range = start_index..end;
                if let None = obj.instances.items.get(range.clone()) {
                    result.errors.push(format!("BSP detail object cell #{cell_i} has an invalid count (can't get instances {range:?})"));
                    continue 'next_cell;
                }
                start_index = end;
            }

            // Make sure all z reference vectors are valid
            for bit_offset in 0..32 {
                let bit = (cell.valid_layers_flags >> bit_offset as u32) & 1;
                if bit == 0 {
                    continue
                }
                let scenario = context.scenario.lock().unwrap();
                let scenario: &Scenario = scenario.as_any().downcast_ref().unwrap();

                // Check if it exists. If so, yay!
                let Some(_) = scenario
                    .detail_object_collection_palette
                    .items
                    .get(bit_offset)
                    .map(|r| &r.reference)
                    .and_then(|r| r.path())
                    .and_then(|r| context.tag_tree.open_tag_shared(r).ok()) else {
                    let bit = 1 << bit_offset;
                    if (complained_bitfield & bit) != 0 {
                        complained_bitfield |= bit;
                        result.errors.push(format!("Missing detail object collection palette #{bit_offset} in scenario tag"));
                    }
                    continue;
                };
            }
        }
    }

    let mut non_normal_vectors_found = false;
    'outer: for lm in &scenario_structure_bsp.lightmaps {
        for mat in &lm.materials {
            if mat.uncompressed_vertices.bytes.is_empty() {
                continue;
            }

            let rendered_count = mat.rendered_vertices.vertex_count as usize;
            let iterator = ScenarioStructureBSPMaterialUncompressedRenderedVertex::read_chunks_to_iterator::<LittleEndian>(&mat.uncompressed_vertices.bytes[..rendered_count * ScenarioStructureBSPMaterialUncompressedRenderedVertex::simple_size()]).unwrap().into_infallible();

            for vertex in iterator {
                if !vertex.normal.is_unit_vector() {
                    non_normal_vectors_found = true;
                    break 'outer;
                }
            }
        }
    }
    if non_normal_vectors_found {
        result.errors.push("Non-normal vectors detected! This can be automatically repaired with the bludgeon command.".to_owned());
    }
}

pub(crate) fn check_scenario_structure_bsp_vertex_data_size_correct(bsp: &ScenarioStructureBSP) -> bool {
    for lm in &bsp.lightmaps {
        for mat in &lm.materials {
            let rendered_count = mat.rendered_vertices.vertex_count as usize;
            let lightmap_count = mat.lightmap_vertices.vertex_count as usize;

            if lightmap_count != 0 && lightmap_count != rendered_count {
                return false;
            }

            let expected_size = |rendered_size: usize, lightmap_size: usize| -> RinghopperResult<usize> {
                rendered_count
                    .mul_overflow_checked(rendered_size)?
                    .add_overflow_checked(lightmap_count
                        .mul_overflow_checked(lightmap_size)?
                    )
            };

            if !mat.uncompressed_vertices.bytes.is_empty() {
                match expected_size(
                    ScenarioStructureBSPMaterialUncompressedRenderedVertex::simple_size(),
                    ScenarioStructureBSPMaterialUncompressedLightmapVertex::simple_size(),
                ) {
                    Ok(n) if n == mat.uncompressed_vertices.bytes.len() => (),
                    _ => return false
                };
            }

            if !mat.compressed_vertices.bytes.is_empty() {
                match expected_size(
                    ScenarioStructureBSPMaterialCompressedRenderedVertex::simple_size(),
                    ScenarioStructureBSPMaterialCompressedLightmapVertex::simple_size(),
                ) {
                    Ok(n) if n == mat.compressed_vertices.bytes.len() => (),
                    _ => return false
                };
            }
        }
    }
    true
}
