use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};

use crate::{
    HalfEdgeMesh, LoopAttributes, MeshError,
    topology::{MeshSnapshot, PolygonFace},
};

impl HalfEdgeMesh {
    pub fn from_bevy_mesh(mesh: &Mesh) -> Result<Self, MeshError> {
        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
            return Err(MeshError::UnsupportedPrimitiveTopology(format!(
                "{:?}",
                mesh.primitive_topology()
            )));
        }

        let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(values)) => values,
            Some(other) => {
                return Err(MeshError::UnsupportedMesh(format!(
                    "position attribute has unsupported format {other:?}"
                )));
            }
            None => return Err(MeshError::MissingAttribute("POSITION")),
        };

        let normals = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            Some(VertexAttributeValues::Float32x3(values)) => Some(values.as_slice()),
            Some(other) => {
                return Err(MeshError::UnsupportedMesh(format!(
                    "normal attribute has unsupported format {other:?}"
                )));
            }
            None => None,
        };

        let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            Some(VertexAttributeValues::Float32x2(values)) => Some(values.as_slice()),
            Some(other) => {
                return Err(MeshError::UnsupportedMesh(format!(
                    "uv attribute has unsupported format {other:?}"
                )));
            }
            None => None,
        };

        let tangents = match mesh.attribute(Mesh::ATTRIBUTE_TANGENT) {
            Some(VertexAttributeValues::Float32x4(values)) => Some(values.as_slice()),
            Some(other) => {
                return Err(MeshError::UnsupportedMesh(format!(
                    "tangent attribute has unsupported format {other:?}"
                )));
            }
            None => None,
        };

        let indices: Vec<usize> = match mesh.indices() {
            Some(Indices::U16(values)) => values.iter().map(|value| *value as usize).collect(),
            Some(Indices::U32(values)) => values.iter().map(|value| *value as usize).collect(),
            None => return Err(MeshError::MissingIndices),
        };

        if !indices.len().is_multiple_of(3) {
            return Err(MeshError::UnsupportedMesh(
                "triangle index buffer length must be divisible by 3".to_string(),
            ));
        }

        let mut snapshot = MeshSnapshot::default();
        let mut welded_vertices = HashMap::<(u32, u32, u32), usize>::new();

        for triangle in indices.chunks_exact(3) {
            let mut face_vertices = Vec::with_capacity(3);
            let mut face_loops = Vec::with_capacity(3);

            for &corner_index in triangle {
                let position = positions
                    .get(corner_index)
                    .ok_or_else(|| {
                        MeshError::UnsupportedMesh(format!(
                            "index {corner_index} points beyond the position attribute"
                        ))
                    })
                    .copied()?;
                let position_vec = Vec3::from(position);
                let key = (
                    position[0].to_bits(),
                    position[1].to_bits(),
                    position[2].to_bits(),
                );

                let vertex_index = *welded_vertices.entry(key).or_insert_with(|| {
                    snapshot.vertices.push(crate::VertexPayload {
                        position: position_vec,
                        ..default()
                    });
                    snapshot.vertices.len() - 1
                });

                face_vertices.push(vertex_index);
                face_loops.push(LoopAttributes {
                    uv: uvs.and_then(|values| values.get(corner_index).copied().map(Vec2::from)),
                    normal: normals
                        .and_then(|values| values.get(corner_index).copied().map(Vec3::from)),
                    tangent: tangents
                        .and_then(|values| values.get(corner_index).copied().map(Vec4::from)),
                });
            }

            snapshot.faces.push(PolygonFace {
                vertices: face_vertices,
                loops: face_loops,
                data: default(),
            });
        }

        Self::from_snapshot(snapshot)
    }

    pub fn to_bevy_mesh(&self) -> Result<Mesh, MeshError> {
        let mut export_mesh = self.clone();
        if !export_mesh.has_loop_normals() {
            export_mesh.recompute_normals()?;
        }
        if export_mesh.has_loop_uvs() && !export_mesh.has_loop_tangents() {
            let _ = export_mesh.recompute_tangents();
        }

        let has_uvs = export_mesh.has_loop_uvs();
        let has_tangents = export_mesh.has_loop_tangents();

        let mut positions = Vec::<[f32; 3]>::new();
        let mut normals = Vec::<[f32; 3]>::new();
        let mut uvs = Vec::<[f32; 2]>::new();
        let mut tangents = Vec::<[f32; 4]>::new();
        let mut indices = Vec::<u32>::new();

        for face in export_mesh.face_ids() {
            let halfedges = export_mesh.face_halfedges(face)?.collect::<Vec<_>>();
            if halfedges.len() < 3 {
                return Err(MeshError::TriangulationFailed(face));
            }

            let base_index = positions.len() as u32;
            for halfedge in &halfedges {
                let vertex = export_mesh.halfedge_origin(*halfedge)?;
                let payload = export_mesh.vertex_payload(vertex)?;
                let loop_data = export_mesh.halfedge_loop_attributes(*halfedge)?;
                positions.push(payload.position.to_array());
                normals.push(loop_data.normal.unwrap_or(Vec3::Z).to_array());
                if has_uvs {
                    uvs.push(loop_data.uv.unwrap_or(Vec2::ZERO).to_array());
                }
                if has_tangents {
                    tangents.push(loop_data.tangent.unwrap_or(Vec4::X).to_array());
                }
            }

            for corner in 1..halfedges.len() - 1 {
                indices.extend_from_slice(&[
                    base_index,
                    base_index + corner as u32,
                    base_index + corner as u32 + 1,
                ]);
            }
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        if has_uvs {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }
        if has_tangents {
            mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
        }
        mesh.insert_indices(Indices::U32(indices));
        Ok(mesh)
    }
}

#[cfg(test)]
#[path = "conversion_tests.rs"]
mod tests;
