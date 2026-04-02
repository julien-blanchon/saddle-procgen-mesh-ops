use bevy::prelude::*;

use crate::{
    EdgeId, FaceId, HalfEdgeId, MeshError, VertexId,
    attributes::{FacePayload, LoopAttributes, VertexPayload},
    iterators::{FaceHalfedges, FaceVertices, VertexOutgoingHalfedges},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum FaceKind {
    Interior,
    Boundary,
}

#[derive(Debug, Clone, Reflect)]
pub(crate) struct VertexRecord {
    pub outgoing: HalfEdgeId,
    pub data: VertexPayload,
}

#[derive(Debug, Clone, Reflect)]
pub(crate) struct HalfEdgeRecord {
    pub origin: VertexId,
    pub twin: HalfEdgeId,
    pub next: HalfEdgeId,
    pub prev: HalfEdgeId,
    pub face: FaceId,
    pub edge: EdgeId,
    pub data: LoopAttributes,
}

#[derive(Debug, Clone, Reflect)]
pub(crate) struct EdgeRecord {
    pub halfedge: HalfEdgeId,
}

#[derive(Debug, Clone, Reflect)]
pub(crate) struct FaceRecord {
    pub halfedge: HalfEdgeId,
    pub kind: FaceKind,
    pub data: FacePayload,
}

#[derive(Debug, Clone, Default, Reflect)]
pub struct HalfEdgeMesh {
    pub(crate) vertices: Vec<VertexRecord>,
    pub(crate) halfedges: Vec<HalfEdgeRecord>,
    pub(crate) edges: Vec<EdgeRecord>,
    pub(crate) faces: Vec<FaceRecord>,
    pub(crate) interior_face_count: usize,
}

impl HalfEdgeMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.halfedges.is_empty() && self.edges.is_empty()
    }

    pub fn add_vertex(&mut self, position: Vec3) -> VertexId {
        self.add_vertex_with_payload(VertexPayload {
            position,
            ..default()
        })
    }

    pub fn add_vertex_with_payload(&mut self, payload: VertexPayload) -> VertexId {
        let id = VertexId(self.vertices.len() as u32);
        self.vertices.push(VertexRecord {
            outgoing: HalfEdgeId::INVALID,
            data: payload,
        });
        id
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn halfedge_count(&self) -> usize {
        self.halfedges.len()
    }

    pub fn face_count(&self) -> usize {
        self.interior_face_count
    }

    pub fn boundary_face_count(&self) -> usize {
        self.faces.len().saturating_sub(self.interior_face_count)
    }

    pub fn total_face_count(&self) -> usize {
        self.faces.len()
    }

    pub fn vertex_ids(&self) -> impl Iterator<Item = VertexId> + '_ {
        (0..self.vertices.len()).map(|index| VertexId(index as u32))
    }

    pub fn edge_ids(&self) -> impl Iterator<Item = EdgeId> + '_ {
        (0..self.edges.len()).map(|index| EdgeId(index as u32))
    }

    pub fn halfedge_ids(&self) -> impl Iterator<Item = HalfEdgeId> + '_ {
        (0..self.halfedges.len()).map(|index| HalfEdgeId(index as u32))
    }

    pub fn face_ids(&self) -> impl Iterator<Item = FaceId> + '_ {
        (0..self.interior_face_count).map(|index| FaceId(index as u32))
    }

    pub fn boundary_face_ids(&self) -> impl Iterator<Item = FaceId> + '_ {
        (self.interior_face_count..self.faces.len()).map(|index| FaceId(index as u32))
    }

    pub fn vertex_payload(&self, vertex: VertexId) -> Result<&VertexPayload, MeshError> {
        self.vertices
            .get(vertex.index())
            .map(|record| &record.data)
            .ok_or(MeshError::InvalidVertex(vertex))
    }

    pub fn vertex_payload_mut(
        &mut self,
        vertex: VertexId,
    ) -> Result<&mut VertexPayload, MeshError> {
        self.vertices
            .get_mut(vertex.index())
            .map(|record| &mut record.data)
            .ok_or(MeshError::InvalidVertex(vertex))
    }

    pub fn face_payload(&self, face: FaceId) -> Result<&FacePayload, MeshError> {
        self.faces
            .get(face.index())
            .map(|record| &record.data)
            .ok_or(MeshError::InvalidFace(face))
    }

    pub fn face_payload_mut(&mut self, face: FaceId) -> Result<&mut FacePayload, MeshError> {
        self.faces
            .get_mut(face.index())
            .map(|record| &mut record.data)
            .ok_or(MeshError::InvalidFace(face))
    }

    pub fn face_kind(&self, face: FaceId) -> Result<FaceKind, MeshError> {
        self.faces
            .get(face.index())
            .map(|record| record.kind)
            .ok_or(MeshError::InvalidFace(face))
    }

    pub fn halfedge_loop_attributes(
        &self,
        halfedge: HalfEdgeId,
    ) -> Result<&LoopAttributes, MeshError> {
        self.halfedges
            .get(halfedge.index())
            .map(|record| &record.data)
            .ok_or(MeshError::InvalidHalfEdge(halfedge))
    }

    pub fn halfedge_loop_attributes_mut(
        &mut self,
        halfedge: HalfEdgeId,
    ) -> Result<&mut LoopAttributes, MeshError> {
        self.halfedges
            .get_mut(halfedge.index())
            .map(|record| &mut record.data)
            .ok_or(MeshError::InvalidHalfEdge(halfedge))
    }

    pub fn halfedge_origin(&self, halfedge: HalfEdgeId) -> Result<VertexId, MeshError> {
        self.halfedges
            .get(halfedge.index())
            .map(|record| record.origin)
            .ok_or(MeshError::InvalidHalfEdge(halfedge))
    }

    pub fn halfedge_destination(&self, halfedge: HalfEdgeId) -> Result<VertexId, MeshError> {
        let twin = self.halfedge_twin(halfedge)?;
        self.halfedge_origin(twin)
    }

    pub fn halfedge_face(&self, halfedge: HalfEdgeId) -> Result<FaceId, MeshError> {
        self.halfedges
            .get(halfedge.index())
            .map(|record| record.face)
            .ok_or(MeshError::InvalidHalfEdge(halfedge))
    }

    pub fn halfedge_edge(&self, halfedge: HalfEdgeId) -> Result<EdgeId, MeshError> {
        self.halfedges
            .get(halfedge.index())
            .map(|record| record.edge)
            .ok_or(MeshError::InvalidHalfEdge(halfedge))
    }

    pub fn halfedge_twin(&self, halfedge: HalfEdgeId) -> Result<HalfEdgeId, MeshError> {
        self.halfedges
            .get(halfedge.index())
            .map(|record| record.twin)
            .ok_or(MeshError::InvalidHalfEdge(halfedge))
    }

    pub fn halfedge_next(&self, halfedge: HalfEdgeId) -> Result<HalfEdgeId, MeshError> {
        self.halfedges
            .get(halfedge.index())
            .map(|record| record.next)
            .ok_or(MeshError::InvalidHalfEdge(halfedge))
    }

    pub fn halfedge_prev(&self, halfedge: HalfEdgeId) -> Result<HalfEdgeId, MeshError> {
        self.halfedges
            .get(halfedge.index())
            .map(|record| record.prev)
            .ok_or(MeshError::InvalidHalfEdge(halfedge))
    }

    pub fn edge_halfedges(&self, edge: EdgeId) -> Result<(HalfEdgeId, HalfEdgeId), MeshError> {
        let halfedge = self
            .edges
            .get(edge.index())
            .map(|record| record.halfedge)
            .ok_or(MeshError::InvalidEdge(edge))?;
        Ok((halfedge, self.halfedge_twin(halfedge)?))
    }

    pub fn edge_endpoints(&self, edge: EdgeId) -> Result<(VertexId, VertexId), MeshError> {
        let (halfedge, twin) = self.edge_halfedges(edge)?;
        Ok((self.halfedge_origin(halfedge)?, self.halfedge_origin(twin)?))
    }

    pub fn edge_is_boundary(&self, edge: EdgeId) -> Result<bool, MeshError> {
        let (halfedge, twin) = self.edge_halfedges(edge)?;
        Ok(self.halfedge_is_boundary(halfedge)? || self.halfedge_is_boundary(twin)?)
    }

    pub fn halfedge_is_boundary(&self, halfedge: HalfEdgeId) -> Result<bool, MeshError> {
        let face = self.halfedge_face(halfedge)?;
        Ok(matches!(self.face_kind(face)?, FaceKind::Boundary))
    }

    pub fn face_halfedges(&self, face: FaceId) -> Result<FaceHalfedges<'_>, MeshError> {
        let start = self
            .faces
            .get(face.index())
            .map(|record| record.halfedge)
            .ok_or(MeshError::InvalidFace(face))?;
        Ok(FaceHalfedges::new(self, start))
    }

    pub fn face_vertices(&self, face: FaceId) -> Result<FaceVertices<'_>, MeshError> {
        let start = self
            .faces
            .get(face.index())
            .map(|record| record.halfedge)
            .ok_or(MeshError::InvalidFace(face))?;
        Ok(FaceVertices::new(self, start))
    }

    pub fn vertex_outgoing_halfedges(
        &self,
        vertex: VertexId,
    ) -> Result<VertexOutgoingHalfedges<'_>, MeshError> {
        let start = self
            .vertices
            .get(vertex.index())
            .map(|record| record.outgoing)
            .ok_or(MeshError::InvalidVertex(vertex))?;
        Ok(VertexOutgoingHalfedges::new(self, start))
    }

    pub fn face_vertex_ids(&self, face: FaceId) -> Result<Vec<VertexId>, MeshError> {
        Ok(self.face_vertices(face)?.collect())
    }

    pub fn face_positions(&self, face: FaceId) -> Result<Vec<Vec3>, MeshError> {
        self.face_vertices(face)?
            .map(|vertex| Ok(self.vertex_payload(vertex)?.position))
            .collect()
    }

    pub fn face_loop_attributes(&self, face: FaceId) -> Result<Vec<LoopAttributes>, MeshError> {
        self.face_halfedges(face)?
            .map(|halfedge| self.halfedge_loop_attributes(halfedge).cloned())
            .collect()
    }

    pub fn face_centroid(&self, face: FaceId) -> Result<Vec3, MeshError> {
        let positions = self.face_positions(face)?;
        if positions.is_empty() {
            return Err(MeshError::DegenerateFace(face));
        }

        let sum = positions
            .iter()
            .copied()
            .fold(Vec3::ZERO, |acc, position| acc + position);
        Ok(sum / positions.len() as f32)
    }

    pub fn face_normal(&self, face: FaceId) -> Result<Vec3, MeshError> {
        let positions = self.face_positions(face)?;
        if positions.len() < 3 {
            return Err(MeshError::DegenerateFace(face));
        }

        let mut normal = Vec3::ZERO;
        for index in 0..positions.len() {
            let current = positions[index];
            let next = positions[(index + 1) % positions.len()];
            normal.x += (current.y - next.y) * (current.z + next.z);
            normal.y += (current.z - next.z) * (current.x + next.x);
            normal.z += (current.x - next.x) * (current.y + next.y);
        }

        let normal = normal.normalize_or_zero();
        if normal == Vec3::ZERO {
            return Err(MeshError::DegenerateFace(face));
        }

        Ok(normal)
    }

    pub fn face_area(&self, face: FaceId) -> Result<f32, MeshError> {
        let positions = self.face_positions(face)?;
        if positions.len() < 3 {
            return Err(MeshError::DegenerateFace(face));
        }

        let origin = positions[0];
        let mut area = 0.0;
        for index in 1..positions.len() - 1 {
            let left = positions[index] - origin;
            let right = positions[index + 1] - origin;
            area += left.cross(right).length() * 0.5;
        }
        Ok(area)
    }

    pub fn vertex_normal(&self, vertex: VertexId) -> Result<Vec3, MeshError> {
        let mut normal = Vec3::ZERO;
        for halfedge in self.vertex_outgoing_halfedges(vertex)? {
            let face = self.halfedge_face(halfedge)?;
            if matches!(self.face_kind(face)?, FaceKind::Boundary) {
                continue;
            }
            let area = self.face_area(face)?;
            let face_normal = self.face_normal(face)?;
            normal += face_normal * area;
        }
        Ok(normal.normalize_or_zero())
    }

    pub fn has_loop_uvs(&self) -> bool {
        self.halfedges
            .iter()
            .any(|halfedge| halfedge.data.uv.is_some())
    }

    pub fn has_loop_normals(&self) -> bool {
        self.halfedges
            .iter()
            .any(|halfedge| halfedge.data.normal.is_some())
    }

    pub fn has_loop_tangents(&self) -> bool {
        self.halfedges
            .iter()
            .any(|halfedge| halfedge.data.tangent.is_some())
    }
}

#[cfg(test)]
#[path = "mesh_tests.rs"]
mod tests;
