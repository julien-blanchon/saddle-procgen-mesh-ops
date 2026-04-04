# Architecture

## Layers

`saddle-procgen-mesh-ops` is split into two layers:

1. Pure core:
   - dense typed-ID half-edge/DCEL-style topology
   - polygon snapshot rebuild path for complex edits
   - voxel boolean CSG for closed-mesh union / intersection / difference
   - validation, traversal, conversion helpers
2. Thin Bevy runtime:
   - `EditableMesh` + `MeshOpsTarget` components
   - `MeshOpsRequest` / `MeshTopologyChanged` / `MeshOpsFailed` messages
   - async subdivision / boolean handoff
   - sync back to `Assets<Mesh>`
   - optional gizmo debug draw

## Storage Layout

- Vertices, half-edges, edges, and faces are stored in dense `Vec` arenas.
- Public IDs are typed `u32` newtypes: `VertexId`, `HalfEdgeId`, `EdgeId`, `FaceId`.
- Each half-edge stores:
  - `origin`
  - `twin`
  - `next`
  - `prev`
  - `face`
  - `edge`
  - per-loop attributes (`uv`, `normal`, `tangent`)
- Each vertex stores one outgoing half-edge plus the per-vertex payload (`position`, optional weight/tag).
- Each face stores one representative half-edge, `FaceKind`, and optional material/region metadata.

## Boundary Representation

- Boundary edges are represented by ghost boundary half-edges and explicit boundary faces.
- This keeps hot traversal paths simple:
  - every half-edge always has a twin
  - boundary detection is a face-kind query, not an `Option`
- `boundary_loops()` walks those explicit boundary faces and returns loop half-edge IDs.

## Invariants

`validate()` checks both storage integrity and manifold expectations.

Storage checks:

- every stored ID reference is in range
- every half-edge has a valid twin
- `next` / `prev` rings close correctly
- every face loop points back to the correct face
- every vertex’s outgoing half-edge really originates at that vertex

Manifold checks:

- each edge has two distinct incident faces
- one-ring traversal around a vertex visits the same number of outgoing half-edges as a raw scan

## Attribute Layers

- Vertex payloads:
  - position
  - optional scalar weight
  - optional tag
- Loop payloads:
  - UV
  - normal
  - tangent with handedness sign
- Face payloads:
  - optional material ID
  - optional region ID

Loop attributes are first-class because UV seams, hard normals, and tangents are corner data, not strictly vertex data.

## Edit Pipeline

- Small, local operations either mutate payloads directly or edit a `MeshSnapshot`.
- Snapshot edits rebuild the half-edge topology with `HalfEdgeMesh::from_snapshot(...)`.
- Boolean CSG samples closed operands into a bounded voxel grid, reconstructs exposed quads, and then rebuilds through the same validated snapshot path.
- Rebuilds re-run validation immediately, so broken edits fail fast.

This pass intentionally favors:

- deterministic rebuilds
- simple reasoning
- good tests

over:

- maximum in-place mutation performance
- non-manifold feature breadth

## Import / Export

Import:

- accepts indexed `Mesh` values with `PrimitiveTopology::TriangleList`
- welds shared topology by exact position equality
- preserves normals/UVs/tangents as loop attributes where present

Export:

- clones the editable mesh
- recomputes normals/tangents on demand when the source lacks them
- triangulates deterministically with a fan per polygon face
- emits one Bevy vertex per face corner so seams survive export

## ECS Runtime Flow

1. `MeshOpsRequest` arrives for an entity with `EditableMesh` + `MeshOpsTarget`.
2. Light operations run inline in `MeshOpsSystems::ProcessRequests`.
3. Heavy subdivision requests may spawn async jobs.
4. `MeshTopologyChanged` is emitted on success, `MeshOpsFailed` on failure.
5. `MeshOpsSystems::SyncMeshes` rebuilds the Bevy `Mesh` only when the editable mesh revision changed.
6. Sync clears the dirty flag and removes the entity `Aabb` when configured so bounds are refreshed.

## Async Job Flow

- Async work is used only for selected heavy operations, currently subdivision.
- Boolean requests can also move off-thread when their configured face threshold is exceeded.
- Each job stores the mesh revision it was spawned from.
- On completion:
  - if the entity still exists and the revision still matches, the result is applied
  - if the entity changed in the meantime, the async result is dropped as stale

This avoids the common failure mode where slow jobs overwrite newer edits.
