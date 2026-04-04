use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    HalfEdgeMesh, MeshError,
    attributes::VertexPayload,
    topology::{MeshSnapshot, PolygonFace},
};

const BOOLEAN_RAY_DIRECTION: Vec3 = Vec3::new(0.863_868_4, 0.431_934_2, 0.259_160_52);
const BOOLEAN_INTERSECTION_EPSILON: f32 = 1.0e-5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum MeshBooleanOperation {
    Union,
    Intersection,
    Difference,
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub struct MeshBooleanConfig {
    pub voxel_size: f32,
    pub padding_voxels: u32,
    pub max_cells_per_axis: u32,
}

impl Default for MeshBooleanConfig {
    fn default() -> Self {
        Self {
            voxel_size: 0.12,
            padding_voxels: 1,
            max_cells_per_axis: 48,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Triangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
}

#[derive(Debug, Clone, Copy)]
struct GridSize {
    x: usize,
    y: usize,
    z: usize,
}

impl GridSize {
    fn volume(self) -> usize {
        self.x * self.y * self.z
    }
}

impl HalfEdgeMesh {
    pub fn boolean_with(
        &self,
        other: &HalfEdgeMesh,
        operation: MeshBooleanOperation,
        config: &MeshBooleanConfig,
    ) -> Result<HalfEdgeMesh, MeshError> {
        validate_boolean_config(config)?;

        if self.is_empty() || other.is_empty() {
            return Ok(match operation {
                MeshBooleanOperation::Union => {
                    if self.is_empty() {
                        other.clone()
                    } else {
                        self.clone()
                    }
                }
                MeshBooleanOperation::Intersection => HalfEdgeMesh::new(),
                MeshBooleanOperation::Difference => self.clone(),
            });
        }

        if !self.is_closed() || !other.is_closed() {
            return Err(MeshError::RequiresClosedMesh {
                operation: "boolean_with",
            });
        }

        let self_bounds = mesh_bounds(self)?;
        let other_bounds = mesh_bounds(other)?;
        let Some((min, max)) = boolean_bounds(self_bounds, other_bounds, operation, config) else {
            return Ok(HalfEdgeMesh::new());
        };
        let grid = build_grid_size(min, max, config)?;

        let self_triangles = mesh_triangles(self)?;
        let other_triangles = mesh_triangles(other)?;
        let self_bounds = expand_bounds(self_bounds, BOOLEAN_INTERSECTION_EPSILON);
        let other_bounds = expand_bounds(other_bounds, BOOLEAN_INTERSECTION_EPSILON);

        let mut occupied = vec![false; grid.volume()];
        for z in 0..grid.z {
            for y in 0..grid.y {
                for x in 0..grid.x {
                    let center = voxel_center(min, config.voxel_size, x, y, z);
                    let inside_a = point_within_bounds(center, self_bounds)
                        && point_inside_mesh(center, &self_triangles);
                    let inside_b = point_within_bounds(center, other_bounds)
                        && point_inside_mesh(center, &other_triangles);

                    occupied[grid_index(grid, x, y, z)] = match operation {
                        MeshBooleanOperation::Union => inside_a || inside_b,
                        MeshBooleanOperation::Intersection => inside_a && inside_b,
                        MeshBooleanOperation::Difference => inside_a && !inside_b,
                    };
                }
            }
        }

        build_voxel_surface(min, grid, &occupied, config.voxel_size)
    }

    pub fn apply_boolean(
        &mut self,
        other: &HalfEdgeMesh,
        operation: MeshBooleanOperation,
        config: &MeshBooleanConfig,
    ) -> Result<(), MeshError> {
        *self = self.boolean_with(other, operation, config)?;
        Ok(())
    }
}

fn validate_boolean_config(config: &MeshBooleanConfig) -> Result<(), MeshError> {
    if !config.voxel_size.is_finite() || config.voxel_size <= 0.0 {
        return Err(MeshError::InvalidBooleanConfig(
            "voxel_size must be finite and greater than zero".to_string(),
        ));
    }
    if config.max_cells_per_axis < 2 {
        return Err(MeshError::InvalidBooleanConfig(
            "max_cells_per_axis must be at least 2".to_string(),
        ));
    }
    Ok(())
}

fn mesh_bounds(mesh: &HalfEdgeMesh) -> Result<(Vec3, Vec3), MeshError> {
    let mut vertices = mesh.vertex_ids();
    let Some(first) = vertices.next() else {
        return Err(MeshError::UnsupportedOperation {
            operation: "boolean_with",
            detail: "boolean operands must contain at least one vertex".to_string(),
        });
    };
    let first_position = mesh.vertex_payload(first)?.position;
    let mut min = first_position;
    let mut max = first_position;
    for vertex in vertices {
        let position = mesh.vertex_payload(vertex)?.position;
        min = min.min(position);
        max = max.max(position);
    }
    Ok((min, max))
}

fn boolean_bounds(
    self_bounds: (Vec3, Vec3),
    other_bounds: (Vec3, Vec3),
    operation: MeshBooleanOperation,
    config: &MeshBooleanConfig,
) -> Option<(Vec3, Vec3)> {
    let padding = Vec3::splat(config.voxel_size * config.padding_voxels as f32);
    match operation {
        MeshBooleanOperation::Union => Some((
            self_bounds.0.min(other_bounds.0) - padding,
            self_bounds.1.max(other_bounds.1) + padding,
        )),
        MeshBooleanOperation::Difference => {
            Some((self_bounds.0 - padding, self_bounds.1 + padding))
        }
        MeshBooleanOperation::Intersection => {
            let min = self_bounds.0.max(other_bounds.0) - padding;
            let max = self_bounds.1.min(other_bounds.1) + padding;
            (min.cmple(max).all()).then_some((min, max))
        }
    }
}

fn build_grid_size(
    min: Vec3,
    max: Vec3,
    config: &MeshBooleanConfig,
) -> Result<GridSize, MeshError> {
    let extent = (max - min).max(Vec3::splat(config.voxel_size));
    let x = (extent.x / config.voxel_size).ceil() as usize;
    let y = (extent.y / config.voxel_size).ceil() as usize;
    let z = (extent.z / config.voxel_size).ceil() as usize;

    if x > config.max_cells_per_axis as usize
        || y > config.max_cells_per_axis as usize
        || z > config.max_cells_per_axis as usize
    {
        return Err(MeshError::BooleanGridTooDense {
            x: x as u32,
            y: y as u32,
            z: z as u32,
            max_axis: config.max_cells_per_axis,
        });
    }

    Ok(GridSize { x, y, z })
}

fn mesh_triangles(mesh: &HalfEdgeMesh) -> Result<Vec<Triangle>, MeshError> {
    let snapshot = mesh.to_snapshot();
    let mut triangles = Vec::new();
    for face in snapshot.faces {
        if face.vertices.len() < 3 {
            continue;
        }

        for corner in 1..face.vertices.len() - 1 {
            let a = snapshot.vertices[face.vertices[0]].position;
            let b = snapshot.vertices[face.vertices[corner]].position;
            let c = snapshot.vertices[face.vertices[corner + 1]].position;
            triangles.push(Triangle { a, b, c });
        }
    }
    Ok(triangles)
}

fn expand_bounds(bounds: (Vec3, Vec3), epsilon: f32) -> (Vec3, Vec3) {
    (
        bounds.0 - Vec3::splat(epsilon),
        bounds.1 + Vec3::splat(epsilon),
    )
}

fn point_within_bounds(point: Vec3, bounds: (Vec3, Vec3)) -> bool {
    point.cmpge(bounds.0).all() && point.cmple(bounds.1).all()
}

fn point_inside_mesh(point: Vec3, triangles: &[Triangle]) -> bool {
    let origin = point + BOOLEAN_RAY_DIRECTION * BOOLEAN_INTERSECTION_EPSILON;
    let mut hits = triangles
        .iter()
        .filter_map(|triangle| ray_triangle_intersection(origin, BOOLEAN_RAY_DIRECTION, *triangle))
        .collect::<Vec<_>>();

    hits.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    hits.dedup_by(|left, right| (*left - *right).abs() <= BOOLEAN_INTERSECTION_EPSILON);
    hits.len() % 2 == 1
}

fn ray_triangle_intersection(origin: Vec3, direction: Vec3, triangle: Triangle) -> Option<f32> {
    let edge_ab = triangle.b - triangle.a;
    let edge_ac = triangle.c - triangle.a;
    let p = direction.cross(edge_ac);
    let determinant = edge_ab.dot(p);
    if determinant.abs() <= BOOLEAN_INTERSECTION_EPSILON {
        return None;
    }

    let inverse_determinant = 1.0 / determinant;
    let offset = origin - triangle.a;
    let barycentric_u = offset.dot(p) * inverse_determinant;
    if !(0.0 - BOOLEAN_INTERSECTION_EPSILON..=1.0 + BOOLEAN_INTERSECTION_EPSILON)
        .contains(&barycentric_u)
    {
        return None;
    }

    let q = offset.cross(edge_ab);
    let barycentric_v = direction.dot(q) * inverse_determinant;
    if barycentric_v < -BOOLEAN_INTERSECTION_EPSILON
        || barycentric_u + barycentric_v > 1.0 + BOOLEAN_INTERSECTION_EPSILON
    {
        return None;
    }

    let distance = edge_ac.dot(q) * inverse_determinant;
    (distance > BOOLEAN_INTERSECTION_EPSILON).then_some(distance)
}

fn voxel_center(min: Vec3, voxel_size: f32, x: usize, y: usize, z: usize) -> Vec3 {
    min + voxel_size * Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5)
}

fn build_voxel_surface(
    min: Vec3,
    grid: GridSize,
    occupied: &[bool],
    voxel_size: f32,
) -> Result<HalfEdgeMesh, MeshError> {
    let mut snapshot = MeshSnapshot::default();
    let mut vertex_cache = HashMap::<(usize, usize, usize), usize>::new();

    for z in 0..grid.z {
        for y in 0..grid.y {
            for x in 0..grid.x {
                if !occupied[grid_index(grid, x, y, z)] {
                    continue;
                }

                for face in exposed_faces(grid, occupied, x, y, z) {
                    let corners = face.corner_indices(x, y, z);
                    let positions = corners.map(|corner| {
                        min + voxel_size
                            * Vec3::new(corner.0 as f32, corner.1 as f32, corner.2 as f32)
                    });
                    let mut vertex_indices = corners.map(|corner| {
                        *vertex_cache.entry(corner).or_insert_with(|| {
                            snapshot.vertices.push(VertexPayload {
                                position: min
                                    + voxel_size
                                        * Vec3::new(
                                            corner.0 as f32,
                                            corner.1 as f32,
                                            corner.2 as f32,
                                        ),
                                ..default()
                            });
                            snapshot.vertices.len() - 1
                        })
                    });

                    if triangle_normal(positions[0], positions[1], positions[2]).dot(face.normal())
                        < 0.0
                    {
                        vertex_indices.reverse();
                    }

                    snapshot
                        .faces
                        .push(PolygonFace::new(vertex_indices.to_vec()));
                }
            }
        }
    }

    if snapshot.faces.is_empty() {
        return Ok(HalfEdgeMesh::new());
    }

    HalfEdgeMesh::from_snapshot(snapshot)
}

fn grid_index(grid: GridSize, x: usize, y: usize, z: usize) -> usize {
    x + grid.x * (y + grid.y * z)
}

#[derive(Debug, Clone, Copy)]
enum ExposedFace {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl ExposedFace {
    fn normal(self) -> Vec3 {
        match self {
            Self::NegX => -Vec3::X,
            Self::PosX => Vec3::X,
            Self::NegY => -Vec3::Y,
            Self::PosY => Vec3::Y,
            Self::NegZ => -Vec3::Z,
            Self::PosZ => Vec3::Z,
        }
    }

    fn corner_indices(self, x: usize, y: usize, z: usize) -> [(usize, usize, usize); 4] {
        match self {
            Self::NegX => [(x, y, z), (x, y + 1, z), (x, y + 1, z + 1), (x, y, z + 1)],
            Self::PosX => [
                (x + 1, y, z),
                (x + 1, y, z + 1),
                (x + 1, y + 1, z + 1),
                (x + 1, y + 1, z),
            ],
            Self::NegY => [(x, y, z), (x, y, z + 1), (x + 1, y, z + 1), (x + 1, y, z)],
            Self::PosY => [
                (x, y + 1, z),
                (x + 1, y + 1, z),
                (x + 1, y + 1, z + 1),
                (x, y + 1, z + 1),
            ],
            Self::NegZ => [(x, y, z), (x + 1, y, z), (x + 1, y + 1, z), (x, y + 1, z)],
            Self::PosZ => [
                (x, y, z + 1),
                (x, y + 1, z + 1),
                (x + 1, y + 1, z + 1),
                (x + 1, y, z + 1),
            ],
        }
    }
}

fn exposed_faces(
    grid: GridSize,
    occupied: &[bool],
    x: usize,
    y: usize,
    z: usize,
) -> impl Iterator<Item = ExposedFace> {
    [
        (
            ExposedFace::NegX,
            x == 0 || !occupied[grid_index(grid, x - 1, y, z)],
        ),
        (
            ExposedFace::PosX,
            x + 1 >= grid.x || !occupied[grid_index(grid, x + 1, y, z)],
        ),
        (
            ExposedFace::NegY,
            y == 0 || !occupied[grid_index(grid, x, y - 1, z)],
        ),
        (
            ExposedFace::PosY,
            y + 1 >= grid.y || !occupied[grid_index(grid, x, y + 1, z)],
        ),
        (
            ExposedFace::NegZ,
            z == 0 || !occupied[grid_index(grid, x, y, z - 1)],
        ),
        (
            ExposedFace::PosZ,
            z + 1 >= grid.z || !occupied[grid_index(grid, x, y, z + 1)],
        ),
    ]
    .into_iter()
    .filter_map(|(face, visible)| visible.then_some(face))
}

fn triangle_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    (b - a).cross(c - a).normalize_or_zero()
}

#[cfg(test)]
#[path = "boolean_tests.rs"]
mod tests;
