use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_procgen_mesh_ops::HalfEdgeMesh;

#[derive(Resource, Clone, PartialEq)]
struct SubdivisionConfig {
    catmull_levels: u32,
    loop_levels: u32,
}

impl Default for SubdivisionConfig {
    fn default() -> Self {
        Self {
            catmull_levels: 1,
            loop_levels: 1,
        }
    }
}

#[derive(Resource, Pane)]
#[pane(title = "Subdivision", position = "top-right")]
struct SubdivisionPane {
    #[pane(slider, min = 0.0, max = 2.0, step = 1.0)]
    catmull_levels: u32,
    #[pane(slider, min = 0.0, max = 2.0, step = 1.0)]
    loop_levels: u32,
}

impl Default for SubdivisionPane {
    fn default() -> Self {
        Self {
            catmull_levels: 1,
            loop_levels: 1,
        }
    }
}

#[derive(Resource, Default)]
struct SubdivisionSummary {
    original_faces: usize,
    catmull_faces: usize,
    loop_faces: usize,
}

#[derive(Component)]
struct SubdivisionRoot;

#[derive(Component)]
struct SubdivisionOverlay;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.07)))
        .init_resource::<SubdivisionConfig>()
        .init_resource::<SubdivisionPane>()
        .init_resource::<SubdivisionSummary>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh_ops subdivision".to_string(),
                resolution: (1500, 920).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PanePlugin)
        .register_pane::<SubdivisionPane>()
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
        Name::new("Subdivision Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.8, 11.0).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Subdivision Light"),
        DirectionalLight {
            illuminance: 22_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.95, 0.85, 0.0)),
    ));
    commands.spawn((
        Name::new("Subdivision Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(18.0, 10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.09, 0.11),
            perceptual_roughness: 0.96,
            ..default()
        })),
    ));
    commands.spawn((
        Name::new("Subdivision Overlay"),
        SubdivisionOverlay,
        Text::new("mesh_ops subdivision"),
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

fn sync_pane_to_config(pane: Res<SubdivisionPane>, mut config: ResMut<SubdivisionConfig>) {
    let next = SubdivisionConfig {
        catmull_levels: pane.catmull_levels.min(2),
        loop_levels: pane.loop_levels.min(2),
    };
    if *config != next {
        *config = next;
    }
}

fn rebuild_showcase(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<SubdivisionConfig>,
    mut summary: ResMut<SubdivisionSummary>,
    roots: Query<Entity, With<SubdivisionRoot>>,
) {
    if !config.is_changed() && summary.original_faces > 0 {
        return;
    }

    for entity in &roots {
        commands.entity(entity).despawn();
    }

    let original = HalfEdgeMesh::unit_cube().expect("cube");
    let mut catmull = HalfEdgeMesh::unit_cube().expect("cube");
    catmull
        .subdivide_catmull_clark(config.catmull_levels)
        .expect("catmull");
    catmull.recompute_normals().expect("catmull normals");

    let mut loop_mesh = HalfEdgeMesh::unit_tetrahedron().expect("tetrahedron");
    loop_mesh.subdivide_loop(config.loop_levels).expect("loop");
    loop_mesh.recompute_normals().expect("loop normals");

    summary.original_faces = original.face_count();
    summary.catmull_faces = catmull.face_count();
    summary.loop_faces = loop_mesh.face_count();

    for (label, mesh, x, color) in [
        ("Original", original, -3.6, Color::srgb(0.76, 0.34, 0.31)),
        ("Catmull-Clark", catmull, 0.0, Color::srgb(0.29, 0.70, 0.88)),
        ("Loop", loop_mesh, 3.6, Color::srgb(0.72, 0.78, 0.36)),
    ] {
        commands.spawn((
            Name::new(label),
            SubdivisionRoot,
            Mesh3d(meshes.add(mesh.to_bevy_mesh().expect("subdivision mesh"))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.82,
                ..default()
            })),
            Transform::from_xyz(x, 1.1, 0.0),
        ));
        commands.spawn((
            Name::new(format!("{label} Label")),
            SubdivisionRoot,
            Text2d::new(label),
            TextFont::from_font_size(20.0),
            TextColor(Color::WHITE),
            Transform::from_xyz(x, 2.9, 0.0),
        ));
    }
}

fn update_overlay(
    config: Res<SubdivisionConfig>,
    summary: Res<SubdivisionSummary>,
    mut overlay: Single<&mut Text, With<SubdivisionOverlay>>,
) {
    if !config.is_changed() && !summary.is_changed() {
        return;
    }

    **overlay = Text::new(format!(
        "mesh_ops subdivision\ncatmull levels: {}\nloop levels: {}\nfaces original/catmull/loop: {}/{}/{}",
        config.catmull_levels,
        config.loop_levels,
        summary.original_faces,
        summary.catmull_faces,
        summary.loop_faces
    ));
}
