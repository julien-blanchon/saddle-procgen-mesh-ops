use bevy::{
    asset::RenderAssetUsages,
    mesh::{Mesh, PrimitiveTopology},
    prelude::*,
};

use super::*;

#[test]
fn triangulated_cube_roundtrips_through_bevy_mesh() {
    let mut mesh = HalfEdgeMesh::unit_cube().expect("cube");
    mesh.triangulate_faces().expect("triangulate");
    mesh.recompute_normals().expect("normals");

    let exported = mesh.to_bevy_mesh().expect("export");
    let imported = HalfEdgeMesh::from_bevy_mesh(&exported).expect("import");

    assert_eq!(imported.vertex_count(), mesh.vertex_count());
    assert_eq!(imported.face_count(), mesh.face_count());
    assert!(imported.validate().is_ok());
}

#[test]
fn uv_attributes_survive_roundtrip() {
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

    let exported = mesh.to_bevy_mesh().expect("export");
    let imported = HalfEdgeMesh::from_bevy_mesh(&exported).expect("import");

    assert!(imported.has_loop_uvs());
    assert!(imported.face_ids().all(|face| {
        imported
            .face_loop_attributes(face)
            .expect("loops")
            .into_iter()
            .all(|attributes| attributes.uv.is_some())
    }));
}

#[test]
fn vertex_colors_survive_roundtrip() {
    let mut mesh = HalfEdgeMesh::unit_quad().expect("quad");
    for vertex in mesh.vertex_ids().collect::<Vec<_>>() {
        mesh.vertex_payload_mut(vertex)
            .expect("vertex")
            .color = Some(Vec4::new(vertex.index() as f32 / 4.0, 0.2, 0.8, 1.0));
    }
    mesh.triangulate_faces().expect("triangulate");
    mesh.recompute_normals().expect("normals");

    let exported = mesh.to_bevy_mesh().expect("export");
    let imported = HalfEdgeMesh::from_bevy_mesh(&exported).expect("import");

    assert!(imported.vertex_ids().all(|vertex| {
        imported
            .vertex_payload(vertex)
            .expect("vertex")
            .color
            .is_some()
    }));
}

#[test]
fn hard_normals_survive_position_weld_roundtrip() {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-1.0_f32, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ],
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![
            [0.0_f32, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 3, 4, 5]));

    let imported = HalfEdgeMesh::from_bevy_mesh(&mesh).expect("import");
    let exported = imported.to_bevy_mesh().expect("export");
    let roundtripped = HalfEdgeMesh::from_bevy_mesh(&exported).expect("roundtrip import");

    let seam_vertex = roundtripped
        .vertex_ids()
        .find(|vertex| {
            roundtripped
                .vertex_payload(*vertex)
                .map(|payload| payload.position == Vec3::new(-1.0, -1.0, 0.0))
                .unwrap_or(false)
        })
        .expect("welded seam vertex");

    let distinct_normals = roundtripped
        .vertex_outgoing_halfedges(seam_vertex)
        .expect("outgoing halfedges")
        .filter_map(|halfedge| {
            roundtripped
                .halfedge_loop_attributes(halfedge)
                .ok()
                .and_then(|attributes| attributes.normal)
                .map(|normal| [normal.x.to_bits(), normal.y.to_bits(), normal.z.to_bits()])
        })
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(distinct_normals.len(), 2);
}

#[test]
fn unsupported_topology_is_rejected() {
    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0]],
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1]));

    let error = HalfEdgeMesh::from_bevy_mesh(&mesh).expect_err("line list should fail");
    assert!(matches!(error, MeshError::UnsupportedPrimitiveTopology(_)));
}

#[test]
fn out_of_range_indices_are_rejected() {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 3]));

    let error = HalfEdgeMesh::from_bevy_mesh(&mesh).expect_err("invalid index should fail");
    assert!(matches!(error, MeshError::UnsupportedMesh(_)));
}

#[test]
fn missing_indices_are_rejected() {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
    );

    let error = HalfEdgeMesh::from_bevy_mesh(&mesh).expect_err("unindexed mesh should fail");
    assert_eq!(error, MeshError::MissingIndices);
}
