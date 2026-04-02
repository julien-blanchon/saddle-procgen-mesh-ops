use bevy::prelude::*;
use saddle_procgen_mesh_ops::{
    EditableMesh, HalfEdgeMesh, MeshEditCommand, MeshOpsPlugin, MeshOpsRequest, MeshOpsTarget,
};

#[derive(Component)]
struct RuntimeDemo;

#[derive(Resource, Default)]
struct RuntimeTimer(Timer);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh_ops runtime".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MeshOpsPlugin::default())
        .insert_resource(RuntimeTimer(Timer::from_seconds(1.2, TimerMode::Repeating)))
        .add_systems(Startup, setup)
        .add_systems(Update, drive_runtime_demo)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 2.8, 5.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 18_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.6, 0.0)),
    ));

    let mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let handle = meshes.add(mesh.to_bevy_mesh().expect("bevy mesh"));
    commands.spawn((
        Name::new("Runtime Demo Mesh"),
        RuntimeDemo,
        Mesh3d(handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.65, 0.74, 0.88),
            perceptual_roughness: 0.82,
            ..default()
        })),
        EditableMesh::new(mesh),
        MeshOpsTarget::new(handle),
    ));
}

fn drive_runtime_demo(
    time: Res<Time>,
    mut timer: ResMut<RuntimeTimer>,
    target: Single<Entity, With<RuntimeDemo>>,
    mut requests: MessageWriter<MeshOpsRequest>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        requests.write(MeshOpsRequest {
            entity: *target,
            command: MeshEditCommand::SubdivideCatmullClark { levels: 1 },
            prefer_async: true,
        });
    }
}
