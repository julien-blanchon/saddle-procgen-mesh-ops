use bevy::platform::time::Instant;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_procgen_mesh_ops::{HalfEdgeMesh, MeshLodConfig};

#[derive(Resource, Clone, PartialEq)]
struct PerfConfig {
    subdivide_levels: u32,
    lod_levels: u32,
    reduction_ratio: f32,
}

impl Default for PerfConfig {
    fn default() -> Self {
        Self {
            subdivide_levels: 2,
            lod_levels: 3,
            reduction_ratio: 0.6,
        }
    }
}

#[derive(Resource, Pane)]
#[pane(title = "LOD Bench", position = "top-right")]
struct PerfPane {
    #[pane(slider, min = 1.0, max = 3.0, step = 1.0)]
    subdivide_levels: u32,
    #[pane(slider, min = 2.0, max = 4.0, step = 1.0)]
    lod_levels: u32,
    #[pane(slider, min = 0.35, max = 0.85, step = 0.05)]
    reduction_ratio: f32,
}

impl Default for PerfPane {
    fn default() -> Self {
        Self {
            subdivide_levels: 2,
            lod_levels: 3,
            reduction_ratio: 0.6,
        }
    }
}

#[derive(Resource, Default)]
struct PerfSummary {
    export_ms: f32,
    import_ms: f32,
    lod_face_counts: Vec<usize>,
}

#[derive(Component)]
struct PerfRoot;

#[derive(Component)]
struct PerfOverlay;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.07)))
        .init_resource::<PerfConfig>()
        .init_resource::<PerfPane>()
        .init_resource::<PerfSummary>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh_ops perf".to_string(),
                resolution: (1500, 920).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            bevy_flair::FlairPlugin,
            bevy_input_focus::InputDispatchPlugin,
            bevy_ui_widgets::UiWidgetsPlugins,
            bevy_input_focus::tab_navigation::TabNavigationPlugin,
            PanePlugin,
        ))
        .register_pane::<PerfPane>()
        .add_systems(Startup, setup)
        .add_systems(Update, (sync_pane_to_config, rebuild_lod_showcase, update_overlay).chain())
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Perf Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.5, 14.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Perf Light"),
        DirectionalLight {
            illuminance: 22_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.95, 0.9, 0.0)),
    ));
    commands.spawn((
        Name::new("Perf Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 12.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.09, 0.11),
            perceptual_roughness: 0.96,
            ..default()
        })),
    ));
    commands.spawn((
        Name::new("Perf Overlay"),
        PerfOverlay,
        Text::new("mesh_ops perf"),
        Node {
            position_type: PositionType::Absolute,
            top: px(18),
            left: px(18),
            width: px(390),
            padding: UiRect::all(px(12)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.88)),
    ));
}

fn sync_pane_to_config(pane: Res<PerfPane>, mut config: ResMut<PerfConfig>) {
    let next = PerfConfig {
        subdivide_levels: pane.subdivide_levels.clamp(1, 3),
        lod_levels: pane.lod_levels.clamp(2, 4),
        reduction_ratio: pane.reduction_ratio.clamp(0.25, 0.9),
    };
    if *config != next {
        *config = next;
    }
}

fn rebuild_lod_showcase(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<PerfConfig>,
    mut summary: ResMut<PerfSummary>,
    roots: Query<Entity, With<PerfRoot>>,
) {
    if !config.is_changed() && !summary.lod_face_counts.is_empty() {
        return;
    }

    for entity in &roots {
        commands.entity(entity).despawn();
    }

    let mut base = HalfEdgeMesh::unit_cube().expect("cube");
    base.subdivide_catmull_clark(config.subdivide_levels)
        .expect("subdivide");

    let start = Instant::now();
    let exported = base.to_bevy_mesh().expect("export");
    summary.export_ms = start.elapsed().as_secs_f32() * 1_000.0;

    let start = Instant::now();
    let imported = HalfEdgeMesh::from_bevy_mesh(&exported).expect("import");
    summary.import_ms = start.elapsed().as_secs_f32() * 1_000.0;

    let lods = imported
        .build_lod_chain(&MeshLodConfig {
            level_count: config.lod_levels,
            reduction_ratio: config.reduction_ratio,
            ..default()
        })
        .expect("lod chain");

    summary.lod_face_counts = lods.iter().map(|level| level.face_count).collect();
    let spacing = 4.0;
    let start_x = -((lods.len().saturating_sub(1)) as f32 * spacing) * 0.5;

    for (index, lod) in lods.into_iter().enumerate() {
        let x = start_x + index as f32 * spacing;
        commands.spawn((
            Name::new(format!("LOD {}", lod.level)),
            PerfRoot,
            Mesh3d(meshes.add(lod.mesh.to_bevy_mesh().expect("lod mesh"))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::hsl(30.0 + index as f32 * 38.0, 0.58, 0.58),
                perceptual_roughness: 0.82,
                ..default()
            })),
            Transform::from_xyz(x, 1.2, 0.0),
        ));
        commands.spawn((
            Name::new(format!("LOD {} Label", lod.level)),
            PerfRoot,
            Text2d::new(format!("LOD {}\n{} faces", lod.level, lod.face_count)),
            TextFont::from_font_size(18.0),
            TextColor(Color::WHITE),
            Transform::from_xyz(x, 3.4, 0.0),
        ));
    }
}

fn update_overlay(
    config: Res<PerfConfig>,
    summary: Res<PerfSummary>,
    mut overlay: Single<&mut Text, With<PerfOverlay>>,
) {
    if !config.is_changed() && !summary.is_changed() {
        return;
    }

    **overlay = Text::new(format!(
        "mesh_ops perf\nsubdivide levels: {}\nlod levels: {}\nreduction ratio: {:.2}\nexport/import: {:.2}ms / {:.2}ms\nlod faces: {:?}",
        config.subdivide_levels,
        config.lod_levels,
        config.reduction_ratio,
        summary.export_ms,
        summary.import_ms,
        summary.lod_face_counts
    ));
}
