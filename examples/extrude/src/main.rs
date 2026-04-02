use bevy::prelude::*;
use saddle_procgen_mesh_ops::HalfEdgeMesh;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh_ops extrude".to_string(),
                resolution: (1280, 720).into(),
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
        Transform::from_xyz(3.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 18_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.7, 0.0)),
    ));

    let mut mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let face = mesh.face_ids().next().expect("cube face");
    mesh.extrude_faces(&[face], 0.35).expect("extrude");
    mesh.recompute_normals().expect("normals");

    commands.spawn((
        Name::new("Extruded Mesh"),
        Mesh3d(meshes.add(mesh.to_bevy_mesh().expect("bevy mesh"))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.78, 0.7, 0.56),
            perceptual_roughness: 0.82,
            ..default()
        })),
    ));
}
