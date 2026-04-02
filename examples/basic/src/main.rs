use saddle_procgen_mesh_ops::HalfEdgeMesh;

fn main() -> Result<(), saddle_procgen_mesh_ops::MeshError> {
    let mut mesh = HalfEdgeMesh::unit_cube()?;
    let face = mesh.face_ids().next().expect("cube has a face");
    mesh.poke_face(face)?;
    mesh.recompute_normals()?;

    println!(
        "mesh_ops basic: vertices={} edges={} faces={} closed={}",
        mesh.vertex_count(),
        mesh.edge_count(),
        mesh.face_count(),
        mesh.is_closed()
    );
    Ok(())
}
