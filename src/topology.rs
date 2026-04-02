use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

use crate::{
    EdgeId, FaceId, HalfEdgeId, MeshError, VertexId,
    attributes::{FacePayload, LoopAttributes, VertexPayload},
    mesh::{EdgeRecord, FaceKind, FaceRecord, HalfEdgeMesh, HalfEdgeRecord, VertexRecord},
};

#[derive(Debug, Clone, Default, Reflect)]
pub struct PolygonFace {
    pub vertices: Vec<usize>,
    pub loops: Vec<LoopAttributes>,
    pub data: FacePayload,
}

impl PolygonFace {
    pub fn new(vertices: Vec<usize>) -> Self {
        Self {
            vertices,
            loops: Vec::new(),
            data: FacePayload::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Reflect)]
pub struct MeshSnapshot {
    pub vertices: Vec<VertexPayload>,
    pub faces: Vec<PolygonFace>,
}

impl MeshSnapshot {
    pub fn compact(&mut self) {
        let _ = self.compact_with_map();
    }

    pub fn compact_with_map(&mut self) -> Vec<Option<usize>> {
        let mut used = vec![false; self.vertices.len()];
        for face in &self.faces {
            for &vertex in &face.vertices {
                if let Some(entry) = used.get_mut(vertex) {
                    *entry = true;
                }
            }
        }

        let mut remap = vec![None; self.vertices.len()];
        let mut new_vertices = Vec::with_capacity(self.vertices.len());
        for (index, vertex) in self.vertices.iter().cloned().enumerate() {
            if used.get(index).copied().unwrap_or(false) {
                remap[index] = Some(new_vertices.len());
                new_vertices.push(vertex);
            }
        }

        for face in &mut self.faces {
            for vertex in &mut face.vertices {
                *vertex =
                    remap[*vertex].expect("compaction only remaps vertices referenced by faces");
            }
        }

        self.vertices = new_vertices;
        remap
    }
}

impl HalfEdgeMesh {
    pub fn from_snapshot(snapshot: MeshSnapshot) -> Result<Self, MeshError> {
        Self::from_polygon_faces(snapshot.vertices, snapshot.faces)
    }

    pub fn from_polygon_faces(
        vertices: Vec<VertexPayload>,
        faces: Vec<PolygonFace>,
    ) -> Result<Self, MeshError> {
        let mut mesh = Self::new();
        mesh.vertices = vertices
            .into_iter()
            .map(|data| VertexRecord {
                outgoing: HalfEdgeId::INVALID,
                data,
            })
            .collect();
        mesh.interior_face_count = faces.len();
        mesh.faces.reserve(faces.len());

        let mut directed_edges = HashMap::<(usize, usize), HalfEdgeId>::new();
        let mut edge_keys = Vec::<(usize, usize)>::new();

        for face in faces {
            if face.vertices.len() < 3 {
                return Err(MeshError::FaceTooSmall {
                    count: face.vertices.len(),
                });
            }
            if !face.loops.is_empty() && face.loops.len() != face.vertices.len() {
                return Err(MeshError::Validation(
                    "loop attribute count must match face corner count".to_string(),
                ));
            }

            let face_id = FaceId(mesh.faces.len() as u32);
            mesh.faces.push(FaceRecord {
                halfedge: HalfEdgeId::INVALID,
                kind: FaceKind::Interior,
                data: face.data,
            });

            let first_halfedge = mesh.halfedges.len();
            for index in 0..face.vertices.len() {
                let origin = face.vertices[index];
                let destination = face.vertices[(index + 1) % face.vertices.len()];
                if origin >= mesh.vertices.len() || destination >= mesh.vertices.len() {
                    return Err(MeshError::Validation(format!(
                        "face references out-of-range vertex {origin}->{destination}"
                    )));
                }
                if origin == destination {
                    return Err(MeshError::DuplicateConsecutiveVertex { corner: index });
                }

                let origin_id = VertexId(origin as u32);
                let destination_id = VertexId(destination as u32);
                if directed_edges.contains_key(&(origin, destination)) {
                    return Err(MeshError::DuplicateDirectedEdge {
                        from: origin_id,
                        to: destination_id,
                    });
                }

                let halfedge_id = HalfEdgeId(mesh.halfedges.len() as u32);
                let attributes = face.loops.get(index).cloned().unwrap_or_default();
                mesh.halfedges.push(HalfEdgeRecord {
                    origin: origin_id,
                    twin: HalfEdgeId::INVALID,
                    next: HalfEdgeId::INVALID,
                    prev: HalfEdgeId::INVALID,
                    face: face_id,
                    edge: EdgeId::INVALID,
                    data: attributes,
                });
                edge_keys.push((origin, destination));
                directed_edges.insert((origin, destination), halfedge_id);

                if !mesh.vertices[origin].outgoing.is_valid() {
                    mesh.vertices[origin].outgoing = halfedge_id;
                }
                if index == 0 {
                    mesh.faces[face_id.index()].halfedge = halfedge_id;
                }
            }

            let corner_count = face.vertices.len();
            for offset in 0..corner_count {
                let halfedge = HalfEdgeId((first_halfedge + offset) as u32);
                let next = HalfEdgeId((first_halfedge + (offset + 1) % corner_count) as u32);
                let prev = HalfEdgeId(
                    (first_halfedge + (offset + corner_count - 1) % corner_count) as u32,
                );
                mesh.halfedges[halfedge.index()].next = next;
                mesh.halfedges[halfedge.index()].prev = prev;
            }
        }

        let mut boundary_interior_halfedges = Vec::new();
        for halfedge in mesh.halfedge_ids().collect::<Vec<_>>() {
            if mesh.halfedges[halfedge.index()].edge.is_valid() {
                continue;
            }

            let (origin, destination) = edge_keys[halfedge.index()];
            if let Some(&twin) = directed_edges.get(&(destination, origin)) {
                if twin == halfedge {
                    return Err(MeshError::Validation(
                        "self-twin edge detected while building topology".to_string(),
                    ));
                }

                if mesh.halfedges[twin.index()].edge.is_valid() {
                    continue;
                }

                let edge_id = EdgeId(mesh.edges.len() as u32);
                mesh.edges.push(EdgeRecord { halfedge });
                mesh.halfedges[halfedge.index()].twin = twin;
                mesh.halfedges[twin.index()].twin = halfedge;
                mesh.halfedges[halfedge.index()].edge = edge_id;
                mesh.halfedges[twin.index()].edge = edge_id;
            } else {
                boundary_interior_halfedges.push(halfedge);
            }
        }

        let mut boundary_successor = HashMap::<HalfEdgeId, HalfEdgeId>::new();
        for &halfedge in &boundary_interior_halfedges {
            boundary_successor.insert(
                halfedge,
                find_next_boundary_interior_halfedge(&mesh, halfedge)?,
            );
        }

        let mut boundary_map = HashMap::<HalfEdgeId, HalfEdgeId>::new();
        for &halfedge in &boundary_interior_halfedges {
            let destination = edge_keys[halfedge.index()].1;
            let boundary_halfedge = HalfEdgeId(mesh.halfedges.len() as u32);
            let edge_id = EdgeId(mesh.edges.len() as u32);
            mesh.edges.push(EdgeRecord { halfedge });
            mesh.halfedges[halfedge.index()].twin = boundary_halfedge;
            mesh.halfedges[halfedge.index()].edge = edge_id;
            mesh.halfedges.push(HalfEdgeRecord {
                origin: VertexId(destination as u32),
                twin: halfedge,
                next: HalfEdgeId::INVALID,
                prev: HalfEdgeId::INVALID,
                face: FaceId::INVALID,
                edge: edge_id,
                data: LoopAttributes::default(),
            });
            if !mesh.vertices[destination].outgoing.is_valid() {
                mesh.vertices[destination].outgoing = boundary_halfedge;
            }
            boundary_map.insert(halfedge, boundary_halfedge);
        }

        for &halfedge in &boundary_interior_halfedges {
            let boundary_halfedge = boundary_map[&halfedge];
            let successor = boundary_successor[&halfedge];
            let next_boundary = boundary_map[&successor];
            mesh.halfedges[boundary_halfedge.index()].next = next_boundary;
            mesh.halfedges[next_boundary.index()].prev = boundary_halfedge;
        }

        let boundary_halfedges = boundary_interior_halfedges
            .iter()
            .map(|halfedge| boundary_map[halfedge])
            .collect::<Vec<_>>();
        let mut boundary_visited = vec![false; mesh.halfedges.len()];
        for boundary_halfedge in boundary_halfedges {
            if boundary_visited[boundary_halfedge.index()] {
                continue;
            }

            let face_id = FaceId(mesh.faces.len() as u32);
            mesh.faces.push(FaceRecord {
                halfedge: boundary_halfedge,
                kind: FaceKind::Boundary,
                data: FacePayload::default(),
            });

            let mut cursor = boundary_halfedge;
            let mut steps = 0usize;
            loop {
                steps += 1;
                if steps > mesh.halfedges.len().max(1) {
                    return Err(MeshError::Validation(
                        "boundary loop construction exceeded traversal guard".to_string(),
                    ));
                }
                boundary_visited[cursor.index()] = true;
                mesh.halfedges[cursor.index()].face = face_id;
                cursor = mesh.halfedges[cursor.index()].next;
                if cursor == boundary_halfedge {
                    break;
                }
            }
        }

        mesh.validate()?;
        Ok(mesh)
    }

    pub fn to_snapshot(&self) -> MeshSnapshot {
        let vertices = self
            .vertices
            .iter()
            .map(|record| record.data.clone())
            .collect();
        let faces = self
            .face_ids()
            .map(|face| {
                let vertices = self
                    .face_halfedges(face)
                    .expect("validated mesh must expose a face loop")
                    .map(|halfedge| self.halfedges[halfedge.index()].origin.index())
                    .collect::<Vec<_>>();
                let loops = self
                    .face_halfedges(face)
                    .expect("validated mesh must expose a face loop")
                    .map(|halfedge| self.halfedges[halfedge.index()].data.clone())
                    .collect::<Vec<_>>();
                PolygonFace {
                    vertices,
                    loops,
                    data: self.faces[face.index()].data,
                }
            })
            .collect();

        MeshSnapshot { vertices, faces }
    }

    pub fn validate(&self) -> Result<(), MeshError> {
        self.validate_storage()?;
        self.validate_manifold_conditions()?;
        Ok(())
    }

    pub fn is_manifold(&self) -> bool {
        self.validate_manifold_conditions().is_ok()
    }

    pub fn is_closed(&self) -> bool {
        self.boundary_face_count() == 0
    }

    pub fn boundary_loops(&self) -> Vec<Vec<HalfEdgeId>> {
        self.boundary_face_ids()
            .filter_map(|face| self.face_halfedges(face).ok())
            .map(|loop_halfedges| loop_halfedges.collect::<Vec<_>>())
            .collect()
    }

    pub fn connected_components(&self) -> Vec<Vec<FaceId>> {
        let mut visited = vec![false; self.face_count()];
        let mut components = Vec::new();

        for face in self.face_ids() {
            if visited[face.index()] {
                continue;
            }

            let mut queue = VecDeque::from([face]);
            let mut component = Vec::new();
            visited[face.index()] = true;

            while let Some(current) = queue.pop_front() {
                component.push(current);
                for halfedge in self.face_halfedges(current).into_iter().flatten() {
                    let twin = self.halfedges[halfedge.index()].twin;
                    let adjacent_face = self.halfedges[twin.index()].face;
                    if adjacent_face.index() < self.interior_face_count
                        && !visited[adjacent_face.index()]
                    {
                        visited[adjacent_face.index()] = true;
                        queue.push_back(adjacent_face);
                    }
                }
            }

            components.push(component);
        }

        components
    }

    pub fn has_degenerate_faces(&self) -> bool {
        self.face_ids().any(|face| {
            self.face_vertex_ids(face)
                .map(|vertices| {
                    let unique = vertices
                        .iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>();
                    unique.len() < 3 || self.face_area(face).unwrap_or(0.0) <= 1.0e-6
                })
                .unwrap_or(true)
        })
    }

    pub fn unit_cube() -> Result<Self, MeshError> {
        let vertices = vec![
            VertexPayload {
                position: Vec3::new(-0.5, -0.5, 0.5),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(0.5, -0.5, 0.5),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(0.5, 0.5, 0.5),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(-0.5, 0.5, 0.5),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(-0.5, -0.5, -0.5),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(0.5, -0.5, -0.5),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(0.5, 0.5, -0.5),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(-0.5, 0.5, -0.5),
                ..default()
            },
        ];
        let faces = vec![
            PolygonFace::new(vec![0, 1, 2, 3]),
            PolygonFace::new(vec![5, 4, 7, 6]),
            PolygonFace::new(vec![4, 0, 3, 7]),
            PolygonFace::new(vec![1, 5, 6, 2]),
            PolygonFace::new(vec![3, 2, 6, 7]),
            PolygonFace::new(vec![4, 5, 1, 0]),
        ];
        Self::from_polygon_faces(vertices, faces)
    }

    pub fn unit_triangle() -> Result<Self, MeshError> {
        Self::from_polygon_faces(
            vec![
                VertexPayload {
                    position: Vec3::new(-0.5, 0.0, 0.0),
                    ..default()
                },
                VertexPayload {
                    position: Vec3::new(0.5, 0.0, 0.0),
                    ..default()
                },
                VertexPayload {
                    position: Vec3::new(0.0, 0.75, 0.0),
                    ..default()
                },
            ],
            vec![PolygonFace::new(vec![0, 1, 2])],
        )
    }

    pub fn unit_quad() -> Result<Self, MeshError> {
        Self::from_polygon_faces(
            vec![
                VertexPayload {
                    position: Vec3::new(-0.5, -0.5, 0.0),
                    ..default()
                },
                VertexPayload {
                    position: Vec3::new(0.5, -0.5, 0.0),
                    ..default()
                },
                VertexPayload {
                    position: Vec3::new(0.5, 0.5, 0.0),
                    ..default()
                },
                VertexPayload {
                    position: Vec3::new(-0.5, 0.5, 0.0),
                    ..default()
                },
            ],
            vec![PolygonFace::new(vec![0, 1, 2, 3])],
        )
    }

    pub fn unit_tetrahedron() -> Result<Self, MeshError> {
        Self::from_polygon_faces(
            vec![
                VertexPayload {
                    position: Vec3::new(0.0, 0.75, 0.0),
                    ..default()
                },
                VertexPayload {
                    position: Vec3::new(-0.6, -0.35, 0.5),
                    ..default()
                },
                VertexPayload {
                    position: Vec3::new(0.6, -0.35, 0.5),
                    ..default()
                },
                VertexPayload {
                    position: Vec3::new(0.0, -0.35, -0.65),
                    ..default()
                },
            ],
            vec![
                PolygonFace::new(vec![0, 2, 1]),
                PolygonFace::new(vec![0, 1, 3]),
                PolygonFace::new(vec![0, 3, 2]),
                PolygonFace::new(vec![1, 2, 3]),
            ],
        )
    }

    fn validate_storage(&self) -> Result<(), MeshError> {
        for (index, vertex) in self.vertices.iter().enumerate() {
            if vertex.outgoing.is_valid() {
                let outgoing = self.halfedges.get(vertex.outgoing.index()).ok_or_else(|| {
                    MeshError::Validation(format!(
                        "vertex {index} points to missing outgoing half-edge {:?}",
                        vertex.outgoing
                    ))
                })?;
                if outgoing.origin != VertexId(index as u32) {
                    return Err(MeshError::Validation(format!(
                        "vertex {index} outgoing half-edge {:?} does not originate at the vertex",
                        vertex.outgoing
                    )));
                }
            }
        }

        for (index, edge) in self.edges.iter().enumerate() {
            let halfedge = self.halfedges.get(edge.halfedge.index()).ok_or_else(|| {
                MeshError::Validation(format!(
                    "edge {index} references missing half-edge {:?}",
                    edge.halfedge
                ))
            })?;
            if halfedge.edge != EdgeId(index as u32) {
                return Err(MeshError::Validation(format!(
                    "edge {index} is not mirrored on its representative half-edge"
                )));
            }
        }

        for (index, face) in self.faces.iter().enumerate() {
            let halfedge = self.halfedges.get(face.halfedge.index()).ok_or_else(|| {
                MeshError::Validation(format!(
                    "face {index} references missing half-edge {:?}",
                    face.halfedge
                ))
            })?;
            if halfedge.face != FaceId(index as u32) {
                return Err(MeshError::Validation(format!(
                    "face {index} half-edge {:?} points at face {:?}",
                    face.halfedge, halfedge.face
                )));
            }
        }

        for (index, halfedge) in self.halfedges.iter().enumerate() {
            let twin = self.halfedges.get(halfedge.twin.index()).ok_or_else(|| {
                MeshError::Validation(format!(
                    "half-edge {index} references missing twin {:?}",
                    halfedge.twin
                ))
            })?;
            if twin.twin != HalfEdgeId(index as u32) {
                return Err(MeshError::Validation(format!(
                    "half-edge {index} twin symmetry is broken"
                )));
            }
            if self.halfedges.get(halfedge.next.index()).is_none() {
                return Err(MeshError::Validation(format!(
                    "half-edge {index} next pointer is invalid"
                )));
            }
            if self.halfedges.get(halfedge.prev.index()).is_none() {
                return Err(MeshError::Validation(format!(
                    "half-edge {index} prev pointer is invalid"
                )));
            }
            let next = &self.halfedges[halfedge.next.index()];
            let prev = &self.halfedges[halfedge.prev.index()];
            if next.prev != HalfEdgeId(index as u32) {
                return Err(MeshError::Validation(format!(
                    "half-edge {index} next/prev linkage is broken"
                )));
            }
            if prev.next != HalfEdgeId(index as u32) {
                return Err(MeshError::Validation(format!(
                    "half-edge {index} prev/next linkage is broken"
                )));
            }
            if halfedge.origin.index() >= self.vertices.len() {
                return Err(MeshError::Validation(format!(
                    "half-edge {index} origin {:?} is out of range",
                    halfedge.origin
                )));
            }
            if halfedge.face.index() >= self.faces.len() {
                return Err(MeshError::Validation(format!(
                    "half-edge {index} face {:?} is out of range",
                    halfedge.face
                )));
            }
            if halfedge.edge.index() >= self.edges.len() {
                return Err(MeshError::Validation(format!(
                    "half-edge {index} edge {:?} is out of range",
                    halfedge.edge
                )));
            }
        }

        for face in self.face_ids().chain(self.boundary_face_ids()) {
            let loop_halfedges = self.face_halfedges(face)?.collect::<Vec<_>>();
            if loop_halfedges.len() < 3 {
                return Err(MeshError::Validation(format!(
                    "face {:?} contains fewer than 3 half-edges",
                    face
                )));
            }
            if loop_halfedges
                .iter()
                .any(|halfedge| self.halfedges[halfedge.index()].face != face)
            {
                return Err(MeshError::Validation(format!(
                    "face {:?} loop contains a half-edge with a mismatched face pointer",
                    face
                )));
            }
        }

        Ok(())
    }

    fn validate_manifold_conditions(&self) -> Result<(), MeshError> {
        for edge in self.edge_ids() {
            let (halfedge, twin) = self.edge_halfedges(edge)?;
            let face = self.halfedges[halfedge.index()].face;
            let twin_face = self.halfedges[twin.index()].face;
            if face == twin_face {
                return Err(MeshError::Validation(format!(
                    "edge {:?} has identical face ids on both sides",
                    edge
                )));
            }
        }

        for vertex in self.vertex_ids() {
            let scanned = self
                .halfedges
                .iter()
                .enumerate()
                .filter_map(|(index, halfedge)| {
                    (halfedge.origin == vertex).then_some(HalfEdgeId(index as u32))
                })
                .collect::<Vec<_>>();
            let traversed = self.vertex_outgoing_halfedges(vertex)?.collect::<Vec<_>>();
            if !scanned.is_empty() && traversed.len() != scanned.len() {
                return Err(MeshError::Validation(format!(
                    "vertex {:?} fan traversal visited {} half-edges but {} originate there",
                    vertex,
                    traversed.len(),
                    scanned.len()
                )));
            }
        }

        Ok(())
    }
}

fn find_next_boundary_interior_halfedge(
    mesh: &HalfEdgeMesh,
    boundary_halfedge: HalfEdgeId,
) -> Result<HalfEdgeId, MeshError> {
    let mut cursor = mesh.halfedges[boundary_halfedge.index()].prev;
    let mut steps = 0usize;
    loop {
        steps += 1;
        if steps > mesh.halfedges.len().max(1) {
            return Err(MeshError::Validation(
                "boundary successor search exceeded traversal guard".to_string(),
            ));
        }

        if !mesh.halfedges[cursor.index()].twin.is_valid() {
            return Ok(cursor);
        }

        let twin = mesh.halfedges[cursor.index()].twin;
        cursor = mesh.halfedges[twin.index()].prev;
    }
}

#[cfg(test)]
#[path = "topology_tests.rs"]
mod tests;
