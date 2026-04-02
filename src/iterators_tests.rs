use super::*;

use crate::HalfEdgeMesh;

#[test]
fn quad_face_vertices_follow_face_order() {
    let mesh = HalfEdgeMesh::unit_quad().expect("quad");
    let face = mesh.face_ids().next().expect("face");
    let vertices = mesh
        .face_vertices(face)
        .expect("vertices")
        .collect::<Vec<_>>();
    assert_eq!(
        vertices,
        vec![VertexId(0), VertexId(1), VertexId(2), VertexId(3)]
    );
}

#[test]
fn cube_vertex_one_ring_visits_three_halfedges() {
    let mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let ring = mesh
        .vertex_outgoing_halfedges(VertexId(0))
        .expect("one ring")
        .collect::<Vec<_>>();
    assert_eq!(ring.len(), 3);
}
