#[cfg(feature = "e2e")]
mod e2e;

use bevy::prelude::*;
#[cfg(feature = "dev")]
use bevy::remote::RemotePlugin;
#[cfg(feature = "dev")]
use bevy::remote::http::RemoteHttpPlugin;
#[cfg(feature = "dev")]
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_pane::prelude::*;
use saddle_procgen_mesh_ops::{
    EditableMesh, FaceId, HalfEdgeMesh, MeshEditCommand, MeshOpsDebugView, MeshOpsFailed,
    MeshOpsPlugin, MeshOpsRequest, MeshOpsSystems, MeshOpsTarget, MeshTopologyChanged, VertexId,
};

#[derive(Component)]
struct ExtrudeDemo;

#[derive(Component)]
struct BevelDemo;

#[derive(Component)]
struct SubdivisionDemo;

#[derive(Component)]
struct CraterDemo;

#[derive(Resource)]
struct LabEntities {
    extrude: Entity,
    bevel: Entity,
    subdivision: Entity,
    crater: Entity,
}

#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
pub struct LabControl {
    pub pending_extrude: bool,
    pub pending_bevel: bool,
    pub pending_subdivide: bool,
    pub pending_crater_steps: u32,
    pub extrude_request_count: u32,
    pub bevel_request_count: u32,
    pub subdivide_request_count: u32,
    pub crater_request_count: u32,
}

#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct LabConfig {
    pub extrude_distance: f32,
    pub bevel_width: f32,
    pub subdivision_levels: u32,
    pub crater_depth: f32,
}

impl Default for LabConfig {
    fn default() -> Self {
        Self {
            extrude_distance: 0.35,
            bevel_width: 0.18,
            subdivision_levels: 1,
            crater_depth: 0.08,
        }
    }
}

#[derive(Resource, Debug, Clone, Pane)]
#[pane(title = "Mesh Ops Lab", position = "top-right")]
pub struct LabPane {
    #[pane(slider, min = 0.15, max = 0.75, step = 0.02)]
    pub extrude_distance: f32,
    #[pane(slider, min = 0.08, max = 0.32, step = 0.01)]
    pub bevel_width: f32,
    #[pane(slider, min = 1.0, max = 2.0, step = 1.0)]
    pub subdivision_levels: u32,
    #[pane(slider, min = 0.03, max = 0.18, step = 0.01)]
    pub crater_depth: f32,
    #[pane(number, min = 0.0, step = 1.0)]
    pub extrude_requests: u32,
    #[pane(number, min = 0.0, step = 1.0)]
    pub bevel_requests: u32,
    #[pane(number, min = 0.0, step = 1.0)]
    pub subdivide_requests: u32,
    #[pane(number, min = 0.0, step = 1.0)]
    pub crater_requests: u32,
}

impl Default for LabPane {
    fn default() -> Self {
        Self {
            extrude_distance: 0.35,
            bevel_width: 0.18,
            subdivision_levels: 1,
            crater_depth: 0.08,
            extrude_requests: 0,
            bevel_requests: 0,
            subdivide_requests: 0,
            crater_requests: 0,
        }
    }
}

#[derive(Debug, Clone, Default, Reflect)]
pub struct DemoStats {
    pub revision: u64,
    pub vertices: usize,
    pub edges: usize,
    pub faces: usize,
    pub min_y: f32,
    pub max_y: f32,
}

#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
pub struct LabDiagnostics {
    pub extrude: DemoStats,
    pub bevel: DemoStats,
    pub subdivision: DemoStats,
    pub crater: DemoStats,
    pub topology_changes: u32,
    pub failures: u32,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "mesh_ops lab".to_string(),
            resolution: (1520, 920).into(),
            ..default()
        }),
        ..default()
    }));
    app.insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)));
    app.add_plugins(MeshOpsPlugin::default());
    app.init_resource::<LabControl>();
    app.init_resource::<LabConfig>();
    app.init_resource::<LabPane>();
    app.init_resource::<LabDiagnostics>();
    app.register_type::<LabControl>();
    app.register_type::<LabConfig>();
    app.register_type::<DemoStats>();
    app.register_type::<LabDiagnostics>();
    app.add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        PanePlugin,
    ));
    app.register_pane::<LabPane>();
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            sync_pane_to_resources,
            handle_keyboard_input,
            emit_lab_requests.before(MeshOpsSystems::ProcessRequests),
            collect_runtime_messages.after(MeshOpsSystems::SyncMeshes),
            sync_diagnostics.after(MeshOpsSystems::SyncMeshes),
        ),
    );
    #[cfg(feature = "dev")]
    app.add_plugins(RemotePlugin::default());
    #[cfg(feature = "dev")]
    app.add_plugins(BrpExtrasPlugin::with_http_plugin(
        RemoteHttpPlugin::default(),
    ));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::MeshOpsLabE2EPlugin);
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("MeshOps Lab Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 6.5, 11.0).looking_at(Vec3::new(0.0, 0.4, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("MeshOps Lab Light"),
        DirectionalLight {
            illuminance: 18_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.95, 0.85, 0.0)),
    ));

    let extrude = spawn_demo(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Extrude Demo",
        build_extrude_mesh(),
        Transform::from_xyz(-3.5, 0.0, 2.0),
        Color::srgb(0.75, 0.53, 0.36),
        ExtrudeDemo,
    );
    let bevel = spawn_demo(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Bevel Demo",
        build_bevel_mesh(),
        Transform::from_xyz(3.5, 0.0, 2.0),
        Color::srgb(0.34, 0.73, 0.86),
        BevelDemo,
    );
    let subdivision = spawn_demo(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Subdivision Demo",
        build_subdivision_mesh(),
        Transform::from_xyz(-3.5, 0.0, -2.2),
        Color::srgb(0.69, 0.73, 0.34),
        SubdivisionDemo,
    );
    let crater = spawn_demo(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Crater Demo",
        build_crater_mesh(),
        Transform::from_xyz(3.5, 0.0, -2.2),
        Color::srgb(0.62, 0.62, 0.7),
        CraterDemo,
    );

    commands.insert_resource(LabEntities {
        extrude,
        bevel,
        subdivision,
        crater,
    });
}

fn spawn_demo<M: Component>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    mut editable: HalfEdgeMesh,
    transform: Transform,
    color: Color,
    marker: M,
) -> Entity {
    editable.recompute_normals().expect("demo normals");
    let handle = meshes.add(editable.to_bevy_mesh().expect("demo mesh"));
    commands
        .spawn((
            Name::new(name.to_owned()),
            marker,
            Mesh3d(handle.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.86,
                ..default()
            })),
            transform,
            EditableMesh::new(editable),
            MeshOpsTarget::new(handle),
            MeshOpsDebugView {
                enabled: true,
                draw_edges: true,
                draw_boundary_edges: true,
                draw_face_normals: false,
                draw_vertex_normals: false,
            },
        ))
        .id()
}

fn handle_keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut control: ResMut<LabControl>) {
    if keys.just_pressed(KeyCode::KeyE) {
        control.pending_extrude = true;
        control.extrude_request_count = control.extrude_request_count.saturating_add(1);
    }
    if keys.just_pressed(KeyCode::KeyB) {
        control.pending_bevel = true;
        control.bevel_request_count = control.bevel_request_count.saturating_add(1);
    }
    if keys.just_pressed(KeyCode::KeyS) {
        control.pending_subdivide = true;
        control.subdivide_request_count = control.subdivide_request_count.saturating_add(1);
    }
    if keys.just_pressed(KeyCode::KeyC) {
        control.pending_crater_steps = control.pending_crater_steps.saturating_add(1);
        control.crater_request_count = control.crater_request_count.saturating_add(1);
    }
}

fn sync_pane_to_resources(
    pane: Res<LabPane>,
    mut control: ResMut<LabControl>,
    mut config: ResMut<LabConfig>,
) {
    if !pane.is_changed() {
        return;
    }

    *config = LabConfig {
        extrude_distance: pane.extrude_distance.clamp(0.1, 0.9),
        bevel_width: pane.bevel_width.clamp(0.05, 0.4),
        subdivision_levels: pane.subdivision_levels.clamp(1, 2),
        crater_depth: pane.crater_depth.clamp(0.02, 0.2),
    };

    if pane.extrude_requests > control.extrude_request_count {
        control.pending_extrude = true;
        control.extrude_request_count = pane.extrude_requests;
    }
    if pane.bevel_requests > control.bevel_request_count {
        control.pending_bevel = true;
        control.bevel_request_count = pane.bevel_requests;
    }
    if pane.subdivide_requests > control.subdivide_request_count {
        control.pending_subdivide = true;
        control.subdivide_request_count = pane.subdivide_requests;
    }
    if pane.crater_requests > control.crater_request_count {
        let delta = pane.crater_requests - control.crater_request_count;
        control.pending_crater_steps = control.pending_crater_steps.saturating_add(delta);
        control.crater_request_count = pane.crater_requests;
    }
}

fn emit_lab_requests(
    mut control: ResMut<LabControl>,
    config: Res<LabConfig>,
    entities: Res<LabEntities>,
    meshes: Query<&EditableMesh>,
    mut requests: MessageWriter<MeshOpsRequest>,
) {
    if control.pending_extrude {
        control.pending_extrude = false;
        requests.write(MeshOpsRequest {
            entity: entities.extrude,
            command: MeshEditCommand::ExtrudeFaces {
                faces: vec![FaceId(0)],
                distance: config.extrude_distance,
            },
            prefer_async: false,
        });
    }

    if control.pending_bevel {
        control.pending_bevel = false;
        if let Ok(mesh) = meshes.get(entities.bevel) {
            if let Some(edge) = find_edge_by_endpoints(&mesh.mesh, 1, 2) {
                requests.write(MeshOpsRequest {
                    entity: entities.bevel,
                    command: MeshEditCommand::BevelEdges {
                        edges: vec![edge],
                        width: config.bevel_width,
                    },
                    prefer_async: false,
                });
            }
        }
    }

    if control.pending_subdivide {
        control.pending_subdivide = false;
        requests.write(MeshOpsRequest {
            entity: entities.subdivision,
            command: MeshEditCommand::SubdivideCatmullClark {
                levels: config.subdivision_levels,
            },
            prefer_async: true,
        });
    }

    if control.pending_crater_steps > 0 {
        control.pending_crater_steps -= 1;
        if let Ok(mesh) = meshes.get(entities.crater) {
            let vertices = crater_vertices(&mesh.mesh);
            requests.write(MeshOpsRequest {
                entity: entities.crater,
                command: MeshEditCommand::OffsetVertices {
                    vertices,
                    offset: Vec3::new(0.0, -config.crater_depth, 0.0),
                },
                prefer_async: false,
            });
            requests.write(MeshOpsRequest {
                entity: entities.crater,
                command: MeshEditCommand::RecomputeNormals,
                prefer_async: false,
            });
        }
    }
}

fn collect_runtime_messages(
    mut changed: MessageReader<MeshTopologyChanged>,
    mut failed: MessageReader<MeshOpsFailed>,
    mut diagnostics: ResMut<LabDiagnostics>,
) {
    diagnostics.topology_changes = diagnostics
        .topology_changes
        .saturating_add(changed.read().count() as u32);
    diagnostics.failures = diagnostics
        .failures
        .saturating_add(failed.read().count() as u32);
}

fn sync_diagnostics(
    entities: Res<LabEntities>,
    meshes: Query<&EditableMesh>,
    mut diagnostics: ResMut<LabDiagnostics>,
) {
    diagnostics.extrude = meshes
        .get(entities.extrude)
        .map(demo_stats)
        .unwrap_or_default();
    diagnostics.bevel = meshes
        .get(entities.bevel)
        .map(demo_stats)
        .unwrap_or_default();
    diagnostics.subdivision = meshes
        .get(entities.subdivision)
        .map(demo_stats)
        .unwrap_or_default();
    diagnostics.crater = meshes
        .get(entities.crater)
        .map(demo_stats)
        .unwrap_or_default();
}

fn demo_stats(mesh: &EditableMesh) -> DemoStats {
    let (min_y, max_y) = mesh
        .mesh
        .vertex_ids()
        .filter_map(|vertex| {
            mesh.mesh
                .vertex_payload(vertex)
                .ok()
                .map(|payload| payload.position.y)
        })
        .fold((f32::MAX, f32::MIN), |(min_y, max_y), value| {
            (min_y.min(value), max_y.max(value))
        });

    DemoStats {
        revision: mesh.revision,
        vertices: mesh.mesh.vertex_count(),
        edges: mesh.mesh.edge_count(),
        faces: mesh.mesh.face_count(),
        min_y: if min_y.is_finite() { min_y } else { 0.0 },
        max_y: if max_y.is_finite() { max_y } else { 0.0 },
    }
}

fn find_edge_by_endpoints(
    mesh: &HalfEdgeMesh,
    a: usize,
    b: usize,
) -> Option<saddle_procgen_mesh_ops::EdgeId> {
    mesh.edge_ids().find(|edge| {
        mesh.edge_endpoints(*edge)
            .map(|(left, right)| {
                (left.index() == a && right.index() == b)
                    || (left.index() == b && right.index() == a)
            })
            .unwrap_or(false)
    })
}

fn crater_vertices(mesh: &HalfEdgeMesh) -> Vec<VertexId> {
    mesh.vertex_ids()
        .filter(|vertex| {
            mesh.vertex_payload(*vertex)
                .map(|payload| Vec2::new(payload.position.x, payload.position.z).length() <= 0.65)
                .unwrap_or(false)
        })
        .collect()
}

fn build_extrude_mesh() -> HalfEdgeMesh {
    HalfEdgeMesh::unit_cube().expect("cube")
}

fn build_bevel_mesh() -> HalfEdgeMesh {
    HalfEdgeMesh::from_polygon_faces(
        vec![
            saddle_procgen_mesh_ops::VertexPayload {
                position: Vec3::new(0.0, 1.1, 0.0),
                ..default()
            },
            saddle_procgen_mesh_ops::VertexPayload {
                position: Vec3::new(-1.0, 0.0, 0.0),
                ..default()
            },
            saddle_procgen_mesh_ops::VertexPayload {
                position: Vec3::new(1.0, 0.0, 0.0),
                ..default()
            },
            saddle_procgen_mesh_ops::VertexPayload {
                position: Vec3::new(0.0, -1.1, 0.0),
                ..default()
            },
        ],
        vec![
            saddle_procgen_mesh_ops::PolygonFace::new(vec![1, 2, 0]),
            saddle_procgen_mesh_ops::PolygonFace::new(vec![2, 1, 3]),
        ],
    )
    .expect("bevel strip")
}

fn build_subdivision_mesh() -> HalfEdgeMesh {
    HalfEdgeMesh::unit_cube().expect("cube")
}

fn build_crater_mesh() -> HalfEdgeMesh {
    let mut mesh = HalfEdgeMesh::unit_quad().expect("quad");
    for vertex in mesh.vertex_ids().collect::<Vec<_>>() {
        let position = mesh.vertex_payload(vertex).expect("payload").position;
        mesh.vertex_payload_mut(vertex).expect("payload").position =
            Vec3::new(position.x * 2.2, 0.0, position.y * 2.2);
    }
    mesh.subdivide_catmull_clark(2).expect("subdivide");
    mesh
}

#[cfg(feature = "e2e")]
pub(crate) fn trigger_extrude(world: &mut World) {
    world.resource_mut::<LabControl>().pending_extrude = true;
}

#[cfg(feature = "e2e")]
pub(crate) fn trigger_bevel(world: &mut World) {
    world.resource_mut::<LabControl>().pending_bevel = true;
}

#[cfg(feature = "e2e")]
pub(crate) fn trigger_subdivide(world: &mut World) {
    world.resource_mut::<LabControl>().pending_subdivide = true;
}

#[cfg(feature = "e2e")]
pub(crate) fn trigger_crater_steps(world: &mut World, steps: u32) {
    world.resource_mut::<LabControl>().pending_crater_steps = steps;
}
