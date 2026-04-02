use bevy::prelude::*;

use crate::components::{EditableMesh, MeshOpsDebugSettings, MeshOpsDebugView};

pub fn draw_debug(
    mut gizmos: Gizmos,
    settings: Res<MeshOpsDebugSettings>,
    query: Query<(&EditableMesh, &GlobalTransform, &MeshOpsDebugView)>,
) {
    for (editable, transform, view) in &query {
        if !view.enabled {
            continue;
        }

        if view.draw_edges || view.draw_boundary_edges {
            for edge in editable.mesh.edge_ids() {
                let Ok((a, b)) = editable.mesh.edge_endpoints(edge) else {
                    continue;
                };
                let Ok(a_payload) = editable.mesh.vertex_payload(a) else {
                    continue;
                };
                let Ok(b_payload) = editable.mesh.vertex_payload(b) else {
                    continue;
                };
                let color = if editable.mesh.edge_is_boundary(edge).unwrap_or(false) {
                    if !view.draw_boundary_edges {
                        continue;
                    }
                    settings.boundary_edge_color
                } else {
                    if !view.draw_edges {
                        continue;
                    }
                    settings.edge_color
                };
                gizmos.line(
                    transform.transform_point(a_payload.position),
                    transform.transform_point(b_payload.position),
                    color,
                );
            }
        }

        if view.draw_face_normals {
            for face in editable.mesh.face_ids() {
                let Ok(center) = editable.mesh.face_centroid(face) else {
                    continue;
                };
                let Ok(normal) = editable.mesh.face_normal(face) else {
                    continue;
                };
                gizmos.line(
                    transform.transform_point(center),
                    transform.transform_point(center + normal * settings.normal_length),
                    settings.face_normal_color,
                );
            }
        }

        if view.draw_vertex_normals {
            for vertex in editable.mesh.vertex_ids() {
                let Ok(payload) = editable.mesh.vertex_payload(vertex) else {
                    continue;
                };
                let Ok(normal) = editable.mesh.vertex_normal(vertex) else {
                    continue;
                };
                gizmos.line(
                    transform.transform_point(payload.position),
                    transform.transform_point(payload.position + normal * settings.normal_length),
                    settings.vertex_normal_color,
                );
            }
        }
    }
}
