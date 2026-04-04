use super::*;
use proptest::prelude::*;

fn diamond_strip() -> HalfEdgeMesh {
    HalfEdgeMesh::from_polygon_faces(
        vec![
            VertexPayload {
                position: Vec3::new(0.0, 1.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(-1.0, 0.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(1.0, 0.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(0.0, -1.0, 0.0),
                ..default()
            },
        ],
        vec![
            PolygonFace::new(vec![1, 2, 0]),
            PolygonFace::new(vec![2, 1, 3]),
        ],
    )
    .expect("diamond strip")
}

fn edge_between(mesh: &HalfEdgeMesh, a: usize, b: usize) -> EdgeId {
    mesh.edge_ids()
        .find(|edge| {
            let (left, right) = mesh.edge_endpoints(*edge).expect("edge endpoints");
            (left.index() == a && right.index() == b) || (left.index() == b && right.index() == a)
        })
        .expect("edge between vertices")
}

fn separated_quads() -> HalfEdgeMesh {
    HalfEdgeMesh::from_polygon_faces(
        vec![
            VertexPayload {
                position: Vec3::new(-1.0, -1.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(1.0, -1.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(1.0, 1.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(-1.0, 1.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(-1.0, -1.0, 1.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(1.0, -1.0, 1.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(1.0, 1.0, 1.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(-1.0, 1.0, 1.0),
                ..default()
            },
        ],
        vec![
            PolygonFace::new(vec![0, 1, 2, 3]),
            PolygonFace::new(vec![7, 6, 5, 4]),
        ],
    )
    .expect("separated quads")
}

#[test]
fn flip_edge_changes_the_diagonal() {
    let mut mesh = diamond_strip();
    let shared = edge_between(&mesh, 1, 2);
    mesh.flip_edge(shared).expect("flip");

    assert!(mesh.validate().is_ok());
    assert!(mesh.edge_ids().any(|edge| {
        let (left, right) = mesh.edge_endpoints(edge).expect("edge");
        (left.index(), right.index()) == (0, 3) || (left.index(), right.index()) == (3, 0)
    }));
}

#[test]
fn boundary_edge_flip_is_rejected() {
    let mesh = HalfEdgeMesh::unit_quad().expect("quad");
    let error = {
        let mut mesh = mesh;
        mesh.flip_edge(edge_between(&mesh, 0, 1))
            .expect_err("boundary flip should fail")
    };
    assert!(matches!(error, MeshError::BoundaryOperation { .. }));
}

#[test]
fn split_edge_inserts_a_midpoint_vertex() {
    let mut mesh = HalfEdgeMesh::unit_quad().expect("quad");
    let vertex = mesh
        .split_edge(edge_between(&mesh, 0, 1))
        .expect("split edge");
    assert_eq!(mesh.vertex_count(), 5);
    assert!(vertex.index() < mesh.vertex_count());
    assert!(mesh.validate().is_ok());
}

#[test]
fn split_interior_edge_keeps_the_mesh_valid() {
    let mut mesh = diamond_strip();
    let vertex = mesh
        .split_edge(edge_between(&mesh, 1, 2))
        .expect("split shared edge");

    assert_eq!(mesh.vertex_count(), 5);
    assert!(vertex.index() < mesh.vertex_count());
    assert_eq!(mesh.face_count(), 2);
    assert!(mesh.validate().is_ok());
}

#[test]
fn collapse_edge_reduces_tetrahedron_vertex_count() {
    let mut mesh = HalfEdgeMesh::unit_tetrahedron().expect("tetra");
    mesh.collapse_edge(edge_between(&mesh, 0, 1))
        .expect("collapse");
    assert!(mesh.vertex_count() < 4);
    assert!(mesh.validate().is_ok());
}

#[test]
fn poke_face_turns_quad_into_four_triangles() {
    let mut mesh = HalfEdgeMesh::unit_quad().expect("quad");
    let face = mesh.face_ids().next().expect("face");
    mesh.poke_face(face).expect("poke");
    assert_eq!(mesh.face_count(), 4);
    assert_eq!(mesh.vertex_count(), 5);
}

#[test]
fn catmull_clark_cube_counts_match_expected() {
    let mut mesh = HalfEdgeMesh::unit_cube().expect("cube");
    mesh.subdivide_catmull_clark(1).expect("catmull");
    assert_eq!(mesh.vertex_count(), 26);
    assert_eq!(mesh.edge_count(), 48);
    assert_eq!(mesh.face_count(), 24);
}

#[test]
fn loop_subdivision_tetrahedron_counts_match_expected() {
    let mut mesh = HalfEdgeMesh::unit_tetrahedron().expect("tetra");
    mesh.subdivide_loop(1).expect("loop");
    assert_eq!(mesh.vertex_count(), 10);
    assert_eq!(mesh.edge_count(), 24);
    assert_eq!(mesh.face_count(), 16);
}

#[test]
fn single_face_extrusion_builds_top_and_side_faces() {
    let mut mesh = HalfEdgeMesh::unit_quad().expect("quad");
    let face = mesh.face_ids().next().expect("face");
    mesh.extrude_faces(&[face], 0.5).expect("extrude");
    assert_eq!(mesh.face_count(), 5);
    assert_eq!(mesh.vertex_count(), 8);
}

#[test]
fn isolated_strip_bevel_replaces_two_triangles_with_three_faces() {
    let mut mesh = diamond_strip();
    mesh.bevel_edges(&[edge_between(&mesh, 1, 2)], 0.18)
        .expect("bevel");
    assert_eq!(mesh.face_count(), 3);
    assert_eq!(mesh.vertex_count(), 6);
    assert!(mesh.validate().is_ok());
}

#[test]
fn merge_vertices_respects_tolerance() {
    let mut mesh = HalfEdgeMesh::from_polygon_faces(
        vec![
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
                position: Vec3::new(0.0, 0.0, 0.001),
                ..default()
            },
        ],
        vec![
            PolygonFace::new(vec![0, 1, 2]),
            PolygonFace::new(vec![3, 2, 1]),
        ],
    )
    .expect("duplicated vertices");

    let merged = mesh.merge_vertices(0.01).expect("merge");
    assert!(merged > 0);
    assert_eq!(mesh.vertex_count(), 3);
}

#[test]
fn weld_by_position_and_attributes_preserves_uv_seams() {
    let mut face_a = PolygonFace::new(vec![0, 1, 2]);
    face_a.loops = vec![
        LoopAttributes {
            uv: Some(Vec2::new(0.0, 0.0)),
            ..default()
        },
        LoopAttributes {
            uv: Some(Vec2::new(1.0, 0.0)),
            ..default()
        },
        LoopAttributes {
            uv: Some(Vec2::new(1.0, 1.0)),
            ..default()
        },
    ];

    let mut face_b = PolygonFace::new(vec![3, 4, 5]);
    face_b.loops = vec![
        LoopAttributes {
            uv: Some(Vec2::new(0.25, 0.0)),
            ..default()
        },
        LoopAttributes {
            uv: Some(Vec2::new(1.0, 1.0)),
            ..default()
        },
        LoopAttributes {
            uv: Some(Vec2::new(0.0, 1.0)),
            ..default()
        },
    ];

    let mesh = HalfEdgeMesh::from_polygon_faces(
        vec![
            VertexPayload {
                position: Vec3::new(-1.0, -1.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(1.0, -1.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(1.0, 1.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(-1.0, -1.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(1.0, 1.0, 0.0),
                ..default()
            },
            VertexPayload {
                position: Vec3::new(-1.0, 1.0, 0.0),
                ..default()
            },
        ],
        vec![face_a, face_b],
    )
    .expect("mesh with duplicated seam vertices");

    let mut position_only = mesh.clone();
    let mut attribute_aware = mesh;

    assert_eq!(
        position_only.merge_vertices(0.001).expect("position merge"),
        2
    );
    assert_eq!(position_only.vertex_count(), 4);

    let error = attribute_aware
        .weld_by_position_and_attributes(0.001)
        .expect_err("attribute-aware weld should reject the surviving UV seam");
    assert!(matches!(error, MeshError::Validation(_)));
    assert_eq!(attribute_aware.vertex_count(), 6);
    assert!(attribute_aware.validate().is_ok());
}

#[test]
fn recompute_tangents_populates_loop_tangents() {
    let mut mesh = HalfEdgeMesh::unit_quad().expect("quad");
    let face = mesh.face_ids().next().expect("face");
    let halfedges = mesh
        .face_halfedges(face)
        .expect("halfedges")
        .collect::<Vec<_>>();
    let uvs = [
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
    ];
    for (halfedge, uv) in halfedges.iter().zip(uvs) {
        mesh.halfedge_loop_attributes_mut(*halfedge)
            .expect("loop")
            .uv = Some(uv);
    }
    mesh.triangulate_faces().expect("triangulate");
    mesh.recompute_normals().expect("normals");
    mesh.recompute_tangents().expect("tangents");

    assert!(mesh.face_ids().all(|face| {
        mesh.face_halfedges(face)
            .expect("halfedges")
            .all(|halfedge| {
                mesh.halfedge_loop_attributes(halfedge)
                    .expect("loop")
                    .tangent
                    .is_some()
            })
    }));
}

#[test]
fn planar_projection_assigns_loop_uvs() {
    let mut mesh = HalfEdgeMesh::unit_quad().expect("quad");
    mesh.project_uvs(&MeshUvProjection {
        mode: MeshUvProjectionMode::PlanarXY,
        scale: Vec2::new(0.5, 0.25),
        offset: Vec2::new(1.0, -0.5),
    })
    .expect("project uvs");

    let face = mesh.face_ids().next().expect("face");
    let uvs = mesh
        .face_loop_attributes(face)
        .expect("loop attributes")
        .into_iter()
        .map(|attributes| attributes.uv.expect("uv"))
        .collect::<Vec<_>>();
    assert!(uvs.contains(&Vec2::new(0.75, -0.625)));
    assert!(uvs.contains(&Vec2::new(1.25, -0.375)));
}

#[test]
fn vertex_painting_blends_existing_color() {
    let mut mesh = HalfEdgeMesh::unit_quad().expect("quad");
    mesh.vertex_payload_mut(VertexId(0)).expect("vertex").color = Some(Vec4::new(0.2, 0.2, 0.2, 1.0));
    mesh.paint_vertices(
        &[VertexId(0)],
        &VertexColorPaintConfig {
            color: Vec4::new(1.0, 0.4, 0.1, 1.0),
            blend: 0.5,
        },
    )
    .expect("paint");

    let painted = mesh.vertex_payload(VertexId(0)).expect("vertex");
    assert_eq!(painted.color, Some(Vec4::new(0.6, 0.3, 0.15, 1.0)));
}

#[test]
fn bridging_two_boundary_loops_creates_side_faces() {
    let mut mesh = separated_quads();
    assert_eq!(mesh.boundary_loops().len(), 2);

    mesh.bridge_boundary_loops(0, 1, &MeshBridgeConfig::default())
        .expect("bridge loops");

    assert_eq!(mesh.face_count(), 6);
    assert!(mesh.is_closed());
    assert!(mesh.validate().is_ok());
}

proptest! {
    #[test]
    fn successful_edit_sequences_remain_valid(ops in prop::collection::vec(0u8..5, 1..8)) {
        let mut mesh = HalfEdgeMesh::unit_cube().expect("cube");

        for op in ops {
            let result: Result<(), MeshError> = match op {
                0 => {
                    let face = mesh.face_ids().next();
                    face.map_or(Ok(()), |face| mesh.poke_face(face).map(|_| ()))
                }
                1 => {
                    let edge = mesh.edge_ids().next();
                    edge.map_or(Ok(()), |edge| mesh.split_edge(edge).map(|_| ()))
                }
                2 => mesh.triangulate_faces(),
                3 => mesh.merge_vertices(0.001).map(|_| ()),
                _ => mesh.recompute_normals(),
            };

            if result.is_ok() {
                prop_assert!(mesh.validate().is_ok());
            }
        }
    }
}
