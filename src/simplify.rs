use std::collections::HashSet;

use bevy::prelude::*;

use crate::{EdgeId, HalfEdgeMesh, MeshError};

#[derive(Debug, Clone, Reflect)]
pub struct MeshDecimationConfig {
    pub target_face_count: usize,
    pub preserve_boundary: bool,
    pub minimum_edge_length: f32,
    pub max_iterations: usize,
}

impl Default for MeshDecimationConfig {
    fn default() -> Self {
        Self {
            target_face_count: 12,
            preserve_boundary: true,
            minimum_edge_length: 0.0,
            max_iterations: 256,
        }
    }
}

#[derive(Debug, Clone, Reflect)]
pub struct MeshLodConfig {
    pub level_count: u32,
    pub reduction_ratio: f32,
    pub minimum_face_count: usize,
    pub preserve_boundary: bool,
    pub minimum_edge_length: f32,
    pub max_iterations_per_level: usize,
}

impl Default for MeshLodConfig {
    fn default() -> Self {
        Self {
            level_count: 3,
            reduction_ratio: 0.6,
            minimum_face_count: 6,
            preserve_boundary: true,
            minimum_edge_length: 0.0,
            max_iterations_per_level: 256,
        }
    }
}

#[derive(Debug, Clone, Reflect)]
pub struct MeshLodLevel {
    pub level: u32,
    pub face_count: usize,
    pub edge_count: usize,
    pub vertex_count: usize,
    pub mesh: HalfEdgeMesh,
}

impl HalfEdgeMesh {
    pub fn decimate(&mut self, config: &MeshDecimationConfig) -> Result<u32, MeshError> {
        let target_face_count = config.target_face_count.max(1);
        if self.face_count() <= target_face_count {
            return Ok(0);
        }

        let mut collapses = 0_u32;
        let mut attempted = HashSet::<usize>::new();
        let iteration_limit = config.max_iterations.max(1);

        for _ in 0..iteration_limit {
            if self.face_count() <= target_face_count {
                break;
            }

            let Some(candidate) = shortest_collapsible_edge(
                self,
                config.preserve_boundary,
                config.minimum_edge_length,
                &attempted,
            ) else {
                break;
            };

            let previous_faces = self.face_count();
            let mut simplified = self.clone();
            match simplified.collapse_edge(candidate) {
                Ok(_)
                    if simplified.face_count() < previous_faces
                        && simplified.validate().is_ok() =>
                {
                    *self = simplified;
                    collapses = collapses.saturating_add(1);
                    attempted.clear();
                }
                _ => {
                    attempted.insert(candidate.index());
                }
            }
        }

        if collapses == 0 && self.face_count() > target_face_count {
            return Err(MeshError::UnsupportedOperation {
                operation: "decimate",
                detail: "no valid edge collapses were available for the requested target"
                    .to_string(),
            });
        }

        Ok(collapses)
    }

    pub fn build_lod_chain(&self, config: &MeshLodConfig) -> Result<Vec<MeshLodLevel>, MeshError> {
        let mut levels = Vec::with_capacity(config.level_count.max(1) as usize);
        levels.push(MeshLodLevel {
            level: 0,
            face_count: self.face_count(),
            edge_count: self.edge_count(),
            vertex_count: self.vertex_count(),
            mesh: self.clone(),
        });

        let reduction_ratio = config.reduction_ratio.clamp(0.1, 0.95);
        let mut current = self.clone();

        for level in 1..config.level_count {
            if current.face_count() <= config.minimum_face_count.max(1) {
                break;
            }

            let target = ((current.face_count() as f32 * reduction_ratio).round() as usize)
                .max(config.minimum_face_count.max(1));
            let before_face_count = current.face_count();
            let decimation = MeshDecimationConfig {
                target_face_count: target,
                preserve_boundary: config.preserve_boundary,
                minimum_edge_length: config.minimum_edge_length,
                max_iterations: config.max_iterations_per_level,
            };

            match current.decimate(&decimation) {
                Ok(_) if current.face_count() < before_face_count => {
                    levels.push(MeshLodLevel {
                        level,
                        face_count: current.face_count(),
                        edge_count: current.edge_count(),
                        vertex_count: current.vertex_count(),
                        mesh: current.clone(),
                    });
                }
                Ok(_) => break,
                Err(error) if levels.len() > 1 => {
                    let _ = error;
                    break;
                }
                Err(error) => return Err(error),
            }
        }

        Ok(levels)
    }
}

fn shortest_collapsible_edge(
    mesh: &HalfEdgeMesh,
    preserve_boundary: bool,
    minimum_edge_length: f32,
    attempted: &HashSet<usize>,
) -> Option<EdgeId> {
    mesh.edge_ids()
        .filter(|edge| !attempted.contains(&edge.index()))
        .filter_map(|edge| {
            if preserve_boundary && mesh.edge_is_boundary(edge).ok()? {
                return None;
            }

            let (left, right) = mesh.edge_endpoints(edge).ok()?;
            let left_position = mesh.vertex_payload(left).ok()?.position;
            let right_position = mesh.vertex_payload(right).ok()?.position;
            let length = left_position.distance(right_position);
            (length >= minimum_edge_length.max(0.0)).then_some((length, edge))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, edge)| edge)
}

#[cfg(test)]
#[path = "simplify_tests.rs"]
mod tests;
