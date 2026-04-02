# Mesh Ops Lab

Crate-local standalone lab for validating the shared `saddle-procgen-mesh-ops` crate through the Bevy plugin path, BRP inspection, and crate-local E2E scenarios.

## Purpose

- exercise the shared runtime instead of only the pure `HalfEdgeMesh` API
- keep canonical demo targets for extrusion, beveling, subdivision, and repeated deformation
- expose a BRP-queryable `LabDiagnostics` resource so the lab state is inspectable without reading source

## Status

Working

## Run

```bash
cargo run -p saddle-procgen-mesh-ops-lab
```

Keyboard shortcuts:

- `E`: extrude the demo cube
- `B`: bevel the strip demo
- `S`: run Catmull-Clark on the subdivision target
- `C`: apply one crater/deformation step to the patch target

## E2E

```bash
cargo run -p saddle-procgen-mesh-ops-lab --features e2e -- mesh_ops_smoke
cargo run -p saddle-procgen-mesh-ops-lab --features e2e -- mesh_ops_subdivision_compare
cargo run -p saddle-procgen-mesh-ops-lab --features e2e -- mesh_ops_extrude_and_bevel
cargo run -p saddle-procgen-mesh-ops-lab --features e2e -- mesh_ops_runtime_crater
```

## BRP

```bash
uv run --project .codex/skills/bevy-brp/script brp app launch saddle-procgen-mesh-ops-lab
uv run --project .codex/skills/bevy-brp/script brp resource get saddle_procgen_mesh_ops_lab::LabDiagnostics
uv run --project .codex/skills/bevy-brp/script brp resource get saddle_procgen_mesh_ops_lab::LabControl
uv run --project .codex/skills/bevy-brp/script brp world query saddle_procgen_mesh_ops::components::MeshOpsTarget
uv run --project .codex/skills/bevy-brp/script brp world query saddle_procgen_mesh_ops::components::EditableMesh
uv run --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/saddle_procgen_mesh_ops_lab.png
uv run --project .codex/skills/bevy-brp/script brp extras shutdown
```
