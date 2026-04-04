use std::collections::{BTreeSet, HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    EdgeId, FaceId, HalfEdgeMesh, LoopAttributes, MeshError, VertexId,
    attributes::{FacePayload, VertexPayload},
    topology::{MeshSnapshot, PolygonFace},
};

#[derive(Debug, Clone, Copy)]
struct EdgeUse {
    face: usize,
    edge_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LoopAttributeKey {
    uv: Option<[u32; 2]>,
    normal: Option<[u32; 3]>,
    tangent: Option<[u32; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum MeshUvProjectionMode {
    #[default]
    PlanarXY,
    PlanarXZ,
    PlanarYZ,
    Box,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct MeshUvProjection {
    pub mode: MeshUvProjectionMode,
    pub scale: Vec2,
    pub offset: Vec2,
}

impl Default for MeshUvProjection {
    fn default() -> Self {
        Self {
            mode: MeshUvProjectionMode::PlanarXY,
            scale: Vec2::ONE,
            offset: Vec2::ZERO,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct VertexColorPaintConfig {
    pub color: Vec4,
    pub blend: f32,
}

impl Default for VertexColorPaintConfig {
    fn default() -> Self {
        Self {
            color: Vec4::ONE,
            blend: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect, Default)]
pub struct MeshBridgeConfig {
    pub twist_offset: usize,
}

impl HalfEdgeMesh {
    pub fn add_face(&mut self, vertices: &[VertexId]) -> Result<FaceId, MeshError> {
        let mut snapshot = self.to_snapshot();
        let new_face = FaceId(snapshot.faces.len() as u32);
        snapshot.faces.push(PolygonFace {
            vertices: vertices.iter().map(|vertex| vertex.index()).collect(),
            loops: vec![LoopAttributes::default(); vertices.len()],
            data: FacePayload::default(),
        });
        *self = HalfEdgeMesh::from_snapshot(snapshot)?;
        Ok(new_face)
    }

    pub fn remove_face(&mut self, face: FaceId) -> Result<PolygonFace, MeshError> {
        let mut snapshot = self.to_snapshot();
        let removed = snapshot
            .faces
            .get(face.index())
            .cloned()
            .ok_or(MeshError::InvalidFace(face))?;
        snapshot.faces.remove(face.index());
        *self = HalfEdgeMesh::from_snapshot(snapshot)?;
        Ok(removed)
    }

    pub fn split_face(
        &mut self,
        face: FaceId,
        start: VertexId,
        end: VertexId,
    ) -> Result<EdgeId, MeshError> {
        let mut snapshot = self.to_snapshot();
        let source = snapshot
            .faces
            .get(face.index())
            .cloned()
            .ok_or(MeshError::InvalidFace(face))?;
        let start_index = source
            .vertices
            .iter()
            .position(|vertex| *vertex == start.index())
            .ok_or(MeshError::InvalidTopology(
                "split start vertex is not on the face",
            ))?;
        let end_index = source
            .vertices
            .iter()
            .position(|vertex| *vertex == end.index())
            .ok_or(MeshError::InvalidTopology(
                "split end vertex is not on the face",
            ))?;

        if start_index == end_index
            || (start_index + 1) % source.vertices.len() == end_index
            || (end_index + 1) % source.vertices.len() == start_index
        {
            return Err(MeshError::UnsupportedOperation {
                operation: "split_face",
                detail: "split vertices must be non-adjacent corners on the same face".to_string(),
            });
        }

        let first = polygon_path(&source, start_index, end_index);
        let second = polygon_path(&source, end_index, start_index);
        snapshot.faces[face.index()] = first;
        snapshot.faces.push(second);
        *self = HalfEdgeMesh::from_snapshot(snapshot)?;
        find_edge_by_endpoints(self, start.index(), end.index())
    }

    pub fn poke_face(&mut self, face: FaceId) -> Result<VertexId, MeshError> {
        let mut snapshot = self.to_snapshot();
        let source = snapshot
            .faces
            .get(face.index())
            .cloned()
            .ok_or(MeshError::InvalidFace(face))?;
        let face_payloads = source
            .vertices
            .iter()
            .map(|index| snapshot.vertices[*index].clone())
            .collect::<Vec<_>>();
        let center_vertex = snapshot.vertices.len();
        let mut payload = VertexPayload::average(&face_payloads);
        payload.position = face_payloads
            .iter()
            .fold(Vec3::ZERO, |acc, value| acc + value.position)
            / face_payloads.len() as f32;
        snapshot.vertices.push(payload);

        let center_loop = LoopAttributes::average(&source.loops);
        let mut replacement = Vec::with_capacity(source.vertices.len());
        for index in 0..source.vertices.len() {
            let next = (index + 1) % source.vertices.len();
            replacement.push(PolygonFace {
                vertices: vec![source.vertices[index], source.vertices[next], center_vertex],
                loops: vec![
                    source.loops.get(index).cloned().unwrap_or_default(),
                    source.loops.get(next).cloned().unwrap_or_default(),
                    center_loop.clone(),
                ],
                data: source.data,
            });
        }

        snapshot
            .faces
            .splice(face.index()..=face.index(), replacement);
        *self = HalfEdgeMesh::from_snapshot(snapshot)?;
        Ok(VertexId(center_vertex as u32))
    }

    pub fn flip_edge(&mut self, edge: EdgeId) -> Result<(), MeshError> {
        let (halfedge, twin) = self.edge_halfedges(edge)?;
        if self.edge_is_boundary(edge)? {
            return Err(MeshError::BoundaryOperation {
                operation: "flip_edge",
            });
        }

        let face_a = self.halfedge_face(halfedge)?;
        let face_b = self.halfedge_face(twin)?;
        let mut snapshot = self.to_snapshot();
        let polygon_a = snapshot
            .faces
            .get(face_a.index())
            .cloned()
            .ok_or(MeshError::InvalidFace(face_a))?;
        let polygon_b = snapshot
            .faces
            .get(face_b.index())
            .cloned()
            .ok_or(MeshError::InvalidFace(face_b))?;

        if polygon_a.vertices.len() != 3 || polygon_b.vertices.len() != 3 {
            return Err(MeshError::RequiresTriangleFaces {
                operation: "flip_edge",
            });
        }

        let a = self.halfedge_origin(halfedge)?.index();
        let b = self.halfedge_origin(twin)?.index();
        let opposite_a = triangle_opposite(&polygon_a, a, b).ok_or(MeshError::InvalidTopology(
            "triangle A does not match the target edge",
        ))?;
        let opposite_b = triangle_opposite(&polygon_b, b, a).ok_or(MeshError::InvalidTopology(
            "triangle B does not match the target edge",
        ))?;

        let average_normal = polygon_normal(&snapshot, &polygon_a.vertices)
            + polygon_normal(&snapshot, &polygon_b.vertices);

        let candidate_a_1 = vec![opposite_a, opposite_b, b];
        let candidate_b_1 = vec![opposite_b, opposite_a, a];
        let candidate_a_2 = vec![opposite_a, b, opposite_b];
        let candidate_b_2 = vec![opposite_b, a, opposite_a];

        let (new_a, new_b) = if polygon_normal(&snapshot, &candidate_a_1).dot(average_normal) >= 0.0
            && polygon_normal(&snapshot, &candidate_b_1).dot(average_normal) >= 0.0
        {
            (candidate_a_1, candidate_b_1)
        } else {
            (candidate_a_2, candidate_b_2)
        };

        snapshot.faces[face_a.index()] = PolygonFace {
            vertices: new_a.clone(),
            loops: new_a
                .iter()
                .map(|vertex| loop_for_vertex(&polygon_a, &polygon_b, *vertex))
                .collect(),
            data: polygon_a.data,
        };
        snapshot.faces[face_b.index()] = PolygonFace {
            vertices: new_b.clone(),
            loops: new_b
                .iter()
                .map(|vertex| loop_for_vertex(&polygon_b, &polygon_a, *vertex))
                .collect(),
            data: polygon_b.data,
        };

        *self = HalfEdgeMesh::from_snapshot(snapshot)?;
        Ok(())
    }

    pub fn split_edge(&mut self, edge: EdgeId) -> Result<VertexId, MeshError> {
        let (a, b) = self.edge_endpoints(edge)?;
        let created = split_edges_in_snapshot(self, &[(a.index(), b.index())], 0.5)?;
        Ok(VertexId(created[0] as u32))
    }

    pub fn collapse_edge(&mut self, edge: EdgeId) -> Result<VertexId, MeshError> {
        let (halfedge, twin) = self.edge_halfedges(edge)?;
        if self.edge_is_boundary(edge)? {
            return Err(MeshError::BoundaryOperation {
                operation: "collapse_edge",
            });
        }

        let keep = self.halfedge_origin(halfedge)?.index();
        let remove = self.halfedge_origin(twin)?.index();
        let mut snapshot = self.to_snapshot();
        let merged = snapshot.vertices[keep].lerp(&snapshot.vertices[remove], 0.5);
        snapshot.vertices[keep] = merged;

        for face in &mut snapshot.faces {
            let mut vertices = Vec::with_capacity(face.vertices.len());
            let mut loops = Vec::with_capacity(face.loops.len());

            for (index, vertex) in face.vertices.iter().copied().enumerate() {
                let vertex = if vertex == remove { keep } else { vertex };
                let loop_value = face.loops.get(index).cloned().unwrap_or_default();
                if vertices.last().copied() == Some(vertex) {
                    continue;
                }
                vertices.push(vertex);
                loops.push(loop_value);
            }

            if vertices.len() >= 2 && vertices.first() == vertices.last() {
                vertices.pop();
                loops.pop();
            }

            face.vertices = vertices;
            face.loops = loops;
        }

        cleanup_snapshot_faces(&mut snapshot);
        let remap = snapshot.compact_with_map();
        let collapsed =
            remap
                .get(keep)
                .and_then(|mapped| *mapped)
                .ok_or(MeshError::InvalidTopology(
                    "edge collapse removed the surviving vertex from every face",
                ))?;
        *self = HalfEdgeMesh::from_snapshot(snapshot)?;
        Ok(VertexId(collapsed as u32))
    }

    pub fn extrude_faces(&mut self, faces: &[FaceId], distance: f32) -> Result<(), MeshError> {
        if faces.is_empty() {
            return Err(MeshError::EmptySelection("extrude_faces"));
        }

        let selected = faces
            .iter()
            .map(|face| face.index())
            .collect::<HashSet<_>>();
        let snapshot = self.to_snapshot();
        let adjacency = build_edge_adjacency(&snapshot);
        for edge in adjacency.values() {
            let selected_count = edge
                .iter()
                .filter(|usage| selected.contains(&usage.face))
                .count();
            if selected_count > 1 {
                return Err(MeshError::UnsupportedOperation {
                    operation: "extrude_faces",
                    detail: "adjacent selected faces are rejected in pass 1".to_string(),
                });
            }
        }

        let mut output = MeshSnapshot {
            vertices: snapshot.vertices.clone(),
            faces: Vec::new(),
        };

        for (face_index, face) in snapshot.faces.iter().enumerate() {
            if !selected.contains(&face_index) {
                output.faces.push(face.clone());
                continue;
            }

            let normal = polygon_normal(&snapshot, &face.vertices);
            if normal == Vec3::ZERO {
                return Err(MeshError::DegenerateFace(FaceId(face_index as u32)));
            }
            let centroid = polygon_centroid(&snapshot, &face.vertices);
            let mut top_vertices = Vec::with_capacity(face.vertices.len());
            for vertex in &face.vertices {
                let mut payload = snapshot.vertices[*vertex].clone();
                payload.position += normal * distance;
                top_vertices.push(output.vertices.len());
                output.vertices.push(payload);
            }

            output.faces.push(PolygonFace {
                vertices: top_vertices.clone(),
                loops: face.loops.clone(),
                data: face.data,
            });

            for index in 0..face.vertices.len() {
                let next = (index + 1) % face.vertices.len();
                let old_a = face.vertices[index];
                let old_b = face.vertices[next];
                let new_a = top_vertices[index];
                let new_b = top_vertices[next];

                let candidate = vec![old_a, old_b, new_b, new_a];
                let desired =
                    (edge_midpoint(&snapshot, old_a, old_b) - centroid).normalize_or_zero();
                let vertices = if polygon_normal(&output, &candidate).dot(desired) >= 0.0 {
                    candidate
                } else {
                    vec![old_a, new_a, new_b, old_b]
                };
                output.faces.push(PolygonFace {
                    vertices,
                    loops: vec![LoopAttributes::default(); 4],
                    data: face.data,
                });
            }
        }

        *self = HalfEdgeMesh::from_snapshot(output)?;
        Ok(())
    }

    pub fn bevel_edges(&mut self, edges: &[EdgeId], width: f32) -> Result<(), MeshError> {
        if edges.len() != 1 {
            return Err(MeshError::UnsupportedOperation {
                operation: "bevel_edges",
                detail: "pass 1 supports a single two-triangle strip edge at a time".to_string(),
            });
        }

        let edge = edges[0];
        let (halfedge, twin) = self.edge_halfedges(edge)?;
        if self.edge_is_boundary(edge)? {
            return Err(MeshError::BoundaryOperation {
                operation: "bevel_edges",
            });
        }

        let face_a = self.halfedge_face(halfedge)?;
        let face_b = self.halfedge_face(twin)?;
        let snapshot = self.to_snapshot();
        let polygon_a = snapshot
            .faces
            .get(face_a.index())
            .cloned()
            .ok_or(MeshError::InvalidFace(face_a))?;
        let polygon_b = snapshot
            .faces
            .get(face_b.index())
            .cloned()
            .ok_or(MeshError::InvalidFace(face_b))?;

        if polygon_a.vertices.len() != 3 || polygon_b.vertices.len() != 3 {
            return Err(MeshError::RequiresTriangleFaces {
                operation: "bevel_edges",
            });
        }

        let u = self.halfedge_origin(halfedge)?.index();
        let v = self.halfedge_origin(twin)?.index();
        let a = triangle_opposite(&polygon_a, u, v)
            .ok_or(MeshError::InvalidTopology("triangle strip mismatch"))?;
        let b = triangle_opposite(&polygon_b, v, u)
            .ok_or(MeshError::InvalidTopology("triangle strip mismatch"))?;

        let pu = snapshot.vertices[u].position;
        let pv = snapshot.vertices[v].position;
        let pa = snapshot.vertices[a].position;
        let pb = snapshot.vertices[b].position;

        let mut output = snapshot.clone();
        let ua = output.vertices.len();
        output.vertices.push(VertexPayload {
            position: move_towards(pu, pa, width),
            ..snapshot.vertices[u].clone()
        });
        let va = output.vertices.len();
        output.vertices.push(VertexPayload {
            position: move_towards(pv, pa, width),
            ..snapshot.vertices[v].clone()
        });
        let ub = output.vertices.len();
        output.vertices.push(VertexPayload {
            position: move_towards(pu, pb, width),
            ..snapshot.vertices[u].clone()
        });
        let vb = output.vertices.len();
        output.vertices.push(VertexPayload {
            position: move_towards(pv, pb, width),
            ..snapshot.vertices[v].clone()
        });

        let average_normal = polygon_normal(&snapshot, &polygon_a.vertices)
            + polygon_normal(&snapshot, &polygon_b.vertices);

        let tri_a = orient_polygon(&output, vec![a, va, ua], average_normal);
        let tri_b = orient_polygon(&output, vec![b, ub, vb], average_normal);
        let quad = orient_polygon(&output, vec![ua, va, vb, ub], average_normal);

        output.faces = output
            .faces
            .into_iter()
            .enumerate()
            .filter_map(|(index, face)| {
                ((index != face_a.index()) && (index != face_b.index())).then_some(face)
            })
            .collect();
        output.faces.push(PolygonFace {
            vertices: tri_a,
            loops: vec![LoopAttributes::default(); 3],
            data: polygon_a.data,
        });
        output.faces.push(PolygonFace {
            vertices: tri_b,
            loops: vec![LoopAttributes::default(); 3],
            data: polygon_b.data,
        });
        output.faces.push(PolygonFace {
            vertices: quad,
            loops: vec![LoopAttributes::default(); 4],
            data: FacePayload::default(),
        });
        output.compact();
        *self = HalfEdgeMesh::from_snapshot(output)?;
        Ok(())
    }

    pub fn split_edge_ring(
        &mut self,
        edges: &[EdgeId],
        factor: f32,
    ) -> Result<Vec<VertexId>, MeshError> {
        if edges.is_empty() {
            return Err(MeshError::EmptySelection("split_edge_ring"));
        }

        let pairs = edges
            .iter()
            .map(|edge| self.edge_endpoints(*edge))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|(a, b)| (a.index(), b.index()))
            .collect::<Vec<_>>();

        let created = split_edges_in_snapshot(self, &pairs, factor)?;
        Ok(created
            .into_iter()
            .map(|index| VertexId(index as u32))
            .collect())
    }

    pub fn subdivide_catmull_clark(&mut self, levels: u32) -> Result<(), MeshError> {
        for _ in 0..levels {
            let snapshot = self.to_snapshot();
            let adjacency = build_edge_adjacency(&snapshot);
            let mut output = MeshSnapshot {
                vertices: snapshot.vertices.clone(),
                faces: Vec::new(),
            };

            let face_points = snapshot
                .faces
                .iter()
                .map(|face| {
                    let payloads = face
                        .vertices
                        .iter()
                        .map(|index| snapshot.vertices[*index].clone())
                        .collect::<Vec<_>>();
                    let mut payload = VertexPayload::average(&payloads);
                    payload.position = polygon_centroid(&snapshot, &face.vertices);
                    payload
                })
                .collect::<Vec<_>>();

            let edge_points = adjacency
                .iter()
                .map(|(key, uses)| {
                    let (a, b) = *key;
                    let point = if uses.len() == 2 {
                        let face_sum = uses.iter().fold(Vec3::ZERO, |acc, usage| {
                            acc + face_points[usage.face].position
                        });
                        (snapshot.vertices[a].position + snapshot.vertices[b].position + face_sum)
                            / 4.0
                    } else {
                        (snapshot.vertices[a].position + snapshot.vertices[b].position) * 0.5
                    };
                    let mut payload = snapshot.vertices[a].lerp(&snapshot.vertices[b], 0.5);
                    payload.position = point;
                    (*key, payload)
                })
                .collect::<HashMap<_, _>>();

            for vertex_index in 0..snapshot.vertices.len() {
                let connected_edges = adjacency
                    .iter()
                    .filter(|((a, b), _)| *a == vertex_index || *b == vertex_index)
                    .collect::<Vec<_>>();
                if connected_edges.is_empty() {
                    continue;
                }

                let boundary_neighbors = connected_edges
                    .iter()
                    .filter(|(_, uses)| uses.len() == 1)
                    .map(|((a, b), _)| if *a == vertex_index { *b } else { *a })
                    .collect::<Vec<_>>();

                let current = snapshot.vertices[vertex_index].position;
                output.vertices[vertex_index].position = if boundary_neighbors.len() >= 2 {
                    current * 0.75
                        + (snapshot.vertices[boundary_neighbors[0]].position
                            + snapshot.vertices[boundary_neighbors[1]].position)
                            * 0.125
                } else {
                    let face_average = adjacency
                        .iter()
                        .filter(|((a, b), _)| *a == vertex_index || *b == vertex_index)
                        .flat_map(|(_, uses)| {
                            uses.iter().map(|usage| face_points[usage.face].position)
                        })
                        .fold(Vec3::ZERO, |acc, value| acc + value)
                        / connected_edges.len() as f32;
                    let edge_average = connected_edges
                        .iter()
                        .map(|((a, b), _)| {
                            (snapshot.vertices[*a].position + snapshot.vertices[*b].position) * 0.5
                        })
                        .fold(Vec3::ZERO, |acc, value| acc + value)
                        / connected_edges.len() as f32;
                    let n = connected_edges.len() as f32;
                    (face_average + 2.0 * edge_average + (n - 3.0) * current) / n
                };
            }

            let mut edge_point_vertices = HashMap::<(usize, usize), usize>::new();
            for (key, payload) in edge_points {
                let index = output.vertices.len();
                output.vertices.push(payload);
                edge_point_vertices.insert(key, index);
            }

            let mut face_point_vertices = Vec::with_capacity(face_points.len());
            for payload in face_points {
                face_point_vertices.push(output.vertices.len());
                output.vertices.push(payload);
            }

            for (face_index, face) in snapshot.faces.iter().enumerate() {
                let count = face.vertices.len();
                for corner in 0..count {
                    let current = face.vertices[corner];
                    let next = face.vertices[(corner + 1) % count];
                    let previous = face.vertices[(corner + count - 1) % count];
                    let edge_next = edge_point_vertices[&edge_key(current, next)];
                    let edge_prev = edge_point_vertices[&edge_key(previous, current)];
                    output.faces.push(PolygonFace {
                        vertices: vec![
                            current,
                            edge_next,
                            face_point_vertices[face_index],
                            edge_prev,
                        ],
                        loops: vec![
                            face.loops.get(corner).cloned().unwrap_or_default(),
                            face.loops.get(corner).cloned().unwrap_or_default(),
                            LoopAttributes::average(&face.loops),
                            face.loops
                                .get((corner + count - 1) % count)
                                .cloned()
                                .unwrap_or_default(),
                        ],
                        data: face.data,
                    });
                }
            }

            *self = HalfEdgeMesh::from_snapshot(output)?;
        }
        Ok(())
    }

    pub fn subdivide_loop(&mut self, levels: u32) -> Result<(), MeshError> {
        for _ in 0..levels {
            if self.face_ids().any(|face| {
                self.face_vertex_ids(face)
                    .map(|vertices| vertices.len() != 3)
                    .unwrap_or(true)
            }) {
                return Err(MeshError::RequiresTriangleFaces {
                    operation: "subdivide_loop",
                });
            }

            let snapshot = self.to_snapshot();
            let adjacency = build_edge_adjacency(&snapshot);
            let mut output = MeshSnapshot {
                vertices: snapshot.vertices.clone(),
                faces: Vec::new(),
            };

            let mut vertex_neighbors = vec![BTreeSet::<usize>::new(); snapshot.vertices.len()];
            for &(a, b) in adjacency.keys() {
                vertex_neighbors[a].insert(b);
                vertex_neighbors[b].insert(a);
            }

            for (vertex_index, neighbors) in vertex_neighbors.iter().enumerate() {
                let current = snapshot.vertices[vertex_index].position;
                let boundary_neighbors = adjacency
                    .iter()
                    .filter(|((a, b), uses)| {
                        (*a == vertex_index || *b == vertex_index) && uses.len() == 1
                    })
                    .map(|((a, b), _)| if *a == vertex_index { *b } else { *a })
                    .collect::<Vec<_>>();
                output.vertices[vertex_index].position = if boundary_neighbors.len() >= 2 {
                    current * 0.75
                        + (snapshot.vertices[boundary_neighbors[0]].position
                            + snapshot.vertices[boundary_neighbors[1]].position)
                            * 0.125
                } else {
                    let n = neighbors.len() as f32;
                    let beta = if n == 3.0 {
                        3.0 / 16.0
                    } else {
                        3.0 / (8.0 * n)
                    };
                    let neighbor_sum = neighbors.iter().fold(Vec3::ZERO, |acc, neighbor| {
                        acc + snapshot.vertices[*neighbor].position
                    });
                    current * (1.0 - n * beta) + neighbor_sum * beta
                };
            }

            let mut edge_vertices = HashMap::<(usize, usize), usize>::new();
            for ((a, b), uses) in &adjacency {
                let new_position = if uses.len() == 2 {
                    let opposite = uses
                        .iter()
                        .map(|usage| {
                            let face = &snapshot.faces[usage.face];
                            face.vertices[(usage.edge_index + 2) % 3]
                        })
                        .collect::<Vec<_>>();
                    snapshot.vertices[*a].position * 0.375
                        + snapshot.vertices[*b].position * 0.375
                        + snapshot.vertices[opposite[0]].position * 0.125
                        + snapshot.vertices[opposite[1]].position * 0.125
                } else {
                    (snapshot.vertices[*a].position + snapshot.vertices[*b].position) * 0.5
                };

                let mut payload = snapshot.vertices[*a].lerp(&snapshot.vertices[*b], 0.5);
                payload.position = new_position;
                let vertex_index = output.vertices.len();
                output.vertices.push(payload);
                edge_vertices.insert((*a, *b), vertex_index);
            }

            for face in &snapshot.faces {
                let a = face.vertices[0];
                let b = face.vertices[1];
                let c = face.vertices[2];
                let ab = edge_vertices[&edge_key(a, b)];
                let bc = edge_vertices[&edge_key(b, c)];
                let ca = edge_vertices[&edge_key(c, a)];

                output.faces.push(triangle_face(
                    [a, ab, ca],
                    face.loops.first().cloned().unwrap_or_default(),
                    face.data,
                ));
                output.faces.push(triangle_face(
                    [ab, b, bc],
                    face.loops.get(1).cloned().unwrap_or_default(),
                    face.data,
                ));
                output.faces.push(triangle_face(
                    [ca, bc, c],
                    face.loops.get(2).cloned().unwrap_or_default(),
                    face.data,
                ));
                output.faces.push(PolygonFace {
                    vertices: vec![ab, bc, ca],
                    loops: vec![LoopAttributes::average(&face.loops); 3],
                    data: face.data,
                });
            }

            *self = HalfEdgeMesh::from_snapshot(output)?;
        }
        Ok(())
    }

    pub fn merge_vertices(&mut self, tolerance: f32) -> Result<u32, MeshError> {
        merge_vertices_impl(self, tolerance, false)
    }

    pub fn weld_by_position_and_attributes(&mut self, tolerance: f32) -> Result<u32, MeshError> {
        merge_vertices_impl(self, tolerance, true)
    }

    pub fn offset_vertices(
        &mut self,
        vertices: &[VertexId],
        offset: Vec3,
    ) -> Result<(), MeshError> {
        if vertices.is_empty() {
            return Err(MeshError::EmptySelection("offset_vertices"));
        }

        for &vertex in vertices {
            self.vertex_payload_mut(vertex)?.position += offset;
        }
        Ok(())
    }

    pub fn paint_vertices(
        &mut self,
        vertices: &[VertexId],
        config: &VertexColorPaintConfig,
    ) -> Result<(), MeshError> {
        if vertices.is_empty() {
            return Err(MeshError::EmptySelection("paint_vertices"));
        }

        let blend = config.blend.clamp(0.0, 1.0);
        for &vertex in vertices {
            let payload = self.vertex_payload_mut(vertex)?;
            payload.color = Some(
                payload
                    .color
                    .unwrap_or(config.color)
                    .lerp(config.color, blend),
            );
        }
        Ok(())
    }

    pub fn project_uvs(&mut self, projection: &MeshUvProjection) -> Result<(), MeshError> {
        let scale = Vec2::new(
            projection.scale.x.max(f32::EPSILON),
            projection.scale.y.max(f32::EPSILON),
        );
        for face in self.face_ids().collect::<Vec<_>>() {
            let face_mode = match projection.mode {
                MeshUvProjectionMode::Box => dominant_projection(self.face_normal(face)?),
                mode => mode,
            };
            let halfedges = self.face_halfedges(face)?.collect::<Vec<_>>();
            for halfedge in halfedges {
                let vertex = self.halfedge_origin(halfedge)?;
                let position = self.vertex_payload(vertex)?.position;
                self.halfedge_loop_attributes_mut(halfedge)?.uv = Some(project_position_to_uv(
                    position,
                    face_mode,
                    scale,
                    projection.offset,
                ));
            }
        }
        Ok(())
    }

    pub fn bridge_boundary_loops(
        &mut self,
        first_loop: usize,
        second_loop: usize,
        config: &MeshBridgeConfig,
    ) -> Result<(), MeshError> {
        let boundary_loops = self.boundary_loops();
        let Some(first) = boundary_loops.get(first_loop) else {
            return Err(MeshError::UnsupportedOperation {
                operation: "bridge_boundary_loops",
                detail: format!("boundary loop index {first_loop} is out of range"),
            });
        };
        let Some(second) = boundary_loops.get(second_loop) else {
            return Err(MeshError::UnsupportedOperation {
                operation: "bridge_boundary_loops",
                detail: format!("boundary loop index {second_loop} is out of range"),
            });
        };

        if first_loop == second_loop {
            return Err(MeshError::UnsupportedOperation {
                operation: "bridge_boundary_loops",
                detail: "bridge targets must refer to two distinct boundary loops".to_string(),
            });
        }
        if first.len() < 3 || second.len() < 3 || first.len() != second.len() {
            return Err(MeshError::UnsupportedOperation {
                operation: "bridge_boundary_loops",
                detail: "bridge pass 1 requires two boundary loops with matching corner counts"
                    .to_string(),
            });
        }

        let first_vertices = first
            .iter()
            .map(|halfedge| self.halfedge_origin(*halfedge).map(|vertex| vertex.index()))
            .collect::<Result<Vec<_>, _>>()?;
        let mut second_vertices = second
            .iter()
            .map(|halfedge| self.halfedge_origin(*halfedge).map(|vertex| vertex.index()))
            .collect::<Result<Vec<_>, _>>()?;
        second_vertices.reverse();

        let snapshot = self.to_snapshot();
        let twist = config.twist_offset % first_vertices.len();
        let mut output = snapshot.clone();

        for index in 0..first_vertices.len() {
            let next = (index + 1) % first_vertices.len();
            let b_index = (index + twist) % second_vertices.len();
            let b_next = (next + twist) % second_vertices.len();
            output.faces.push(PolygonFace {
                vertices: vec![
                    first_vertices[index],
                    first_vertices[next],
                    second_vertices[b_next],
                    second_vertices[b_index],
                ],
                loops: vec![LoopAttributes::default(); 4],
                data: FacePayload::default(),
            });
        }

        *self = HalfEdgeMesh::from_snapshot(output)?;
        Ok(())
    }

    pub fn recompute_normals(&mut self) -> Result<(), MeshError> {
        let normals = self
            .vertex_ids()
            .map(|vertex| self.vertex_normal(vertex))
            .collect::<Result<Vec<_>, _>>()?;

        let faces = self.face_ids().collect::<Vec<_>>();
        for face in faces {
            let halfedges = self.face_halfedges(face)?.collect::<Vec<_>>();
            for halfedge in halfedges {
                let vertex = self.halfedge_origin(halfedge)?;
                self.halfedge_loop_attributes_mut(halfedge)?.normal = Some(normals[vertex.index()]);
            }
        }
        Ok(())
    }

    pub fn recompute_tangents(&mut self) -> Result<(), MeshError> {
        let mut tangent_sum = vec![Vec3::ZERO; self.halfedge_count()];
        let mut bitangent_sum = vec![Vec3::ZERO; self.halfedge_count()];

        for face in self.face_ids() {
            let halfedges = self.face_halfedges(face)?.collect::<Vec<_>>();
            if halfedges.len() < 3 {
                continue;
            }

            for corner in 1..halfedges.len() - 1 {
                let tri = [halfedges[0], halfedges[corner], halfedges[corner + 1]];
                let positions = tri
                    .iter()
                    .map(|halfedge| {
                        self.vertex_payload(self.halfedge_origin(*halfedge)?)
                            .map(|payload| payload.position)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let loops = tri
                    .iter()
                    .map(|halfedge| self.halfedge_loop_attributes(*halfedge).cloned())
                    .collect::<Result<Vec<_>, _>>()?;
                let Some(uv0) = loops[0].uv else { continue };
                let Some(uv1) = loops[1].uv else { continue };
                let Some(uv2) = loops[2].uv else { continue };

                let edge1 = positions[1] - positions[0];
                let edge2 = positions[2] - positions[0];
                let delta_uv1 = uv1 - uv0;
                let delta_uv2 = uv2 - uv0;
                let determinant = delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x;
                if determinant.abs() <= 1.0e-6 {
                    continue;
                }

                let inverse = 1.0 / determinant;
                let tangent = (edge1 * delta_uv2.y - edge2 * delta_uv1.y) * inverse;
                let bitangent = (edge2 * delta_uv1.x - edge1 * delta_uv2.x) * inverse;
                for halfedge in tri {
                    tangent_sum[halfedge.index()] += tangent;
                    bitangent_sum[halfedge.index()] += bitangent;
                }
            }
        }

        for halfedge in self.halfedge_ids().collect::<Vec<_>>() {
            let attributes = self.halfedge_loop_attributes_mut(halfedge)?;
            let Some(normal) = attributes.normal else {
                attributes.tangent = None;
                continue;
            };

            let tangent = tangent_sum[halfedge.index()];
            if tangent == Vec3::ZERO {
                attributes.tangent = None;
                continue;
            }

            let tangent = (tangent - normal * normal.dot(tangent)).normalize_or_zero();
            let handedness = if normal.cross(tangent).dot(bitangent_sum[halfedge.index()]) < 0.0 {
                -1.0
            } else {
                1.0
            };
            attributes.tangent = Some(tangent.extend(handedness));
        }

        Ok(())
    }

    pub fn triangulate_faces(&mut self) -> Result<(), MeshError> {
        let mut snapshot = self.to_snapshot();
        snapshot.faces = snapshot
            .faces
            .into_iter()
            .flat_map(triangulate_polygon_face)
            .collect();
        *self = HalfEdgeMesh::from_snapshot(snapshot)?;
        Ok(())
    }

    pub fn separate_connected_components(&self) -> Result<Vec<HalfEdgeMesh>, MeshError> {
        let snapshot = self.to_snapshot();
        self.connected_components()
            .into_iter()
            .map(|component| {
                let mut subset = MeshSnapshot {
                    vertices: snapshot.vertices.clone(),
                    faces: component
                        .into_iter()
                        .map(|face| snapshot.faces[face.index()].clone())
                        .collect(),
                };
                subset.compact();
                HalfEdgeMesh::from_snapshot(subset)
            })
            .collect()
    }
}

fn triangle_face(
    vertices: [usize; 3],
    loop_attr: LoopAttributes,
    data: FacePayload,
) -> PolygonFace {
    PolygonFace {
        vertices: vertices.to_vec(),
        loops: vec![loop_attr; 3],
        data,
    }
}

fn merge_vertices_impl(
    mesh: &mut HalfEdgeMesh,
    tolerance: f32,
    require_payload_match: bool,
) -> Result<u32, MeshError> {
    let mut snapshot = mesh.to_snapshot();
    let corner_signatures =
        require_payload_match.then(|| build_vertex_corner_signatures(&snapshot));
    let mut representatives = (0..snapshot.vertices.len()).collect::<Vec<_>>();

    for index in 0..snapshot.vertices.len() {
        for candidate in 0..index {
            let compatible = snapshot.vertices[index]
                .position
                .distance(snapshot.vertices[candidate].position)
                <= tolerance
                && (!require_payload_match
                    || (snapshot.vertices[index].weight == snapshot.vertices[candidate].weight
                        && snapshot.vertices[index].tag == snapshot.vertices[candidate].tag
                        && corner_signatures
                            .as_ref()
                            .is_none_or(|signatures| signatures[index] == signatures[candidate])));
            if compatible {
                representatives[index] = representatives[candidate];
                break;
            }
        }
    }

    let unique_before = snapshot.vertices.len();
    for face in &mut snapshot.faces {
        for vertex in &mut face.vertices {
            *vertex = representatives[*vertex];
        }
    }
    cleanup_snapshot_faces(&mut snapshot);
    snapshot.compact();
    let merged = unique_before.saturating_sub(snapshot.vertices.len()) as u32;
    *mesh = HalfEdgeMesh::from_snapshot(snapshot)?;
    Ok(merged)
}

fn split_edges_in_snapshot(
    mesh: &mut HalfEdgeMesh,
    selected_pairs: &[(usize, usize)],
    factor: f32,
) -> Result<Vec<usize>, MeshError> {
    let factor = factor.clamp(0.05, 0.95);
    let mut snapshot = mesh.to_snapshot();
    let mut unique_pairs = Vec::<(usize, usize)>::new();
    let mut seen = HashSet::<(usize, usize)>::new();
    for &(a, b) in selected_pairs {
        let key = edge_key(a, b);
        if seen.insert(key) {
            unique_pairs.push((a, b));
        }
    }

    let mut created = Vec::with_capacity(unique_pairs.len());
    let mut split_vertices = HashMap::<(usize, usize), usize>::new();
    for (a, b) in unique_pairs {
        let mut payload = snapshot.vertices[a].lerp(&snapshot.vertices[b], factor);
        payload.position = snapshot.vertices[a]
            .position
            .lerp(snapshot.vertices[b].position, factor);
        let vertex_index = snapshot.vertices.len();
        snapshot.vertices.push(payload);
        split_vertices.insert(edge_key(a, b), vertex_index);
        created.push(vertex_index);
    }

    for face in &mut snapshot.faces {
        let count = face.vertices.len();
        let mut vertices = Vec::with_capacity(count * 2);
        let mut loops = Vec::with_capacity(count * 2);
        for index in 0..count {
            let current = face.vertices[index];
            let next = face.vertices[(index + 1) % count];
            let current_loop = face.loops.get(index).cloned().unwrap_or_default();
            let next_loop = face
                .loops
                .get((index + 1) % count)
                .cloned()
                .unwrap_or_default();
            vertices.push(current);
            loops.push(current_loop.clone());
            if let Some(midpoint) = split_vertices.get(&edge_key(current, next)) {
                vertices.push(*midpoint);
                loops.push(current_loop.lerp(&next_loop, 0.5));
            }
        }
        face.vertices = vertices;
        face.loops = loops;
    }

    *mesh = HalfEdgeMesh::from_snapshot(snapshot)?;
    Ok(created)
}

fn build_edge_adjacency(snapshot: &MeshSnapshot) -> HashMap<(usize, usize), Vec<EdgeUse>> {
    let mut adjacency = HashMap::<(usize, usize), Vec<EdgeUse>>::new();
    for (face_index, face) in snapshot.faces.iter().enumerate() {
        for edge_index in 0..face.vertices.len() {
            let a = face.vertices[edge_index];
            let b = face.vertices[(edge_index + 1) % face.vertices.len()];
            adjacency.entry(edge_key(a, b)).or_default().push(EdgeUse {
                face: face_index,
                edge_index,
            });
        }
    }
    adjacency
}

fn edge_key(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

fn triangle_opposite(face: &PolygonFace, a: usize, b: usize) -> Option<usize> {
    (0..face.vertices.len()).find_map(|index| {
        (face.vertices[index] == a && face.vertices[(index + 1) % face.vertices.len()] == b)
            .then_some(face.vertices[(index + 2) % face.vertices.len()])
    })
}

fn polygon_path(face: &PolygonFace, start: usize, end: usize) -> PolygonFace {
    let mut vertices = Vec::new();
    let mut loops = Vec::new();
    let mut cursor = start;
    loop {
        vertices.push(face.vertices[cursor]);
        loops.push(face.loops.get(cursor).cloned().unwrap_or_default());
        if cursor == end {
            break;
        }
        cursor = (cursor + 1) % face.vertices.len();
    }
    PolygonFace {
        vertices,
        loops,
        data: face.data,
    }
}

fn loop_for_vertex(
    primary: &PolygonFace,
    secondary: &PolygonFace,
    vertex: usize,
) -> LoopAttributes {
    primary
        .vertices
        .iter()
        .position(|candidate| *candidate == vertex)
        .and_then(|index| primary.loops.get(index).cloned())
        .or_else(|| {
            secondary
                .vertices
                .iter()
                .position(|candidate| *candidate == vertex)
                .and_then(|index| secondary.loops.get(index).cloned())
        })
        .unwrap_or_default()
}

fn edge_midpoint(snapshot: &MeshSnapshot, a: usize, b: usize) -> Vec3 {
    (snapshot.vertices[a].position + snapshot.vertices[b].position) * 0.5
}

fn polygon_centroid(snapshot: &MeshSnapshot, vertices: &[usize]) -> Vec3 {
    vertices.iter().fold(Vec3::ZERO, |acc, vertex| {
        acc + snapshot.vertices[*vertex].position
    }) / vertices.len() as f32
}

fn polygon_normal(snapshot: &MeshSnapshot, vertices: &[usize]) -> Vec3 {
    if vertices.len() < 3 {
        return Vec3::ZERO;
    }
    let positions = vertices
        .iter()
        .map(|index| snapshot.vertices[*index].position)
        .collect::<Vec<_>>();
    let mut normal = Vec3::ZERO;
    for index in 0..positions.len() {
        let current = positions[index];
        let next = positions[(index + 1) % positions.len()];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    normal.normalize_or_zero()
}

fn orient_polygon(
    snapshot: &MeshSnapshot,
    vertices: Vec<usize>,
    desired_normal: Vec3,
) -> Vec<usize> {
    if polygon_normal(snapshot, &vertices).dot(desired_normal) >= 0.0 {
        vertices
    } else {
        let mut reversed = vertices;
        reversed.reverse();
        reversed
    }
}

fn triangulate_polygon_face(face: PolygonFace) -> Vec<PolygonFace> {
    if face.vertices.len() <= 3 {
        return vec![face];
    }

    (1..face.vertices.len() - 1)
        .map(|corner| PolygonFace {
            vertices: vec![
                face.vertices[0],
                face.vertices[corner],
                face.vertices[corner + 1],
            ],
            loops: vec![
                face.loops.first().cloned().unwrap_or_default(),
                face.loops.get(corner).cloned().unwrap_or_default(),
                face.loops.get(corner + 1).cloned().unwrap_or_default(),
            ],
            data: face.data,
        })
        .collect()
}

fn cleanup_snapshot_faces(snapshot: &mut MeshSnapshot) {
    for face in &mut snapshot.faces {
        let mut vertices = Vec::with_capacity(face.vertices.len());
        let mut loops = Vec::with_capacity(face.loops.len());
        for (index, vertex) in face.vertices.iter().copied().enumerate() {
            if vertices.last().copied() == Some(vertex) {
                continue;
            }
            vertices.push(vertex);
            loops.push(face.loops.get(index).cloned().unwrap_or_default());
        }
        if vertices.len() >= 2 && vertices.first() == vertices.last() {
            vertices.pop();
            loops.pop();
        }
        face.vertices = vertices;
        face.loops = loops;
    }

    snapshot.faces.retain(|face| {
        face.vertices.len() >= 3
            && face.vertices.iter().copied().collect::<BTreeSet<_>>().len() >= 3
    });
}

fn move_towards(from: Vec3, to: Vec3, width: f32) -> Vec3 {
    let direction = to - from;
    let length = direction.length();
    if length <= 1.0e-6 {
        return from;
    }
    let factor = (width / length).clamp(0.08, 0.42);
    from + direction * factor
}

fn dominant_projection(normal: Vec3) -> MeshUvProjectionMode {
    let abs = normal.abs();
    if abs.z >= abs.x && abs.z >= abs.y {
        MeshUvProjectionMode::PlanarXY
    } else if abs.y >= abs.x {
        MeshUvProjectionMode::PlanarXZ
    } else {
        MeshUvProjectionMode::PlanarYZ
    }
}

fn project_position_to_uv(
    position: Vec3,
    mode: MeshUvProjectionMode,
    scale: Vec2,
    offset: Vec2,
) -> Vec2 {
    let projected = match mode {
        MeshUvProjectionMode::PlanarXY => Vec2::new(position.x, position.y),
        MeshUvProjectionMode::PlanarXZ => Vec2::new(position.x, position.z),
        MeshUvProjectionMode::PlanarYZ => Vec2::new(position.y, position.z),
        MeshUvProjectionMode::Box => Vec2::new(position.x, position.y),
    };
    projected * scale + offset
}

fn build_vertex_corner_signatures(snapshot: &MeshSnapshot) -> Vec<Vec<LoopAttributeKey>> {
    let mut signatures = vec![Vec::new(); snapshot.vertices.len()];
    for face in &snapshot.faces {
        for (corner, vertex) in face.vertices.iter().copied().enumerate() {
            let attributes = face.loops.get(corner).cloned().unwrap_or_default();
            signatures[vertex].push(LoopAttributeKey::from(&attributes));
        }
    }

    for signature in &mut signatures {
        signature.sort_unstable();
    }

    signatures
}

impl From<&LoopAttributes> for LoopAttributeKey {
    fn from(attributes: &LoopAttributes) -> Self {
        Self {
            uv: attributes.uv.map(|uv| [uv.x.to_bits(), uv.y.to_bits()]),
            normal: attributes
                .normal
                .map(|normal| [normal.x.to_bits(), normal.y.to_bits(), normal.z.to_bits()]),
            tangent: attributes.tangent.map(|tangent| {
                [
                    tangent.x.to_bits(),
                    tangent.y.to_bits(),
                    tangent.z.to_bits(),
                    tangent.w.to_bits(),
                ]
            }),
        }
    }
}

fn find_edge_by_endpoints(mesh: &HalfEdgeMesh, a: usize, b: usize) -> Result<EdgeId, MeshError> {
    mesh.edge_ids()
        .find(|edge| {
            mesh.edge_endpoints(*edge)
                .map(|(left, right)| {
                    let key = edge_key(left.index(), right.index());
                    key == edge_key(a, b)
                })
                .unwrap_or(false)
        })
        .ok_or(MeshError::InvalidTopology(
            "rebuilt mesh did not contain the expected edge",
        ))
}

#[cfg(test)]
#[path = "operations_tests.rs"]
mod tests;
