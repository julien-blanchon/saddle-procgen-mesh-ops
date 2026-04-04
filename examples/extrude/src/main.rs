use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_procgen_mesh_ops::{HalfEdgeMesh, MeshError};

#[derive(Resource, Clone, PartialEq)]
struct ExtrudeConfig {
    distance: f32,
    smooth_levels: u32,
}

impl Default for ExtrudeConfig {
    fn default() -> Self {
        Self {
            distance: 0.42,
            smooth_levels: 0,
        }
    }
}

#[derive(Resource, Pane)]
#[pane(title = "Tower Extrude", position = "top-right")]
struct ExtrudePane {
    #[pane(slider, min = 0.10, max = 0.80, step = 0.02)]
    distance: f32,
    #[pane(slider, min = 0.0, max = 2.0, step = 1.0)]
    smooth_levels: u32,
}

impl Default for ExtrudePane {
    fn default() -> Self {
        Self {
            distance: 0.42,
            smooth_levels: 0,
        }
    }
}

#[derive(Resource, Default)]
struct ExtrudeSummary {
    faces: usize,
    vertices: usize,
}

#[derive(Component)]
struct ExtrudeRoot;

#[derive(Component)]
struct ExtrudeOverlay;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.045, 0.05, 0.06)))
        .init_resource::<ExtrudeConfig>()
        .init_resource::<ExtrudePane>()
        .init_resource::<ExtrudeSummary>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh_ops extrude".to_string(),
                resolution: (1440, 900).into(),
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
        .register_pane::<ExtrudePane>()
        .add_systems(Startup, setup)
        .add_systems(Update, (sync_pane_to_config, rebuild_showcase, update_overlay).chain())
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Extrude Camera"),
        Camera3d::default(),
        Transform::from_xyz(4.6, 3.8, 8.4).looking_at(Vec3::new(0.0, 1.6, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Extrude Light"),
        DirectionalLight {
            illuminance: 20_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.7, 0.0)),
    ));
    commands.spawn((
        Name::new("Extrude Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(14.0, 14.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.09, 0.11),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));
    commands.spawn((
        Name::new("Extrude Overlay"),
        ExtrudeOverlay,
        Text::new("mesh_ops extrude"),
        Node {
            position_type: PositionType::Absolute,
            top: px(18),
            left: px(18),
            width: px(360),
            padding: UiRect::all(px(12)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.88)),
    ));
}

fn sync_pane_to_config(pane: Res<ExtrudePane>, mut config: ResMut<ExtrudeConfig>) {
    let next = ExtrudeConfig {
        distance: pane.distance.clamp(0.05, 1.0),
        smooth_levels: pane.smooth_levels.min(2),
    };
    if *config != next {
        *config = next;
    }
}

fn rebuild_showcase(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<ExtrudeConfig>,
    mut summary: ResMut<ExtrudeSummary>,
    roots: Query<Entity, With<ExtrudeRoot>>,
) {
    if !config.is_changed() && summary.faces > 0 {
        return;
    }

    for entity in &roots {
        commands.entity(entity).despawn();
    }

    let mesh = build_extrude_mesh(&config).expect("extrude showcase should build");
    summary.faces = mesh.face_count();
    summary.vertices = mesh.vertex_count();

    commands.spawn((
        Name::new("Extruded Tower"),
        ExtrudeRoot,
        Mesh3d(meshes.add(mesh.to_bevy_mesh().expect("extrude mesh"))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.74, 0.62, 0.42),
            metallic: 0.05,
            perceptual_roughness: 0.76,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.3, 0.0),
    ));
}

fn build_extrude_mesh(config: &ExtrudeConfig) -> Result<HalfEdgeMesh, MeshError> {
    let mut mesh = HalfEdgeMesh::unit_cube()?;
    let face = mesh.face_ids().next().expect("cube face");
    mesh.extrude_faces(&[face], config.distance)?;
    if config.smooth_levels > 0 {
        mesh.subdivide_catmull_clark(config.smooth_levels)?;
    }
    mesh.recompute_normals()?;
    Ok(mesh)
}

fn update_overlay(
    config: Res<ExtrudeConfig>,
    summary: Res<ExtrudeSummary>,
    mut overlay: Single<&mut Text, With<ExtrudeOverlay>>,
) {
    if !config.is_changed() && !summary.is_changed() {
        return;
    }

    **overlay = Text::new(format!(
        "mesh_ops extrude\nextrude distance: {:.2}\nsmooth levels: {}\nfaces: {}\nvertices: {}",
        config.distance, config.smooth_levels, summary.faces, summary.vertices
    ));
}
