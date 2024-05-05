use primitives::{byteorder::LittleEndian, parse::SimpleTagData, primitive::Vector, tag::PrimaryTagStructDyn};
use ringhopper_structs::{ScenarioStructureBSP, ScenarioStructureBSPMaterialUncompressedRenderedVertex};
use crate::tag::verify::scenario_structure_bsp::check_scenario_structure_bsp_vertex_data_size_correct;

use super::BludgeonResult;

pub fn repair_scenario_structure_bsp(tag: &mut dyn PrimaryTagStructDyn) -> BludgeonResult {
    let bsp: &mut ScenarioStructureBSP = tag.as_any_mut().downcast_mut().unwrap();

    // Check that shit is not fucked before attempting to repair normals.
    if !check_scenario_structure_bsp_vertex_data_size_correct(bsp) {
        return BludgeonResult::CannotRepair
    }

    // Shit is not fucked. Continue.
    for lm in &mut bsp.lightmaps {
        for mat in &mut lm.materials {
            if mat.uncompressed_vertices.bytes.is_empty() {
                continue
            }

            let rendered_count = mat.rendered_vertices.vertex_count as usize;
            let (rendered, lightmap) = mat.uncompressed_vertices.bytes.split_at(rendered_count * ScenarioStructureBSPMaterialUncompressedRenderedVertex::simple_size());
            let iterator = ScenarioStructureBSPMaterialUncompressedRenderedVertex::read_chunks_to_iterator::<LittleEndian>(rendered)
                .unwrap()
                .into_infallible()
                .map(|mut v| {
                    v.normal = v.normal.normalize();
                    v.as_bytes::<LittleEndian>().unwrap()
                });

            let mut all_data = Vec::new();
            for i in iterator {
                all_data.extend_from_slice(i.bytes());
            }
            all_data.extend_from_slice(lightmap);
            mat.uncompressed_vertices.bytes = all_data;
        }
    }

    BludgeonResult::Done
}
