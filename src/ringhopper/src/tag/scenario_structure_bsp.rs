use definitions::{ScenarioStructureBSP, ScenarioStructureBSPMaterial, ScenarioStructureBSPMaterialCompressedLightmapVertex, ScenarioStructureBSPMaterialCompressedRenderedVertex, ScenarioStructureBSPMaterialUncompressedLightmapVertex, ScenarioStructureBSPMaterialUncompressedRenderedVertex};
use primitives::byteorder::LittleEndian;
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::parse::{RawStructIteratorInfallible, SimpleTagData};
use primitives::primitive::Vector;

/// Detect if compressed or uncompressed vertices are missing and attempt to restore them.
pub fn recompress_scenario_structure_bsp_vertices(bsp: &mut ScenarioStructureBSP) -> RinghopperResult<bool> {
    let mut fixed = false;
    for lightmap in &mut bsp.lightmaps {
        for material in &mut lightmap.materials {
            let no_compressed_vertices = material.compressed_vertices.bytes.is_empty();
            let no_uncompressed_vertices = material.uncompressed_vertices.bytes.is_empty();

            if (no_compressed_vertices && no_uncompressed_vertices) || (!no_compressed_vertices && !no_uncompressed_vertices) {
                continue
            }

            fixed = true;

            macro_rules! transform_vertices {
                ($target_rendered:ty, $target_lightmap:ty, $get:tt, $convert_rendered:tt, $convert_lightmap:tt) => {{
                    let (rendered, lightmap) = $get(material)?;
                    let mut new_data = Vec::with_capacity(
                        rendered.len().mul_overflow_checked(<$target_rendered>::simple_size())?
                        + lightmap.len().mul_overflow_checked(<$target_lightmap>::simple_size())?
                    );
                    for r in rendered {
                        new_data.extend_from_slice($convert_rendered(r).as_bytes::<LittleEndian>().unwrap().bytes());
                    }
                    for l in lightmap {
                        new_data.extend_from_slice($convert_lightmap(l).as_bytes::<LittleEndian>().unwrap().bytes());
                    }
                    new_data
                }};
            }

            if no_compressed_vertices {
                material.compressed_vertices.bytes = transform_vertices!(
                    ScenarioStructureBSPMaterialCompressedRenderedVertex,
                    ScenarioStructureBSPMaterialCompressedLightmapVertex,
                    get_uncompressed_vertices_for_bsp_material,
                    compress_rendered_bsp_vertex,
                    compress_lightmap_bsp_vertex
                )
            }
            else if no_uncompressed_vertices {
                material.uncompressed_vertices.bytes = transform_vertices!(
                    ScenarioStructureBSPMaterialUncompressedRenderedVertex,
                    ScenarioStructureBSPMaterialUncompressedLightmapVertex,
                    get_compressed_vertices_for_bsp_material,
                    decompress_rendered_bsp_vertex,
                    decompress_lightmap_bsp_vertex
                )
            }
            else {
                unreachable!()
            }
        }
    }

    Ok(fixed)
}

macro_rules! convert_rendered_bsp_vertex {
    ($from:expr, $to:tt) => {{
        $to {
            position: $from.position,
            normal: $from.normal.into(),
            binormal: $from.binormal.into(),
            tangent: $from.tangent.into(),
            texture_coords: $from.texture_coords
        }
    }};
}

fn compress_rendered_bsp_vertex(vertex: ScenarioStructureBSPMaterialUncompressedRenderedVertex) -> ScenarioStructureBSPMaterialCompressedRenderedVertex {
    convert_rendered_bsp_vertex!(vertex, ScenarioStructureBSPMaterialCompressedRenderedVertex)
}

fn decompress_rendered_bsp_vertex(vertex: ScenarioStructureBSPMaterialCompressedRenderedVertex) -> ScenarioStructureBSPMaterialUncompressedRenderedVertex {
    let mut q = convert_rendered_bsp_vertex!(vertex, ScenarioStructureBSPMaterialUncompressedRenderedVertex);

    q.normal = q.normal.normalize();
    q.binormal = q.binormal.normalize();
    q.tangent = q.tangent.normalize();

    q
}

macro_rules! convert_lightmap_bsp_vertex {
    ($from:expr, $to:tt) => {{
        $to {
            normal: $from.normal.into(),
            texture_coords: $from.texture_coords.into()
        }
    }};
}

fn compress_lightmap_bsp_vertex(vertex: ScenarioStructureBSPMaterialUncompressedLightmapVertex) -> ScenarioStructureBSPMaterialCompressedLightmapVertex {
    convert_lightmap_bsp_vertex!(vertex, ScenarioStructureBSPMaterialCompressedLightmapVertex)
}

fn decompress_lightmap_bsp_vertex(vertex: ScenarioStructureBSPMaterialCompressedLightmapVertex) -> ScenarioStructureBSPMaterialUncompressedLightmapVertex {
    convert_lightmap_bsp_vertex!(vertex, ScenarioStructureBSPMaterialUncompressedLightmapVertex)
}

macro_rules! get_vertices_for_bsp_material {
    ($material:expr, $rendered:tt, $lightmap:tt, $name:expr, $field:tt) => {{
        let rendered_vertex_count = $material.rendered_vertices.vertex_count as usize;
        let lightmap_vertex_count = $material.lightmap_vertices.vertex_count as usize;

        if lightmap_vertex_count != 0 && lightmap_vertex_count != rendered_vertex_count {
            return Err(Error::InvalidTagData(format!("Invalid lightmap vertex count: expected {rendered_vertex_count} or 0, got {lightmap_vertex_count}")))
        }

        let rendered_vertex_size = <$rendered>::simple_size();
        let lightmap_vertex_size = <$lightmap>::simple_size();
        let rendered_vertices_size = rendered_vertex_size.mul_overflow_checked(rendered_vertex_count)?;
        let lightmap_vertices_size = lightmap_vertex_size.mul_overflow_checked(lightmap_vertex_count)?;

        let total_size = rendered_vertices_size.add_overflow_checked(lightmap_vertices_size)?;
        let expected_size = $material.$field.bytes.len();
        if expected_size != total_size {
            let name = $name;
            return Err(Error::InvalidTagData(format!("Invalid {name} vertices size: expected {expected_size}, got {total_size}")))
        }

        let (rendered_vertices, lightmap_vertices) = $material.$field.bytes.split_at(rendered_vertices_size);

        let rendered_vertices = <$rendered>::read_chunks_to_iterator::<LittleEndian>(rendered_vertices).unwrap().into_infallible();
        let lightmap_vertices = <$lightmap>::read_chunks_to_iterator::<LittleEndian>(lightmap_vertices).unwrap().into_infallible();

        (rendered_vertices, lightmap_vertices)
    }};
}

pub fn get_uncompressed_vertices_for_bsp_material(material: &ScenarioStructureBSPMaterial) -> RinghopperResult<(
    RawStructIteratorInfallible<ScenarioStructureBSPMaterialUncompressedRenderedVertex, LittleEndian>,
    RawStructIteratorInfallible<ScenarioStructureBSPMaterialUncompressedLightmapVertex, LittleEndian>
)> {
    Ok(get_vertices_for_bsp_material!(
        material,
        ScenarioStructureBSPMaterialUncompressedRenderedVertex,
        ScenarioStructureBSPMaterialUncompressedLightmapVertex,
        "uncompressed",
        uncompressed_vertices
    ))
}

pub fn get_compressed_vertices_for_bsp_material(material: &ScenarioStructureBSPMaterial) -> RinghopperResult<(
    RawStructIteratorInfallible<ScenarioStructureBSPMaterialCompressedRenderedVertex, LittleEndian>,
    RawStructIteratorInfallible<ScenarioStructureBSPMaterialCompressedLightmapVertex, LittleEndian>
)> {
    Ok(get_vertices_for_bsp_material!(
        material,
        ScenarioStructureBSPMaterialCompressedRenderedVertex,
        ScenarioStructureBSPMaterialCompressedLightmapVertex,
        "compressed",
        compressed_vertices
    ))
}
