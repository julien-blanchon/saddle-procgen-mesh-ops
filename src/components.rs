use bevy::prelude::*;

use crate::HalfEdgeMesh;

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct EditableMesh {
    pub mesh: HalfEdgeMesh,
    pub revision: u64,
    pub topology_dirty: bool,
}

impl EditableMesh {
    pub fn new(mesh: HalfEdgeMesh) -> Self {
        Self {
            mesh,
            revision: 0,
            topology_dirty: true,
        }
    }

    pub fn mark_changed(&mut self, topology_dirty: bool) {
        self.revision = self.revision.saturating_add(1);
        self.topology_dirty |= topology_dirty;
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct MeshOpsTarget {
    pub mesh_handle: Handle<Mesh>,
    pub synced_revision: u64,
    pub dirty: bool,
}

impl MeshOpsTarget {
    pub fn new(mesh_handle: Handle<Mesh>) -> Self {
        Self {
            mesh_handle,
            synced_revision: 0,
            dirty: true,
        }
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct MeshOpsDebugView {
    pub enabled: bool,
    pub draw_edges: bool,
    pub draw_boundary_edges: bool,
    pub draw_face_normals: bool,
    pub draw_vertex_normals: bool,
}

impl Default for MeshOpsDebugView {
    fn default() -> Self {
        Self {
            enabled: false,
            draw_edges: true,
            draw_boundary_edges: true,
            draw_face_normals: false,
            draw_vertex_normals: false,
        }
    }
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct MeshOpsDebugSettings {
    pub edge_color: Color,
    pub boundary_edge_color: Color,
    pub face_normal_color: Color,
    pub vertex_normal_color: Color,
    pub normal_length: f32,
}

impl Default for MeshOpsDebugSettings {
    fn default() -> Self {
        Self {
            edge_color: Color::srgb(0.85, 0.9, 1.0),
            boundary_edge_color: Color::srgb(1.0, 0.45, 0.25),
            face_normal_color: Color::srgb(0.2, 1.0, 0.55),
            vertex_normal_color: Color::srgb(0.95, 0.8, 0.2),
            normal_length: 0.24,
        }
    }
}
