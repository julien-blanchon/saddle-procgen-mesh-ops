# Configuration

## `MeshOpsConfig`

| Field | Type | Default | Range / Guidance | Effect | Performance Impact |
| --- | --- | --- | --- | --- | --- |
| `async_face_threshold` | `usize` | `24` | `0..` | Minimum face count before subdivision requests are eligible for async execution by default | Lower values move more work off-thread but increase task overhead |
| `allow_async_subdivision` | `bool` | `true` | `true` / `false` | Enables the async path for `SubdivideCatmullClark` and `SubdivideLoop` | Off-thread work helps frame time on heavy meshes |
| `boolean_async_face_threshold` | `usize` | `48` | `0..` | Minimum combined face count before boolean requests are eligible for async execution by default | Lower values move more CSG work off-thread |
| `allow_async_boolean_ops` | `bool` | `true` | `true` / `false` | Enables the async path for `MeshEditCommand::Boolean` | Useful for larger runtime CSG passes |
| `recompute_normals_after_topology_change` | `bool` | `true` | `true` / `false` | Rebuilds loop normals after successful topology-changing requests | Adds extra CPU work after edits |
| `recompute_tangents_after_topology_change` | `bool` | `true` | `true` / `false` | Rebuilds tangents after successful topology-changing requests when UVs exist | Usually cheap relative to subdivision, but still extra work |
| `refresh_aabb_on_sync` | `bool` | `true` | `true` / `false` | Removes `Aabb` after mesh sync so Bevy recalculates bounds | Very small cost; recommended for editable meshes |

## `MeshOpsDebugSettings`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `edge_color` | `Color` | pale blue | Color for ordinary mesh edges |
| `boundary_edge_color` | `Color` | warm orange | Color for boundary edges |
| `face_normal_color` | `Color` | green | Color for face normal lines |
| `vertex_normal_color` | `Color` | yellow | Color for vertex normal lines |
| `normal_length` | `f32` | `0.24` | World-space length of normal gizmos |

## `MeshOpsDebugView`

Per-entity debug toggle component.

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `enabled` | `bool` | `false` | Master switch for debug drawing on that entity |
| `draw_edges` | `bool` | `true` | Draw ordinary edges |
| `draw_boundary_edges` | `bool` | `true` | Draw boundary edges separately |
| `draw_face_normals` | `bool` | `false` | Draw one normal per face |
| `draw_vertex_normals` | `bool` | `false` | Draw averaged vertex normals |

## `MeshBooleanConfig`

Voxelized boolean controls for `HalfEdgeMesh::boolean_with` and `HalfEdgeMesh::apply_boolean`.

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `voxel_size` | `f32` | `0.12` | World-space size of the occupancy grid cells used to approximate the boolean surface |
| `padding_voxels` | `u32` | `1` | Empty border cells around the sampled region so exposed faces are reconstructed cleanly |
| `max_cells_per_axis` | `u32` | `48` | Safety cap that prevents accidentally sampling a prohibitively dense voxel grid |

Smaller `voxel_size` values preserve more detail but cost more CPU and memory.

## `MeshBridgeConfig`

Controls `HalfEdgeMesh::bridge_boundary_loops` and `MeshEditCommand::BridgeBoundaryLoops`.

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `twist_offset` | `usize` | `0` | Rotates how the second loop is paired against the first before the bridge quads are authored |

Pass 1 expects two boundary loops with matching corner counts.

## `MeshUvProjection`

Controls `HalfEdgeMesh::project_uvs` and `MeshEditCommand::ProjectUvs`.

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `mode` | `MeshUvProjectionMode` | `PlanarXY` | Chooses XY, XZ, YZ, or dominant-axis box projection |
| `scale` | `Vec2` | `1,1` | Multiplies the projected coordinates before they are written as UVs |
| `offset` | `Vec2` | `0,0` | Adds a constant UV offset after projection |

### `MeshUvProjectionMode`

- `PlanarXY`
- `PlanarXZ`
- `PlanarYZ`
- `Box`

## `VertexColorPaintConfig`

Controls `HalfEdgeMesh::paint_vertices` and `MeshEditCommand::PaintVertices`.

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `color` | `Vec4` | `1,1,1,1` | Target RGBA color stored on each selected vertex payload |
| `blend` | `f32` | `1.0` | Lerp factor applied between the current color and the new target color |

## `MeshDecimationConfig`

Pure-core simplification settings for `HalfEdgeMesh::decimate`.

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `target_face_count` | `usize` | `12` | Stops collapsing edges once the mesh reaches this many interior faces |
| `preserve_boundary` | `bool` | `true` | Skips boundary edges so open surfaces keep their authored silhouette |
| `minimum_edge_length` | `f32` | `0.0` | Rejects collapses on very short edges when you want to preserve dense details |
| `max_iterations` | `usize` | `256` | Safety cap for greedy edge-collapse attempts |

## `MeshLodConfig`

Batch simplification settings for `HalfEdgeMesh::build_lod_chain`.

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `level_count` | `u32` | `3` | Maximum number of LOD levels to build, including the source mesh as level 0 |
| `reduction_ratio` | `f32` | `0.6` | Target face-count multiplier applied from one LOD level to the next |
| `minimum_face_count` | `usize` | `6` | Lower bound that prevents the chain from collapsing below a usable mesh |
| `preserve_boundary` | `bool` | `true` | Reuses the same boundary-preservation policy as `MeshDecimationConfig` |
| `minimum_edge_length` | `f32` | `0.0` | Passed through to each decimation pass so tiny features can be protected |
| `max_iterations_per_level` | `usize` | `256` | Safety cap for each level's greedy collapse pass |

## Runtime Ownership Guidance

- Editable runtime meshes should normally use `RenderAssetUsages::default()`.
- If the mesh must keep being editable after extraction, keep the asset in both worlds.
- A render-world-only asset is appropriate only for baked outputs that the CPU no longer needs to inspect or modify.
