use super::*;

fn offset_cube(offset: Vec3) -> HalfEdgeMesh {
    let mut mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let vertices = mesh.vertex_ids().collect::<Vec<_>>();
    mesh.offset_vertices(&vertices, offset).expect("offset");
    mesh
}

fn bounds(mesh: &HalfEdgeMesh) -> (Vec3, Vec3) {
    let mut positions = mesh
        .vertex_ids()
        .map(|vertex| mesh.vertex_payload(vertex).expect("vertex payload").position);
    let first = positions.next().expect("mesh has vertices");
    positions.fold((first, first), |(min, max), position| {
        (min.min(position), max.max(position))
    })
}

#[test]
fn union_boolean_expands_to_cover_both_operands() {
    let mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let other = offset_cube(Vec3::new(0.45, 0.3, 0.0));

    let result = mesh
        .boolean_with(
            &other,
            MeshBooleanOperation::Union,
            &MeshBooleanConfig {
                voxel_size: 0.1,
                padding_voxels: 1,
                max_cells_per_axis: 32,
            },
        )
        .expect("union boolean");

    let (min, max) = bounds(&result);
    assert!(result.is_closed());
    assert!(result.validate().is_ok());
    assert!(min.x <= -0.5);
    assert!(max.x >= 0.85);
    assert!(max.y >= 0.75);
}

#[test]
fn intersection_boolean_stays_inside_overlap_region() {
    let mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let other = offset_cube(Vec3::new(0.35, 0.35, 0.0));

    let result = mesh
        .boolean_with(
            &other,
            MeshBooleanOperation::Intersection,
            &MeshBooleanConfig {
                voxel_size: 0.1,
                padding_voxels: 1,
                max_cells_per_axis: 32,
            },
        )
        .expect("intersection boolean");

    let (min, max) = bounds(&result);
    assert!(result.is_closed());
    assert!(result.validate().is_ok());
    assert!(min.x >= -0.2);
    assert!(min.y >= -0.2);
    assert!(max.x <= 0.55);
    assert!(max.y <= 0.55);
}

#[test]
fn difference_boolean_keeps_source_bounds_and_creates_cut_surface() {
    let mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let other = offset_cube(Vec3::new(0.35, 0.35, 0.0));

    let result = mesh
        .boolean_with(
            &other,
            MeshBooleanOperation::Difference,
            &MeshBooleanConfig {
                voxel_size: 0.1,
                padding_voxels: 1,
                max_cells_per_axis: 32,
            },
        )
        .expect("difference boolean");

    let (min, max) = bounds(&result);
    assert!(result.is_closed());
    assert!(result.validate().is_ok());
    assert!(min.x <= -0.5);
    assert!(max.x >= 0.45);
    assert!(result.face_count() > HalfEdgeMesh::unit_cube().expect("cube").face_count());
}

#[test]
fn boolean_rejects_non_closed_operands() {
    let mesh = HalfEdgeMesh::unit_quad().expect("quad");
    let other = HalfEdgeMesh::unit_cube().expect("cube");
    let error = mesh
        .boolean_with(
            &other,
            MeshBooleanOperation::Union,
            &MeshBooleanConfig::default(),
        )
        .expect_err("open mesh should fail");

    assert!(matches!(error, MeshError::RequiresClosedMesh { .. }));
}

#[test]
fn boolean_grid_limits_are_reported_clearly() {
    let mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let other = offset_cube(Vec3::new(0.2, 0.0, 0.0));
    let error = mesh
        .boolean_with(
            &other,
            MeshBooleanOperation::Union,
            &MeshBooleanConfig {
                voxel_size: 0.01,
                padding_voxels: 1,
                max_cells_per_axis: 8,
            },
        )
        .expect_err("dense grid should fail");

    assert!(matches!(error, MeshError::BooleanGridTooDense { .. }));
}
