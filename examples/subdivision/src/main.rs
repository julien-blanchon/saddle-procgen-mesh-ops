use bevy::prelude::*;
use saddle_procgen_mesh_ops::HalfEdgeMesh;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh_ops subdivision".to_string(),
                resolution: (1440, 820).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 9.0).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 20_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.9, 0.0)),
    ));

    let mut catmull = HalfEdgeMesh::unit_cube().expect("cube");
    catmull.subdivide_catmull_clark(1).expect("catmull");

    let mut loop_mesh = HalfEdgeMesh::unit_tetrahedron().expect("tetra");
    loop_mesh.subdivide_loop(1).expect("loop");
    loop_mesh.recompute_normals().expect("loop normals");

    let original = HalfEdgeMesh::unit_cube().expect("cube");

    for (label, mesh, x, color) in [
        ("Original", original, -3.0, Color::srgb(0.75, 0.35, 0.3)),
        ("Catmull-Clark", catmull, 0.0, Color::srgb(0.28, 0.7, 0.88)),
        ("Loop", loop_mesh, 3.0, Color::srgb(0.72, 0.78, 0.36)),
    ] {
        commands.spawn((
            Name::new(label),
            Mesh3d(meshes.add(mesh.to_bevy_mesh().expect("bevy mesh"))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.84,
                ..default()
            })),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
}
