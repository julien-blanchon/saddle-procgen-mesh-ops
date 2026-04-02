use bevy::platform::time::Instant;

use saddle_procgen_mesh_ops::HalfEdgeMesh;

fn main() -> Result<(), saddle_procgen_mesh_ops::MeshError> {
    let cube = HalfEdgeMesh::unit_cube()?;

    let start = Instant::now();
    let cube_export = cube.to_bevy_mesh()?;
    let export_time = start.elapsed();

    let start = Instant::now();
    let imported = HalfEdgeMesh::from_bevy_mesh(&cube_export)?;
    let import_time = start.elapsed();

    let mut catmull = HalfEdgeMesh::unit_cube()?;
    let start = Instant::now();
    catmull.subdivide_catmull_clark(2)?;
    let catmull_time = start.elapsed();

    let mut moderate = HalfEdgeMesh::unit_cube()?;
    moderate.subdivide_catmull_clark(2)?;
    moderate.triangulate_faces()?;
    let start = Instant::now();
    let moderate_export = moderate.to_bevy_mesh()?;
    let sync_time = start.elapsed();

    println!("mesh_ops perf");
    println!(
        "  cube export: {:?} (faces={}, vertices={}, bevy_vertices={})",
        export_time,
        cube.face_count(),
        cube.vertex_count(),
        cube_export.count_vertices(),
    );
    println!(
        "  cube import: {:?} (faces={}, vertices={})",
        import_time,
        imported.face_count(),
        imported.vertex_count(),
    );
    println!(
        "  catmull-clark x2: {:?} (faces={}, vertices={})",
        catmull_time,
        catmull.face_count(),
        catmull.vertex_count(),
    );
    println!(
        "  moderate export: {:?} (faces={}, vertices={}, bevy_vertices={})",
        sync_time,
        moderate.face_count(),
        moderate.vertex_count(),
        moderate_export.count_vertices(),
    );

    Ok(())
}
