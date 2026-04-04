use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct MeshOpsConfig {
    pub async_face_threshold: usize,
    pub allow_async_subdivision: bool,
    pub boolean_async_face_threshold: usize,
    pub allow_async_boolean_ops: bool,
    pub recompute_normals_after_topology_change: bool,
    pub recompute_tangents_after_topology_change: bool,
    pub refresh_aabb_on_sync: bool,
}

impl Default for MeshOpsConfig {
    fn default() -> Self {
        Self {
            async_face_threshold: 24,
            allow_async_subdivision: true,
            boolean_async_face_threshold: 48,
            allow_async_boolean_ops: true,
            recompute_normals_after_topology_change: true,
            recompute_tangents_after_topology_change: true,
            refresh_aabb_on_sync: true,
        }
    }
}
