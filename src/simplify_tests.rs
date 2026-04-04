use super::*;

#[test]
fn decimate_reduces_tetrahedron_face_count() {
    let mut mesh = HalfEdgeMesh::unit_tetrahedron().expect("tetrahedron");

    let collapses = mesh
        .decimate(&MeshDecimationConfig {
            target_face_count: 2,
            preserve_boundary: false,
            minimum_edge_length: 0.0,
            max_iterations: 32,
        })
        .expect("tetrahedron decimation should succeed");

    assert!(collapses > 0);
    assert!(mesh.face_count() < 4);
    assert!(mesh.validate().is_ok());
}

#[test]
fn lod_chain_builds_progressively_smaller_triangle_meshes() {
    let mut mesh = HalfEdgeMesh::unit_tetrahedron().expect("tetrahedron");
    mesh.subdivide_loop(2).expect("subdivide");

    let lods = mesh
        .build_lod_chain(&MeshLodConfig {
            level_count: 4,
            reduction_ratio: 0.55,
            minimum_face_count: 4,
            preserve_boundary: false,
            minimum_edge_length: 0.0,
            max_iterations_per_level: 64,
        })
        .expect("lod chain should succeed");

    assert!(lods.len() >= 2);
    for pair in lods.windows(2) {
        assert!(pair[1].face_count < pair[0].face_count);
        assert!(pair[1].mesh.validate().is_ok());
    }
}
