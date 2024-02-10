use definitions::{GBXModel, GBXModelFlags, GBXModelGeometry, GBXModelGeometryPart, Model, ModelFlags, ModelGeometry, ModelGeometryPart, ModelNode, ModelRegion, ModelShaderReference, ModelVertexCompressed, ModelVertexUncompressed};
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::{Index, Reflexive, Vector2D, Vector3D};

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

    /// Get the regions of the model.
    fn regions(&self) -> &[ModelRegion];

    /// Get the number of geometries in the model.
    fn geometry_count(&self) -> usize;
}

impl ModelFunctions for Model {
    fn convert_to_model(self) -> Model {
        self
    }

    fn convert_to_gbxmodel(self) -> GBXModel {
        GBXModel {
            hash: 0,
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
        for geometry in &self.geometries.items {
            for part in &geometry.parts.items {
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
    fn regions(&self) -> &[ModelRegion] {
        self.regions.items.as_ref()
    }
    fn geometry_count(&self) -> usize {
        self.geometries.items.len()
    }
}

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

                    for v in &mut part.uncompressed_vertices.items {
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
            hash: 0,
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
        for geometry in &self.geometries.items {
            for part in &geometry.parts.items {
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

    fn regions(&self) -> &[ModelRegion] {
        self.regions.items.as_ref()
    }
    fn geometry_count(&self) -> usize {
        self.geometries.items.len()
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
        for permutation in &region.permutations.items {
            bad_geometry_index(permutation.high)?;
            bad_geometry_index(permutation.low)?;
            bad_geometry_index(permutation.super_high)?;
            bad_geometry_index(permutation.super_low)?;
            bad_geometry_index(permutation.medium)?;

            for m in &permutation.markers.items {
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

fn decompress_vector3d(vector: u32) -> Vector3D {
    Vector3D {
        x: decompress_float::<11>(vector),
        y: decompress_float::<11>(vector >> 11),
        z: decompress_float::<10>(vector >> 22),
    }
}

fn decompress_float<const BITS: usize>(float: u32) -> f32 {
    let signed_bit = 1u32 << BITS;
    let mask = signed_bit - 1;
    let value = (float & mask) as f64 / (mask as f64);
    if (float & signed_bit) != 0 {
        (value - 1.0) as f32
    }
    else {
        value as f32
    }
}

fn decompress_model_vertex(model_vertex_compressed: &ModelVertexCompressed) -> ModelVertexUncompressed {
    let node0_weight = model_vertex_compressed.node0_weight as f32 / u16::MAX as f32;
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
        normal: decompress_vector3d(model_vertex_compressed.normal),
        binormal: decompress_vector3d(model_vertex_compressed.binormal),
        tangent: decompress_vector3d(model_vertex_compressed.tangent),
        node0_weight,
        node1_weight: 1.0 - node0_weight,
        texture_coords: Vector2D {
            x: decompress_float::<16>(model_vertex_compressed.texture_coordinate_u as u32),
            y: decompress_float::<16>(model_vertex_compressed.texture_coordinate_v as u32)
        }
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