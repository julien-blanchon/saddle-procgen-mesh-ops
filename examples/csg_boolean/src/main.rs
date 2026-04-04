use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_procgen_mesh_ops::{
    HalfEdgeMesh, MeshBooleanConfig, MeshBooleanOperation, MeshError,
};

#[derive(Resource, Clone, PartialEq)]
struct FortressBreachConfig {
    voxel_size: f32,
    overlap: f32,
    impact_height: f32,
    attack_angle_deg: f32,
}

impl Default for FortressBreachConfig {
    fn default() -> Self {
        Self {
            voxel_size: 0.12,
            overlap: 0.95,
            impact_height: 1.2,
            attack_angle_deg: 18.0,
        }
    }
}

#[derive(Resource, Pane)]
#[pane(title = "Fortress Breach", position = "top-right")]
struct FortressBreachPane {
    #[pane(slider, min = 0.08, max = 0.22, step = 0.01)]
    voxel_size: f32,
    #[pane(slider, min = 0.45, max = 1.35, step = 0.05)]
    overlap: f32,
    #[pane(slider, min = 0.75, max = 1.8, step = 0.05)]
    impact_height: f32,
    #[pane(slider, min = -35.0, max = 35.0, step = 1.0)]
    attack_angle_deg: f32,
}

impl Default for FortressBreachPane {
    fn default() -> Self {
        Self {
            voxel_size: 0.12,
            overlap: 0.95,
            impact_height: 1.2,
            attack_angle_deg: 18.0,
        }
    }
}

#[derive(Resource)]
struct FortressSceneMeshes {
    union: Handle<Mesh>,
    intersection: Handle<Mesh>,
    difference: Handle<Mesh>,
}

#[derive(Component, Clone, Copy)]
enum OperationSlot {
    Union,
    Intersection,
    Difference,
}

impl OperationSlot {
    fn all() -> [Self; 3] {
        [Self::Union, Self::Intersection, Self::Difference]
    }

    fn center(self) -> Vec3 {
        match self {
            Self::Union => Vec3::new(-4.4, 0.0, 0.0),
            Self::Intersection => Vec3::new(0.0, 0.0, 0.0),
            Self::Difference => Vec3::new(4.4, 0.0, 0.0),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Union => "Union",
            Self::Intersection => "Intersection",
            Self::Difference => "Difference",
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Union => Color::srgb(0.50, 0.78, 0.59),
            Self::Intersection => Color::srgb(0.94, 0.55, 0.34),
            Self::Difference => Color::srgb(0.77, 0.73, 0.61),
        }
    }
}

#[derive(Component)]
struct ResultMesh(OperationSlot);

#[derive(Component)]
struct CutterPreview(OperationSlot);

#[derive(Component)]
struct ExampleOverlay;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.03, 0.04, 0.06)))
        .init_resource::<FortressBreachConfig>()
        .init_resource::<FortressBreachPane>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mesh_ops csg_boolean".to_string(),
                resolution: (1540, 920).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PanePlugin)
        .register_pane::<FortressBreachPane>()
        .add_systems(Startup, (setup_scene, build_initial_showcase).chain())
        .add_systems(
            Update,
            (
                sync_pane_to_config,
                update_cutter_previews,
                rebuild_showcase_meshes,
                update_overlay,
            )
                .chain(),
        )
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Fortress Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.8, 13.6).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Moon Key Light"),
        DirectionalLight {
            illuminance: 16_500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.05, 0.85, 0.0)),
    ));
    commands.spawn((
        Name::new("Forge Light Left"),
        PointLight {
            intensity: 95_000.0,
            color: Color::srgb(1.0, 0.68, 0.46),
            range: 22.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-8.0, 4.6, 5.5),
    ));
    commands.spawn((
        Name::new("Forge Light Right"),
        PointLight {
            intensity: 60_000.0,
            color: Color::srgb(0.42, 0.62, 1.0),
            range: 18.0,
            ..default()
        },
        Transform::from_xyz(8.0, 3.6, -3.0),
    ));

    commands.spawn((
        Name::new("Arena Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(22.0, 16.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.09, 0.11),
            perceptual_roughness: 0.96,
            metallic: 0.03,
            ..default()
        })),
    ));
    commands.spawn((
        Name::new("Back Wall"),
        Mesh3d(meshes.add(Cuboid::new(18.0, 6.5, 0.35))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.07, 0.08, 0.10),
            perceptual_roughness: 0.92,
            ..default()
        })),
        Transform::from_xyz(0.0, 3.0, -5.8),
    ));

    let tower_mesh = meshes.add(Cuboid::new(1.6, 2.8, 1.6));
    let ram_mesh = meshes.add(Capsule3d::new(0.55, 1.6).mesh().rings(8).latitudes(10));
    let union_handle = meshes.add(Cuboid::new(0.2, 0.2, 0.2));
    let intersection_handle = meshes.add(Cuboid::new(0.2, 0.2, 0.2));
    let difference_handle = meshes.add(Cuboid::new(0.2, 0.2, 0.2));

    commands.insert_resource(FortressSceneMeshes {
        union: union_handle.clone(),
        intersection: intersection_handle.clone(),
        difference: difference_handle.clone(),
    });

    for slot in OperationSlot::all() {
        let center = slot.center();
        commands.spawn((
            Name::new(format!("{} Pedestal", slot.label())),
            Mesh3d(meshes.add(Cuboid::new(3.2, 0.7, 3.2))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.12, 0.13, 0.16),
                metallic: 0.08,
                perceptual_roughness: 0.92,
                ..default()
            })),
            Transform::from_translation(center + Vec3::new(0.0, 0.35, 0.0)),
        ));
        commands.spawn((
            Name::new(format!("{} Tower Preview", slot.label())),
            Mesh3d(tower_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.76, 0.79, 0.84, 0.18),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(center + Vec3::new(0.0, 1.75, 0.0)),
        ));
        commands.spawn((
            Name::new(format!("{} Cutter Preview", slot.label())),
            CutterPreview(slot),
            Mesh3d(ram_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.48, 0.35, 0.26),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(center),
        ));
        commands.spawn((
            Name::new(format!("{} Result", slot.label())),
            ResultMesh(slot),
            Mesh3d(match slot {
                OperationSlot::Union => union_handle.clone(),
                OperationSlot::Intersection => intersection_handle.clone(),
                OperationSlot::Difference => difference_handle.clone(),
            }),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: slot.color(),
                metallic: 0.05,
                perceptual_roughness: 0.7,
                ..default()
            })),
            Transform::from_translation(center + Vec3::new(0.0, 0.72, 0.0)),
        ));
    }

    commands.spawn((
        Name::new("Boolean Overlay"),
        ExampleOverlay,
        Text::new("mesh_ops csg_boolean"),
        Node {
            position_type: PositionType::Absolute,
            left: px(18),
            top: px(18),
            width: px(460),
            padding: UiRect::axes(px(14), px(12)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.07, 0.09, 0.88)),
    ));
}

fn build_initial_showcase(
    config: Res<FortressBreachConfig>,
    scene: Res<FortressSceneMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cutters: Query<(&CutterPreview, &mut Transform)>,
    mut results: Query<(&ResultMesh, &mut Transform)>,
) {
    update_preview_transforms(&config, &mut cutters, &mut results);
    regenerate_mesh_assets(&config, &scene, &mut meshes).expect("initial boolean showcase");
}

fn sync_pane_to_config(
    pane: Res<FortressBreachPane>,
    mut config: ResMut<FortressBreachConfig>,
) {
    let next = FortressBreachConfig {
        voxel_size: pane.voxel_size.clamp(0.05, 0.3),
        overlap: pane.overlap.clamp(0.3, 1.5),
        impact_height: pane.impact_height.clamp(0.6, 2.0),
        attack_angle_deg: pane.attack_angle_deg.clamp(-45.0, 45.0),
    };
    if *config != next {
        *config = next;
    }
}

fn update_cutter_previews(
    config: Res<FortressBreachConfig>,
    mut cutters: Query<(&CutterPreview, &mut Transform)>,
    mut results: Query<(&ResultMesh, &mut Transform)>,
) {
    if !config.is_changed() {
        return;
    }

    update_preview_transforms(&config, &mut cutters, &mut results);
}

fn update_preview_transforms(
    config: &FortressBreachConfig,
    cutters: &mut Query<(&CutterPreview, &mut Transform)>,
    results: &mut Query<(&ResultMesh, &mut Transform)>,
) {
    for (preview, mut transform) in cutters.iter_mut() {
        let center = preview.0.center();
        *transform = cutter_transform(config, center);
    }
    for (result, mut transform) in results.iter_mut() {
        *transform = Transform::from_translation(result.0.center() + Vec3::new(0.0, 0.72, 0.0));
    }
}

fn rebuild_showcase_meshes(
    config: Res<FortressBreachConfig>,
    scene: Res<FortressSceneMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !config.is_changed() {
        return;
    }

    regenerate_mesh_assets(&config, &scene, &mut meshes).expect("boolean showcase rebuild");
}

fn regenerate_mesh_assets(
    config: &FortressBreachConfig,
    scene: &FortressSceneMeshes,
    meshes: &mut Assets<Mesh>,
) -> Result<(), MeshError> {
    let tower = build_keep_mesh();
    let cutter = build_battering_ram_mesh(config)?;
    let boolean_config = MeshBooleanConfig {
        voxel_size: config.voxel_size,
        padding_voxels: 1,
        max_cells_per_axis: 48,
    };

    let mut union = tower.boolean_with(&cutter, MeshBooleanOperation::Union, &boolean_config)?;
    let mut intersection =
        tower.boolean_with(&cutter, MeshBooleanOperation::Intersection, &boolean_config)?;
    let mut difference =
        tower.boolean_with(&cutter, MeshBooleanOperation::Difference, &boolean_config)?;

    union.recompute_normals()?;
    intersection.recompute_normals()?;
    difference.recompute_normals()?;

    if let Some(mesh) = meshes.get_mut(&scene.union) {
        *mesh = union.to_bevy_mesh()?;
    }
    if let Some(mesh) = meshes.get_mut(&scene.intersection) {
        *mesh = intersection.to_bevy_mesh()?;
    }
    if let Some(mesh) = meshes.get_mut(&scene.difference) {
        *mesh = difference.to_bevy_mesh()?;
    }

    Ok(())
}

fn update_overlay(
    config: Res<FortressBreachConfig>,
    mut overlay: Single<&mut Text, With<ExampleOverlay>>,
) {
    if !config.is_changed() {
        return;
    }

    **overlay = Text::new(format!(
        "mesh_ops csg_boolean\nvoxel size: {:.2}\noverlap: {:.2}\nimpact height: {:.2}\nattack angle: {:.0} deg\n\nFortress tower and battering ram previews are shown as ghosts.\nThe solid meshes demonstrate voxelized union, intersection, and difference in parallel.",
        config.voxel_size,
        config.overlap,
        config.impact_height,
        config.attack_angle_deg
    ));
}

fn build_keep_mesh() -> HalfEdgeMesh {
    let mut mesh = HalfEdgeMesh::unit_cube().expect("keep cube");
    transform_mesh(
        &mut mesh,
        Vec3::new(1.6, 2.8, 1.6),
        Quat::IDENTITY,
        Vec3::new(0.0, 1.4, 0.0),
    );
    mesh
}

fn build_battering_ram_mesh(config: &FortressBreachConfig) -> Result<HalfEdgeMesh, MeshError> {
    let mut mesh = HalfEdgeMesh::from_bevy_mesh(&Mesh::from(
        Capsule3d::new(0.55, 1.6).mesh().rings(8).latitudes(10),
    ))?;
    let rotation = Quat::from_euler(
        EulerRot::XYZ,
        0.0,
        config.attack_angle_deg.to_radians(),
        std::f32::consts::FRAC_PI_2,
    );
    transform_mesh(
        &mut mesh,
        Vec3::ONE,
        rotation,
        Vec3::new(config.overlap, config.impact_height, 0.0),
    );
    Ok(mesh)
}

fn transform_mesh(mesh: &mut HalfEdgeMesh, scale: Vec3, rotation: Quat, translation: Vec3) {
    let vertices = mesh.vertex_ids().collect::<Vec<_>>();
    for vertex in vertices {
        let payload = mesh.vertex_payload_mut(vertex).expect("mesh vertex");
        payload.position = translation + rotation * (payload.position * scale);
    }
}

fn cutter_transform(config: &FortressBreachConfig, center: Vec3) -> Transform {
    Transform::from_translation(center + Vec3::new(config.overlap, config.impact_height, 0.0))
        .with_rotation(Quat::from_euler(
            EulerRot::XYZ,
            0.0,
            config.attack_angle_deg.to_radians(),
            std::f32::consts::FRAC_PI_2,
        ))
}
