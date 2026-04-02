use super::*;

#[test]
fn triangle_build_has_boundary_loop() {
    let mesh = HalfEdgeMesh::unit_triangle().expect("triangle");
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.boundary_face_count(), 1);
    assert_eq!(mesh.boundary_loops().len(), 1);
    assert!(!mesh.is_closed());
    assert!(mesh.is_manifold());
}

#[test]
fn cube_is_closed_manifold_and_connected() {
    let mesh = HalfEdgeMesh::unit_cube().expect("cube");
    assert_eq!(mesh.face_count(), 6);
    assert!(mesh.is_closed());
    assert!(mesh.is_manifold());
    assert_eq!(mesh.connected_components().len(), 1);
}

#[test]
fn disconnected_quads_report_two_components() {
    let vertices = vec![
        VertexPayload {
            position: Vec3::new(-2.0, -0.5, 0.0),
            ..default()
        },
        VertexPayload {
            position: Vec3::new(-1.0, -0.5, 0.0),
            ..default()
        },
        VertexPayload {
            position: Vec3::new(-1.0, 0.5, 0.0),
            ..default()
        },
        VertexPayload {
            position: Vec3::new(-2.0, 0.5, 0.0),
            ..default()
        },
        VertexPayload {
            position: Vec3::new(1.0, -0.5, 0.0),
            ..default()
        },
        VertexPayload {
            position: Vec3::new(2.0, -0.5, 0.0),
            ..default()
        },
        VertexPayload {
            position: Vec3::new(2.0, 0.5, 0.0),
            ..default()
        },
        VertexPayload {
            position: Vec3::new(1.0, 0.5, 0.0),
            ..default()
        },
    ];
    let mesh = HalfEdgeMesh::from_polygon_faces(
        vertices,
        vec![
            PolygonFace::new(vec![0, 1, 2, 3]),
            PolygonFace::new(vec![4, 5, 6, 7]),
        ],
    )
    .expect("disconnected quads");

    assert_eq!(mesh.connected_components().len(), 2);
    assert_eq!(mesh.boundary_loops().len(), 2);
}

#[test]
fn duplicate_directed_edge_is_rejected() {
    let vertices = vec![
        VertexPayload {
            position: Vec3::new(0.0, 0.0, 0.0),
            ..default()
        },
        VertexPayload {
            position: Vec3::new(1.0, 0.0, 0.0),
            ..default()
        },
        VertexPayload {
            position: Vec3::new(0.0, 1.0, 0.0),
            ..default()
        },
        VertexPayload {
            position: Vec3::new(0.0, 0.0, 1.0),
            ..default()
        },
    ];

    let error = HalfEdgeMesh::from_polygon_faces(
        vertices,
        vec![
            PolygonFace::new(vec![0, 1, 2]),
            PolygonFace::new(vec![0, 1, 3]),
        ],
    )
    .expect_err("duplicate directed edge should fail");

    assert!(matches!(error, MeshError::DuplicateDirectedEdge { .. }));
}
