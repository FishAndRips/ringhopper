use definitions::{GBXModel, GBXModelFlags, GBXModelGeometry, GBXModelGeometryPart, Model, ModelDetailCutoff, ModelFlags, ModelGeometry, ModelGeometryPart, ModelNode, ModelRegion, ModelRegionPermutationMarker, ModelShaderReference, ModelVertexCompressed, ModelVertexUncompressed};
use primitives::dynamic::DynamicTagDataArray;
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::{Index, Reflexive, Vector2D};

pub trait ModelFunctions {
    /// Convert into a model tag.
    ///
    /// # Panic
    ///
    /// This function can panic if the model is invalid due to invalid indices.
    fn convert_to_model(self) -> Model;

    /// Convert into a gbxmodel tag.
    fn convert_to_gbxmodel(self) -> GBXModel;

    /// Check the indices for any out-of-bounds.
    fn check_indices(&self) -> RinghopperResult<()>;

    /// Get the shader references
    fn shaders(&self) -> &[ModelShaderReference];

    /// Get the nodes of the model.
    fn nodes(&self) -> &[ModelNode];

    /// Get a mutable reference to the nodes of the model.
    fn nodes_mut(&mut self) -> &mut [ModelNode];

    /// Get the regions of the model.
    fn regions(&self) -> &[ModelRegion];

    /// Get a mutable reference to the regions of the model.
    fn regions_mut(&mut self) -> &mut [ModelRegion];

    /// Get the number of geometries in the model.
    fn geometry_count(&self) -> usize;

    /// Fix runtime markers in model.
    ///
    /// Returns true if the model was fixed, false if the model was OK, and an error if the model is broken.
    fn fix_runtime_markers(&mut self) -> RinghopperResult<bool>;

    /// Fix compressed vertices being missing from a model.
    ///
    /// Returns true if the model was fixed or false if the model was OK (or does not support compressed vertices).
    fn fix_compressed_vertices(&mut self) -> bool;

    /// Fix uncompressed vertices being missing from a model.
    ///
    /// Returns true if the model was fixed or false if the model was OK.
    fn fix_uncompressed_vertices(&mut self) -> bool;

    /// Flip LoD cutoffs.
    ///
    /// This must be called when a model tag enters/exits a cache file.
    fn flip_lod_cutoffs(&mut self);

    /// Return true if the model supports compressed vertices.
    fn supports_compressed_vertices(&mut self) -> bool;
}

macro_rules! fix_runtime_markers {
    ($model:expr) => {{
        let mut changes_made = false;

        for marker in &$model.runtime_markers {
            changes_made = true;
            for instance in &marker.instances {
                let node = instance.node_index as u16;
                let region = instance.region_index as usize;
                let permutation = instance.permutation_index as usize;

                if node as usize >= $model.nodes.items.len() {
                    return Err(Error::InvalidTagData(format!("marker {} has out-of-bounds node", marker.name)))
                }
                if region >= $model.regions.items.len() {
                    return Err(Error::InvalidTagData(format!("marker {} has out-of-bounds region", marker.name)))
                }

                let region = &mut $model.regions.items[region];
                if permutation >= region.permutations.items.len() {
                    return Err(Error::InvalidTagData(format!("marker {} has out-of-bounds permutation", marker.name)))
                }

                let permutation = &mut region.permutations.items[permutation];
                permutation.markers.items.push(ModelRegionPermutationMarker {
                    name: marker.name,
                    node_index: Some(node),
                    rotation: instance.rotation,
                    translation: instance.translation
                })
            }
        }

        // Runtime markers have to be cleared since they are not marked as cache only in the definitions despite being
        // (technically) cache only fields. See the comment in the definition for more information.
        $model.runtime_markers.items.clear();

        Ok(changes_made)
    }};
}

macro_rules! fix_vertices {
    ($model:expr, $fixer:tt) => {{
        let mut fixed = false;

        for g in &mut $model.geometries {
            for p in &mut g.parts {
                fixed |= $fixer(p.get_model_part_mut())
            }
        }

        fixed
    }};
}

#[allow(dead_code)]
pub(crate) trait ModelPartGet {
    /// Get the base model part.
    fn get_model_part(&self) -> &ModelGeometryPart;

    /// Get the base model part.
    fn get_model_part_mut(&mut self) -> &mut ModelGeometryPart;
}

impl ModelPartGet for ModelGeometryPart {
    fn get_model_part(&self) -> &ModelGeometryPart {
        self
    }
    fn get_model_part_mut(&mut self) -> &mut ModelGeometryPart {
        self
    }
}

impl ModelPartGet for GBXModelGeometryPart {
    fn get_model_part(&self) -> &ModelGeometryPart {
        &self.model_geometry_part
    }
    fn get_model_part_mut(&mut self) -> &mut ModelGeometryPart {
        &mut self.model_geometry_part
    }
}

impl ModelFunctions for Model {
    fn convert_to_model(self) -> Model {
        self
    }

    fn convert_to_gbxmodel(self) -> GBXModel {
        GBXModel {
            metadata: Default::default(),
            detail_node_count: self.detail_node_count,
            node_list_checksum: self.node_list_checksum,
            flags: GBXModelFlags {
                blend_shared_normals: self.flags.blend_shared_normals,
                ..Default::default()
            },
            regions: self.regions,
            geometries: Reflexive {
                items: self
                    .geometries
                    .items
                    .into_iter()
                    .map(|g| GBXModelGeometry {
                        flags: g.flags,
                        parts: Reflexive {
                            items: g
                                .parts
                                .items
                                .into_iter()
                                .map(|p| GBXModelGeometryPart {
                                    model_geometry_part: p,
                                    ..Default::default()
                                })
                                .collect()
                        }
                    })
                    .collect()
            },
            nodes: self.nodes,
            shaders: self.shaders,
            detail_cutoff: self.detail_cutoff,
            runtime_markers: self.runtime_markers,
            base_map_v_scale: self.base_map_v_scale,
            base_map_u_scale: self.base_map_u_scale
        }
    }

    fn check_indices(&self) -> RinghopperResult<()> {
        check_base_model(self)?;
        for geometry in &self.geometries {
            for part in &geometry.parts {
                check_indices_for_part(self, &part)?;
                if part.flags.zoner {
                    return Err(Error::InvalidTagData("corrupted model: zoner is set on a tag that does not support local nodes".to_string()))
                }
            }
        }
        Ok(())
    }
    fn shaders(&self) -> &[ModelShaderReference] {
        self.shaders.items.as_ref()
    }
    fn nodes(&self) -> &[ModelNode] {
        self.nodes.items.as_ref()
    }
    fn nodes_mut(&mut self) -> &mut [ModelNode] {
        self.nodes.items.as_mut()
    }
    fn regions(&self) -> &[ModelRegion] {
        self.regions.items.as_ref()
    }
    fn regions_mut(&mut self) -> &mut [ModelRegion] {
        self.regions.items.as_mut()
    }
    fn geometry_count(&self) -> usize {
        self.geometries.items.len()
    }
    fn fix_runtime_markers(&mut self) -> RinghopperResult<bool> {
        fix_runtime_markers!(self)
    }

    fn fix_compressed_vertices(&mut self) -> bool {
        if !self.supports_compressed_vertices() {
            return false
        }

        fix_vertices!(self, restore_missing_compressed_vertices)
    }

    fn fix_uncompressed_vertices(&mut self) -> bool {
        fix_vertices!(self, restore_missing_uncompressed_vertices)
    }

    fn flip_lod_cutoffs(&mut self) {
        flip_lod_cutoffs(&mut self.detail_cutoff)
    }

    fn supports_compressed_vertices(&mut self) -> bool {
        self.nodes.len() <= MAX_NODES_FOR_COMPRESSED_VERTICES
    }
}

const MAX_NODES_FOR_COMPRESSED_VERTICES: usize = 127 / 3;

impl ModelFunctions for GBXModel {
    fn convert_to_model(self) -> Model {
        debug_assert!(self.check_indices().is_ok());

        let uses_local_nodes = self.flags.parts_have_local_nodes;
        let geometries = self.geometries.items.into_iter().map(|g| {
            if uses_local_nodes {
                let parts = g.parts.items.into_iter().map(|p| {
                    let indices = &p.local_node_indices[..p.local_node_count as usize];
                    let mut part = p.model_geometry_part;
                    part.flags.zoner = false;

                    for v in &mut part.uncompressed_vertices {
                        v.node0_index = v.node0_index.map(|m| indices[m as usize] as u16);
                        v.node1_index = v.node1_index.map(|m| indices[m as usize] as u16);
                    }

                    part
                }).collect();

                ModelGeometry {
                    flags: g.flags,
                    parts: Reflexive { items: parts }
                }
            }
            else {
                ModelGeometry {
                    flags: g.flags,
                    parts: Reflexive {
                        items: g.parts.items.into_iter().map(|p| p.model_geometry_part).collect()
                    }
                }
            }
        }).collect();

        Model {
            metadata: Default::default(),
            flags: ModelFlags {
                blend_shared_normals: self.flags.blend_shared_normals
            },
            node_list_checksum: self.node_list_checksum,
            detail_cutoff: self.detail_cutoff,
            detail_node_count: self.detail_node_count,
            base_map_u_scale: self.base_map_u_scale,
            base_map_v_scale: self.base_map_v_scale,
            runtime_markers: self.runtime_markers,
            nodes: self.nodes,
            regions: self.regions,
            geometries: Reflexive {
                items: geometries
            },
            shaders: self.shaders
        }
    }

    fn convert_to_gbxmodel(self) -> GBXModel {
        self
    }

    fn check_indices(&self) -> RinghopperResult<()> {
        check_base_model(self)?;
        for geometry in &self.geometries {
            for part in &geometry.parts {
                check_indices_for_gbxpart(self, &part)?;
                if !self.flags.parts_have_local_nodes && part.model_geometry_part.flags.zoner {
                    return Err(Error::InvalidTagData("corrupted model: parts have local nodes is not set, but zoner is set".to_string()))
                }
                if self.flags.parts_have_local_nodes && !part.model_geometry_part.flags.zoner {
                    return Err(Error::InvalidTagData("corrupted model: parts have local nodes is set, but zoner is not set".to_string()))
                }
            }
        }
        Ok(())
    }

    fn shaders(&self) -> &[ModelShaderReference] {
        self.shaders.items.as_ref()
    }
    fn nodes(&self) -> &[ModelNode] {
        self.nodes.items.as_ref()
    }
    fn nodes_mut(&mut self) -> &mut [ModelNode] {
        self.nodes.items.as_mut()
    }
    fn regions(&self) -> &[ModelRegion] {
        self.regions.items.as_ref()
    }
    fn regions_mut(&mut self) -> &mut [ModelRegion] {
        self.regions.items.as_mut()
    }
    fn geometry_count(&self) -> usize {
        self.geometries.items.len()
    }
    fn fix_runtime_markers(&mut self) -> RinghopperResult<bool> {
        fix_runtime_markers!(self)
    }

    fn fix_compressed_vertices(&mut self) -> bool {
        if !self.supports_compressed_vertices() {
            return false
        }

        fix_vertices!(self, restore_missing_compressed_vertices)
    }

    fn fix_uncompressed_vertices(&mut self) -> bool {
        fix_vertices!(self, restore_missing_uncompressed_vertices)
    }

    fn flip_lod_cutoffs(&mut self) {
        flip_lod_cutoffs(&mut self.detail_cutoff)
    }

    fn supports_compressed_vertices(&mut self) -> bool {
        self.nodes.len() <= MAX_NODES_FOR_COMPRESSED_VERTICES && !self.flags.parts_have_local_nodes
    }
}

// Check everything except geometries
fn check_base_model<M: ModelFunctions>(model: &M) -> RinghopperResult<()> {
    let nodes = model.nodes();
    let node_count = nodes.len();
    let geometry_count = model.geometry_count();

    let bad_node_index = |index: Index|
        if index.is_some_and(|i| node_count <= (i as usize)) {
            Err(Error::InvalidTagData(format!("corrupted model - model node index out-of-bounds {index:?}")))
        }
        else {
            Ok(())
        };

    let bad_geometry_index = |index: Index|
        if index.is_some_and(|i| geometry_count <= (i as usize)) {
            Err(Error::InvalidTagData(format!("corrupted model - geometry index out-of-bounds {index:?}")))
        }
        else {
            Ok(())
        };

    for node in nodes {
        bad_node_index(node.next_sibling_node_index)?;
        bad_node_index(node.first_child_node_index)?;
        bad_node_index(node.parent_node_index)?;
    }

    for region in model.regions() {
        for permutation in &region.permutations {
            bad_geometry_index(permutation.high)?;
            bad_geometry_index(permutation.low)?;
            bad_geometry_index(permutation.super_high)?;
            bad_geometry_index(permutation.super_low)?;
            bad_geometry_index(permutation.medium)?;

            for m in &permutation.markers {
                bad_node_index(m.node_index)?;
            }
        }
    }

    Ok(())
}

// Check the shader and centroid node indices
fn check_base_part<M: ModelFunctions>(model: &M, part: &ModelGeometryPart) -> RinghopperResult<()> {
    let shaders = model.shaders();
    if part.shader_index.is_some_and(|s| shaders.get(s as usize).is_none()) {
        return Err(Error::InvalidTagData(format!("corrupted model: invalid shader index {:?}", part.shader_index)))
    }

    let nodes = model.nodes();
    if part.centroid_primary_node.is_some_and(|n| nodes.get(n as usize).is_none()) {
        return Err(Error::InvalidTagData(format!("corrupted model: invalid centroid_primary_node index {:?}", part.centroid_primary_node)))
    }
    if part.centroid_secondary_node.is_some_and(|n| nodes.get(n as usize).is_none()) {
        return Err(Error::InvalidTagData(format!("corrupted model: invalid centroid_secondary_node index {:?}", part.centroid_secondary_node)))
    }

    Ok(())
}

fn check_indices_for_part<M: ModelFunctions>(model: &M, part: &ModelGeometryPart) -> RinghopperResult<()> {
    let nodes = model.nodes();
    check_vertex_buffer(part.uncompressed_vertices.items.iter().map(ToOwned::to_owned), nodes, None)?;
    check_vertex_buffer(part.compressed_vertices.items.iter().map(decompress_model_vertex), nodes, None)?;
    Ok(())
}

fn decompress_model_vertex(model_vertex_compressed: &ModelVertexCompressed) -> ModelVertexUncompressed {
    let node0_weight: f32 = model_vertex_compressed.node0_weight.into();
    ModelVertexUncompressed {
        position: model_vertex_compressed.position,
        node0_index: match model_vertex_compressed.node0_index {
            n if n >= 253 => None,
            n => Some((n as u16) / 3)
        },
        node1_index: match model_vertex_compressed.node1_index {
            n if n >= 253 => None,
            n => Some((n as u16) / 3)
        },
        normal: model_vertex_compressed.normal.into(),
        binormal: model_vertex_compressed.binormal.into(),
        tangent: model_vertex_compressed.tangent.into(),
        node0_weight,
        node1_weight: 1.0 - node0_weight,
        texture_coords: Vector2D {
            x: model_vertex_compressed.texture_coordinate_u.into(),
            y: model_vertex_compressed.texture_coordinate_v.into()
        }
    }
}

fn compress_model_vertex(model_vertex_uncompressed: &ModelVertexUncompressed) -> ModelVertexCompressed {
    let mut total_weight = model_vertex_uncompressed.node0_weight + model_vertex_uncompressed.node1_weight;
    if total_weight == 0.0 {
        total_weight = 1.0;
    }

    let partial_weight = model_vertex_uncompressed.node0_weight / total_weight;

    debug_assert!(model_vertex_uncompressed.node0_index.unwrap_or(0) < 128 / 3);
    debug_assert!(model_vertex_uncompressed.node1_index.unwrap_or(0) < 128 / 3);

    ModelVertexCompressed {
        node0_weight: partial_weight.into(),
        node0_index: model_vertex_uncompressed.node0_index.map(|n| n * 3).unwrap_or(253) as u8,
        node1_index: model_vertex_uncompressed.node1_index.map(|n| n * 3).unwrap_or(253) as u8,
        texture_coordinate_u: model_vertex_uncompressed.texture_coords.x.into(),
        texture_coordinate_v: model_vertex_uncompressed.texture_coords.y.into(),
        position: model_vertex_uncompressed.position,
        tangent: model_vertex_uncompressed.tangent.into(),
        normal: model_vertex_uncompressed.normal.into(),
        binormal: model_vertex_uncompressed.binormal.into(),
    }
}

// Check the node0/node1 indices for an iterator
fn check_vertex_buffer<I: Iterator<Item = ModelVertexUncompressed>>(vertices: I, nodes: &[ModelNode], local_nodes: Option<&[u8]>) -> RinghopperResult<()> {
    let resolve = |node: &Index| -> RinghopperResult<()> {
        if let Some(n) = node.map(|i| i as usize) {
            let real_index = match local_nodes {
                Some(local_nodes) => match local_nodes.get(n) {
                    Some(n) => *n as usize,
                    None => return Err(Error::InvalidTagData(format!("corrupted model: invalid local node index {n}")))
                },
                None => n
            };
            nodes
                .get(real_index)
                .map(|_| ())
                .ok_or_else(|| Error::InvalidTagData(format!("corrupted model: invalid vertex node index {real_index}")))
        }
        else {
            Ok(())
        }
    };

    for vertex in vertices {
        resolve(&vertex.node0_index)?;
        resolve(&vertex.node1_index)?;
    }

    Ok(())
}

// Check indices for a GBXModel part
fn check_indices_for_gbxpart(gbxmodel: &GBXModel, gbxpart: &GBXModelGeometryPart) -> RinghopperResult<()> {
    check_base_part(gbxmodel, &gbxpart.model_geometry_part)?;

    if gbxmodel.flags.parts_have_local_nodes {
        let nodes = gbxmodel.nodes();

        let local_nodes = gbxpart
            .local_node_indices
            .get(..gbxpart.local_node_count as usize)
            .ok_or_else(|| Error::InvalidTagData("corrupted model: invalid local node count".to_owned()))?;

        check_vertex_buffer(gbxpart.model_geometry_part.uncompressed_vertices.items.iter().map(ToOwned::to_owned), nodes, Some(local_nodes))?;
        check_vertex_buffer(gbxpart.model_geometry_part.compressed_vertices.items.iter().map(decompress_model_vertex), nodes, None)?;
    }
    else {
        check_indices_for_part(gbxmodel, &gbxpart.model_geometry_part)?;
    }

    Ok(())
}

fn restore_missing_compressed_vertices(part: &mut ModelGeometryPart) -> bool {
    if !part.compressed_vertices.items.is_empty() {
        return false
    }

    part.compressed_vertices.items.reserve_exact(part.uncompressed_vertices.items.len());
    for vert in &part.uncompressed_vertices {
        part.compressed_vertices.items.push(compress_model_vertex(&vert));
    }

    true
}

fn restore_missing_uncompressed_vertices(part: &mut ModelGeometryPart) -> bool {
    if !part.uncompressed_vertices.items.is_empty() {
        return false
    }

    part.uncompressed_vertices.items.reserve_exact(part.compressed_vertices.items.len());
    for vert in &part.compressed_vertices {
        part.uncompressed_vertices.items.push(decompress_model_vertex(&vert));
    }

    true
}

fn flip_lod_cutoffs(cutoff: &mut ModelDetailCutoff) {
    std::mem::swap(&mut cutoff.super_low, &mut cutoff.super_high);
    std::mem::swap(&mut cutoff.low, &mut cutoff.high);
}
