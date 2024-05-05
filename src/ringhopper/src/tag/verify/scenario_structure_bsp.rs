use primitives::{byteorder::LittleEndian, error::{OverflowCheck, RinghopperResult}, parse::SimpleTagData, primitive::{TagPath, Vector}, tag::PrimaryTagStructDyn};
use ringhopper_structs::*;

use crate::tag::tree::TagTree;

use super::{VerifyContext, VerifyResult};

pub fn verify_scenario_structure_bsp<T: TagTree + Send + Sync + 'static>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &VerifyContext<T>, result: &mut VerifyResult) {
    let scenario_structure_bsp: &ScenarioStructureBSP = tag.as_any().downcast_ref().unwrap();

    // Check this before we can proceed.
    if !check_scenario_structure_bsp_vertex_data_size_correct(scenario_structure_bsp) {
        result.errors.push("BSP material(s) contain bad lightmap/render size(s). This tag needs remade.".to_owned());
        return;
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
