use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_procgen_mesh_ops::{
    EditableMesh, HalfEdgeMesh, MeshEditCommand, MeshOpsPlugin, MeshOpsRequest, MeshOpsSystems,
    MeshOpsTarget,
};

#[derive(Resource, Clone, PartialEq)]
struct RuntimeConfig {
    interval_seconds: f32,
    levels_per_tick: u32,
    max_cycles: u32,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 1.2,
            levels_per_tick: 1,
            max_cycles: 3,
        }
    }
}

#[derive(Resource, Pane)]
#[pane(title = "Runtime Mesh Ops", position = "top-right")]
struct RuntimePane {
    #[pane(slider, min = 0.4, max = 2.4, step = 0.1)]
    interval_seconds: f32,
    #[pane(slider, min = 1.0, max = 2.0, step = 1.0)]
    levels_per_tick: u32,
    #[pane(slider, min = 1.0, max = 5.0, step = 1.0)]
    max_cycles: u32,
}

impl Default for RuntimePane {
    fn default() -> Self {
        Self {
            interval_seconds: 1.2,
            levels_per_tick: 1,
            max_cycles: 3,
        }
    }
}

#[derive(Resource)]
struct RuntimeState {
    timer: Timer,
    cycles: u32,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.2, TimerMode::Repeating),
            cycles: 0,
        }
    }
}

#[derive(Resource, Default)]
struct RuntimeStats {
    vertices: usize,
    faces: usize,
}

#[derive(Component)]
struct RuntimeDemo;

#[derive(Component)]
struct RuntimeOverlay;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.07)))
        .init_resource::<RuntimeConfig>()
        .init_resource::<RuntimePane>()
        .init_resource::<RuntimeState>()
        .init_resource::<RuntimeStats>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh_ops runtime".to_string(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PanePlugin)
        .register_pane::<RuntimePane>()
        .add_plugins(MeshOpsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                sync_pane_to_config,
                reset_demo_mesh.before(MeshOpsSystems::ProcessRequests),
                drive_runtime_demo.before(MeshOpsSystems::ProcessRequests),
                sync_stats.after(MeshOpsSystems::SyncMeshes),
                update_overlay.after(MeshOpsSystems::SyncMeshes),
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Runtime Camera"),
        Camera3d::default(),
        Transform::from_xyz(4.0, 3.5, 9.2).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Runtime Light"),
        DirectionalLight {
            illuminance: 20_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.95, 0.75, 0.0)),
    ));
    commands.spawn((
        Name::new("Runtime Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(16.0, 16.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.09, 0.11),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));
    spawn_demo_mesh(&mut commands, &mut meshes, &mut materials);
    commands.spawn((
        Name::new("Runtime Overlay"),
        RuntimeOverlay,
        Text::new("mesh_ops runtime"),
        Node {
            position_type: PositionType::Absolute,
            top: px(18),
            left: px(18),
            width: px(380),
            padding: UiRect::all(px(12)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.88)),
    ));
}

fn sync_pane_to_config(pane: Res<RuntimePane>, mut config: ResMut<RuntimeConfig>) {
    let next = RuntimeConfig {
        interval_seconds: pane.interval_seconds.clamp(0.25, 3.0),
        levels_per_tick: pane.levels_per_tick.clamp(1, 2),
        max_cycles: pane.max_cycles.max(1),
    };
    if *config != next {
        *config = next;
    }
}

fn reset_demo_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<RuntimeConfig>,
    mut state: ResMut<RuntimeState>,
    demos: Query<Entity, With<RuntimeDemo>>,
) {
    if !config.is_changed() {
        return;
    }

    for entity in &demos {
        commands.entity(entity).despawn();
    }
    spawn_demo_mesh(&mut commands, &mut meshes, &mut materials);
    state.timer = Timer::from_seconds(config.interval_seconds, TimerMode::Repeating);
    state.cycles = 0;
}

fn spawn_demo_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let handle = meshes.add(mesh.to_bevy_mesh().expect("runtime mesh"));
    commands.spawn((
        Name::new("Runtime Demo Mesh"),
        RuntimeDemo,
        Mesh3d(handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.65, 0.74, 0.88),
            perceptual_roughness: 0.82,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.1, 0.0),
        EditableMesh::new(mesh),
        MeshOpsTarget::new(handle),
    ));
}

fn drive_runtime_demo(
    time: Res<Time>,
    config: Res<RuntimeConfig>,
    mut state: ResMut<RuntimeState>,
    target: Option<Single<Entity, With<RuntimeDemo>>>,
    mut requests: MessageWriter<MeshOpsRequest>,
) {
    let Some(target) = target else {
        return;
    };
    state.timer.tick(time.delta());
    if state.cycles >= config.max_cycles {
        return;
    }
    if state.timer.just_finished() {
        state.cycles = state.cycles.saturating_add(1);
        requests.write(MeshOpsRequest {
            entity: *target,
            command: MeshEditCommand::SubdivideCatmullClark {
                levels: config.levels_per_tick,
            },
            prefer_async: true,
        });
    }
}

fn sync_stats(
    demo: Option<Single<&EditableMesh, With<RuntimeDemo>>>,
    mut stats: ResMut<RuntimeStats>,
) {
    let Some(demo) = demo else {
        return;
    };

    stats.vertices = demo.mesh.vertex_count();
    stats.faces = demo.mesh.face_count();
}

fn update_overlay(
    config: Res<RuntimeConfig>,
    state: Res<RuntimeState>,
    stats: Res<RuntimeStats>,
    mut overlay: Single<&mut Text, With<RuntimeOverlay>>,
) {
    if !config.is_changed() && !state.is_changed() && !stats.is_changed() {
        return;
    }

    **overlay = Text::new(format!(
        "mesh_ops runtime\ninterval: {:.1}s\nlevels per tick: {}\ncycles: {}/{}\nvertices/faces: {}/{}",
        config.interval_seconds,
        config.levels_per_tick,
        state.cycles,
        config.max_cycles,
        stats.vertices,
        stats.faces
    ));
}
