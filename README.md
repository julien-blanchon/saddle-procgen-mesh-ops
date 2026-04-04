# Saddle Procgen Mesh Ops

Reusable half-edge mesh editing and runtime mesh-processing toolkit for Bevy.

`saddle-procgen-mesh-ops` keeps an editable topology mesh as the authoritative data model and treats Bevy `Mesh` assets as derived render output. The crate is aimed at runtime deformation, procedural modeling, destructible geometry, cleanup, subdivision, and tool-time preprocessing inside a Bevy app.

## Quick Start

Pure mesh editing:

```rust
use saddle_procgen_mesh_ops::HalfEdgeMesh;

let mut mesh = HalfEdgeMesh::unit_cube()?;
let face = mesh.face_ids().next().unwrap();
mesh.extrude_faces(&[face], 0.35)?;
mesh.recompute_normals()?;

let bevy_mesh = mesh.to_bevy_mesh()?;
assert!(bevy_mesh.indices().is_some());
# Ok::<(), saddle_procgen_mesh_ops::MeshError>(())
```

Runtime Bevy integration:

```rust,no_run
use bevy::prelude::*;
use saddle_procgen_mesh_ops::{
    EditableMesh, HalfEdgeMesh, MeshEditCommand, MeshOpsPlugin, MeshOpsRequest, MeshOpsTarget,
};

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Boot,
    Running,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.init_state::<DemoState>();
    app.add_plugins(MeshOpsPlugin::new(
        OnEnter(DemoState::Running),
        OnExit(DemoState::Running),
        Update,
    ));
    app.add_systems(Startup, setup);
    app.add_systems(Update, request_edit.run_if(in_state(DemoState::Running)));
    app.run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let editable = HalfEdgeMesh::unit_cube().expect("cube");
    let handle = meshes.add(editable.to_bevy_mesh().expect("bevy mesh"));
    commands.spawn((
        EditableMesh::new(editable),
        Mesh3d(handle.clone()),
        MeshOpsTarget::new(handle),
    ));
}

fn request_edit(
    target: Single<Entity, With<MeshOpsTarget>>,
    mut requests: MessageWriter<MeshOpsRequest>,
) {
    requests.write(MeshOpsRequest {
        entity: *target,
        command: MeshEditCommand::SubdivideCatmullClark { levels: 1 },
        prefer_async: true,
    });
}
```

## Public API

- `HalfEdgeMesh`: editable core mesh with typed IDs, boundary-face support, validation, and Bevy conversion
- `VertexId`, `HalfEdgeId`, `EdgeId`, `FaceId`: compact typed indices
- `MeshError`: fallible operation and conversion surface
- `MeshOpsPlugin`: injectable-schedule Bevy runtime layer
- `MeshOpsSystems`: public system sets for request processing, async completion, mesh sync, and optional debug draw
- `EditableMesh`, `MeshOpsTarget`, `MeshOpsDebugView`: ECS-facing components
- `MeshOpsRequest`, `MeshTopologyChanged`, `MeshOpsFailed`: message surface for runtime editing
- `MeshOpsConfig`, `MeshOpsDebugSettings`: runtime policy resources for sync, normals/tangents, async thresholds, and gizmo appearance
- `MeshBooleanConfig`, `MeshBooleanOperation`: voxel-boolean CSG controls for union, intersection, and difference on closed meshes
- `MeshBridgeConfig`, `MeshUvProjection`, `MeshUvProjectionMode`, `VertexColorPaintConfig`: higher-level modeling controls for loop bridging, UV layout, and vertex painting
- `MeshDecimationConfig`, `MeshLodConfig`, `MeshLodLevel`: pure-core simplification controls for runtime-generated LODs
- `MeshSnapshot`, `PolygonFace`: polygon snapshot surface for tests, offline tools, and deterministic rebuilds

## Supported Operations

Atomic edit layer:

- `add_face`
- `remove_face`
- `split_face`
- `poke_face`
- `flip_edge`
- `split_edge`
- `collapse_edge`

Higher-level operations:

- `extrude_faces`
- `bevel_edges`
- `split_edge_ring`
- `bridge_boundary_loops`
- `subdivide_catmull_clark`
- `subdivide_loop`
- `merge_vertices`
- `weld_by_position_and_attributes`
- `boolean_with`
- `apply_boolean`
- `decimate`
- `build_lod_chain`
- `offset_vertices`
- `paint_vertices`
- `project_uvs`
- `recompute_normals`
- `recompute_tangents`
- `triangulate_faces`
- `separate_connected_components`

Pass 1 intentionally keeps some operations narrow:

- `bevel_edges` is implemented for a single interior edge shared by two triangles and is meant for strip-style chamfering demos, cleanup, and tooling.
- Boolean CSG is currently voxelized rather than analytic: it targets closed meshes, exposes the voxel size directly, and favors robustness and game-runtime predictability over exact surface preservation.
- Triangle and quad heavy workflows are the best-covered paths today.
- Topology-changing operations rebuild the authoritative half-edge mesh from validated polygon snapshots, prioritizing correctness and clear failure modes over in-place mutation performance.

## Supported Mesh IO Constraints

- Import currently supports indexed Bevy `Mesh` values with `PrimitiveTopology::TriangleList`.
- Import welds topology by exact position equality so loop attributes can preserve UV seams and split normals. Coincident-but-disconnected surfaces are not distinguished in pass 1.
- Export preserves polygonal internal topology and triangulates deterministically for Bevy output.
- Loop attributes are stored per half-edge corner, so UV seams and hard normals can survive round-trips when the source mesh fits the supported subset.
- Vertex payload colors round-trip through `Mesh::ATTRIBUTE_COLOR` when present.

## Runtime vs Offline Usage

- Use the pure `HalfEdgeMesh` API for offline generation, preprocessing, tests, or headless tools.
- Use `MeshOpsPlugin` when an entity should own editable topology and sync changes back into `Assets<Mesh>`.
- Position-only edits can stay inline and cheap with `MeshEditCommand::OffsetVertices`.
- Heavier rebuilds such as subdivision can route through the async job path; base-revision guards prevent stale async results from overwriting newer edits.

## Examples

| Example | Description | Run |
| --- | --- | --- |
| `basic` | Pane-driven topology starter scene for repeated face pokes and smoothing | `cargo run -p saddle-procgen-mesh-ops-example-basic` |
| `csg_boolean` | Pane-driven fortress breach scene showing voxel union, intersection, and difference | `cargo run -p saddle-procgen-mesh-ops-example-csg-boolean` |
| `extrude` | Pane-driven tower-block extrusion scene with optional smoothing | `cargo run -p saddle-procgen-mesh-ops-example-extrude` |
| `subdivision` | Pane-driven side-by-side Catmull-Clark and Loop comparison | `cargo run -p saddle-procgen-mesh-ops-example-subdivision` |
| `runtime` | Runtime request-pipeline demo with live subdivision cadence controls | `cargo run -p saddle-procgen-mesh-ops-example-runtime` |
| `perf` | Interactive LOD dashboard covering export/import timings and decimation output | `cargo run -p saddle-procgen-mesh-ops-example-perf` |
| `saddle-procgen-mesh-ops-lab` | Rich crate-local BRP/E2E verification app | `cargo run -p saddle-procgen-mesh-ops-lab` |

## Bevy Notes

- Runtime-editable meshes should usually keep `RenderAssetUsages::default()` so CPU-side data remains available after extraction into the render world.
- Bevy `Mesh` mutation APIs document that modified meshes do not automatically refresh entity `Aabb` values, so the runtime sync path explicitly removes `Aabb` to force a refresh.
- `saddle-procgen-mesh-ops` avoids in-place mutation on extracted render meshes; it rebuilds or replaces CPU-side mesh data during sync instead.

## Documentation

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
- [Algorithms](docs/algorithms.md)
