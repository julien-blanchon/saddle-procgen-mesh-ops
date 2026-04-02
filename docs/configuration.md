# Configuration

## `MeshOpsConfig`

| Field | Type | Default | Range / Guidance | Effect | Performance Impact |
| --- | --- | --- | --- | --- | --- |
| `async_face_threshold` | `usize` | `24` | `0..` | Minimum face count before subdivision requests are eligible for async execution by default | Lower values move more work off-thread but increase task overhead |
| `allow_async_subdivision` | `bool` | `true` | `true` / `false` | Enables the async path for `SubdivideCatmullClark` and `SubdivideLoop` | Off-thread work helps frame time on heavy meshes |
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

## Runtime Ownership Guidance

- Editable runtime meshes should normally use `RenderAssetUsages::default()`.
- If the mesh must keep being editable after extraction, keep the asset in both worlds.
- A render-world-only asset is appropriate only for baked outputs that the CPU no longer needs to inspect or modify.
