use super::*;

#[test]
fn quad_face_geometry_helpers_are_consistent() {
    let mesh = HalfEdgeMesh::unit_quad().expect("quad");
    let face = mesh.face_ids().next().expect("face");

    assert!((mesh.face_area(face).expect("area") - 1.0).abs() <= 1.0e-5);
    assert_eq!(mesh.face_centroid(face).expect("centroid"), Vec3::ZERO);
    assert_eq!(mesh.face_normal(face).expect("normal"), Vec3::Z);
    assert_eq!(mesh.face_positions(face).expect("positions").len(), 4);
}

#[test]
fn cube_vertex_normal_matches_corner_diagonal() {
    let mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let normal = mesh
        .vertex_normal(VertexId(0))
        .expect("vertex normal")
        .normalize_or_zero();
    let expected = Vec3::new(-1.0, -1.0, 1.0).normalize();
    assert!(normal.distance(expected) <= 1.0e-5);
}
