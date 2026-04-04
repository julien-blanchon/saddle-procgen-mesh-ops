use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_procgen_mesh_ops::{HalfEdgeMesh, MeshError};

#[derive(Resource, Clone, PartialEq)]
struct BasicConfig {
    poke_count: u32,
    smooth_levels: u32,
}

impl Default for BasicConfig {
    fn default() -> Self {
        Self {
            poke_count: 1,
            smooth_levels: 0,
        }
    }
}

#[derive(Resource, Pane)]
#[pane(title = "Topology Starter", position = "top-right")]
struct BasicPane {
    #[pane(slider, min = 0.0, max = 4.0, step = 1.0)]
    poke_count: u32,
    #[pane(slider, min = 0.0, max = 2.0, step = 1.0)]
    smooth_levels: u32,
}

impl Default for BasicPane {
    fn default() -> Self {
        Self {
            poke_count: 1,
            smooth_levels: 0,
        }
    }
}

#[derive(Resource, Default)]
struct BasicSummary {
    vertices: usize,
    edges: usize,
    faces: usize,
    closed: bool,
}

#[derive(Component)]
struct BasicRoot;

#[derive(Component)]
struct BasicOverlay;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.07)))
        .init_resource::<BasicConfig>()
        .init_resource::<BasicPane>()
        .init_resource::<BasicSummary>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh_ops basic".to_string(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PanePlugin)
        .register_pane::<BasicPane>()
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
        Name::new("Basic Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.8, 10.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Basic Key Light"),
        DirectionalLight {
            illuminance: 18_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.8, 0.0)),
    ));
    commands.spawn((
        Name::new("Basic Fill Light"),
        PointLight {
            intensity: 95_000.0,
            color: Color::srgb(0.40, 0.62, 1.0),
            range: 18.0,
            ..default()
        },
        Transform::from_xyz(4.0, 4.0, -2.0),
    ));
    commands.spawn((
        Name::new("Workshop Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(16.0, 16.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.09, 0.11),
            perceptual_roughness: 0.96,
            ..default()
        })),
    ));
    commands.spawn((
        Name::new("Basic Overlay"),
        BasicOverlay,
        Text::new("mesh_ops basic"),
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

fn sync_pane_to_config(pane: Res<BasicPane>, mut config: ResMut<BasicConfig>) {
    let next = BasicConfig {
        poke_count: pane.poke_count.min(4),
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
    config: Res<BasicConfig>,
    mut summary: ResMut<BasicSummary>,
    roots: Query<Entity, With<BasicRoot>>,
) {
    if !config.is_changed() && summary.faces > 0 {
        return;
    }

    for entity in &roots {
        commands.entity(entity).despawn();
    }

    let mesh = build_basic_mesh(&config).expect("basic showcase should build");
    summary.vertices = mesh.vertex_count();
    summary.edges = mesh.edge_count();
    summary.faces = mesh.face_count();
    summary.closed = mesh.is_closed();

    commands.spawn((
        Name::new("Basic Showcase"),
        BasicRoot,
        Mesh3d(meshes.add(mesh.to_bevy_mesh().expect("basic bevy mesh"))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.82, 0.70, 0.52),
            metallic: 0.06,
            perceptual_roughness: 0.78,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.45, 0.0),
    ));
}

fn build_basic_mesh(config: &BasicConfig) -> Result<HalfEdgeMesh, MeshError> {
    let mut mesh = HalfEdgeMesh::unit_cube()?;
    for _ in 0..config.poke_count {
        let face = mesh.face_ids().next().expect("cube has a face");
        mesh.poke_face(face)?;
    }
    if config.smooth_levels > 0 {
        mesh.subdivide_catmull_clark(config.smooth_levels)?;
    }
    mesh.recompute_normals()?;
    Ok(mesh)
}

fn update_overlay(
    config: Res<BasicConfig>,
    summary: Res<BasicSummary>,
    mut overlay: Single<&mut Text, With<BasicOverlay>>,
) {
    if !config.is_changed() && !summary.is_changed() {
        return;
    }

    **overlay = Text::new(format!(
        "mesh_ops basic\npoke passes: {}\nsmooth levels: {}\nvertices/edges/faces: {}/{}/{}\nclosed: {}",
        config.poke_count,
        config.smooth_levels,
        summary.vertices,
        summary.edges,
        summary.faces,
        summary.closed
    ));
}
