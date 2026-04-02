# Algorithms

## Implemented in Pass 1

### Topology / Validation

- arena-backed half-edge storage with typed IDs
- explicit ghost-boundary representation
- storage validation
- manifold validation
- connected-component detection
- boundary-loop extraction

### Local / Atomic Edits

- `add_face`
- `remove_face`
- `split_face`
- `poke_face`
- `flip_edge`
- `split_edge`
- `collapse_edge`

These operations rebuild through `MeshSnapshot` where that keeps the code simpler and easier to validate.

### High-Level Edits

- `extrude_faces`
- `bevel_edges`
- `split_edge_ring`
- `subdivide_catmull_clark`
- `subdivide_loop`
- `merge_vertices`
- `weld_by_position_and_attributes`
- `offset_vertices`
- `triangulate_faces`
- `separate_connected_components`

## Scope Notes

- `bevel_edges` is intentionally narrow in pass 1:
  - one interior edge
  - shared by two triangles
- `extrude_faces` rejects adjacent selected faces for now.
- Import/export coverage is strongest for triangle and quad heavy meshes.

## Complexity Notes

Approximate pass-1 costs:

- validation: `O(V + E + F)`
- conversion to/from snapshot: `O(V + E + F)`
- `flip_edge`: `O(F)` because the rebuilt mesh is validated after the local edit
- `split_edge`: `O(F)` via snapshot rebuild
- `collapse_edge`: `O(F)` via snapshot rebuild and cleanup
- `extrude_faces`: `O(F + selected_face_corners)`
- `subdivide_catmull_clark`: `O(V + E + F)` per level
- `subdivide_loop`: `O(V + E + F)` per level on triangle meshes
- `recompute_normals`: `O(V + F)`
- `recompute_tangents`: `O(face corners)`

## Attribute Policy

- Vertex payloads are interpolated when new vertices are created.
- Loop UVs / normals / tangents are preserved when possible and regenerated when necessary.
- `weld_by_position_and_attributes` compares both vertex payloads and the sorted set of incident loop/corner attributes before merging coincident vertices.
- If preserving a loop-attribute seam would leave only part of a duplicated edge welded, the weld is rejected instead of creating a disconnected vertex fan.
- Export uses one Bevy vertex per face corner so loop attributes survive seams.

## Performance Checks

- Use `cargo run -p saddle-procgen-mesh-ops-example-perf` for a lightweight timing pass over import/export and Catmull-Clark subdivision.
- Heavy runtime edits should still go through explicit requests or async jobs; the perf probe is for regression detection, not for frame-by-frame use.

## Not Implemented Yet

Future extension space intentionally left open:

- QEM simplification / runtime LOD
- hole filling
- isotropic remeshing
- ray queries / BVH
- collider export helpers
- progressive mesh metadata
- transactions / undo diffs
