use bevy::{
    camera::primitives::Aabb,
    mesh::Mesh3d,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};

use crate::{
    HalfEdgeMesh, MeshError, MeshOpsRuntimeState,
    components::{EditableMesh, MeshOpsTarget},
    messages::{MeshEditCommand, MeshOpsFailed, MeshOpsRequest, MeshTopologyChanged},
    resources::MeshOpsConfig,
};

#[derive(Default, Resource)]
pub(crate) struct PendingMeshOpsJobs {
    jobs: Vec<PendingMeshJob>,
}

struct PendingMeshJob {
    entity: Entity,
    base_revision: u64,
    command: MeshEditCommand,
    task: Task<Result<(HalfEdgeMesh, bool), MeshError>>,
}

pub(crate) fn activate_runtime(mut runtime: ResMut<MeshOpsRuntimeState>) {
    runtime.active = true;
}

pub(crate) fn deactivate_runtime(mut runtime: ResMut<MeshOpsRuntimeState>) {
    runtime.active = false;
}

pub(crate) fn runtime_is_active(runtime: Option<Res<MeshOpsRuntimeState>>) -> bool {
    runtime.is_some_and(|runtime| runtime.active)
}

pub(crate) fn process_requests(
    mut requests: MessageReader<MeshOpsRequest>,
    mut meshes: Query<(Entity, &mut EditableMesh, &mut MeshOpsTarget)>,
    mut changed: MessageWriter<MeshTopologyChanged>,
    mut failed: MessageWriter<MeshOpsFailed>,
    config: Res<MeshOpsConfig>,
    mut pending: ResMut<PendingMeshOpsJobs>,
) {
    for request in requests.read() {
        let Ok((entity, mut editable, mut target)) = meshes.get_mut(request.entity) else {
            continue;
        };

        if should_run_async(
            &request.command,
            editable.mesh.face_count(),
            &config,
            request.prefer_async,
        ) {
            if pending.jobs.iter().any(|job| job.entity == entity) {
                failed.write(MeshOpsFailed {
                    entity,
                    command: request.command.clone(),
                    error: MeshError::PendingAsyncJob,
                });
                continue;
            }

            let mesh = editable.mesh.clone();
            let command = request.command.clone();
            let task_command = command.clone();
            let config_snapshot = config.clone();
            let base_revision = editable.revision;
            let task = AsyncComputeTaskPool::get().spawn(async move {
                let mut mesh = mesh;
                let topology_changed = apply_command(&mut mesh, &task_command)?;
                postprocess(&mut mesh, topology_changed, &config_snapshot)?;
                Ok((mesh, topology_changed))
            });

            pending.jobs.push(PendingMeshJob {
                entity,
                base_revision,
                command,
                task,
            });
            continue;
        }

        match apply_command(&mut editable.mesh, &request.command).and_then(|topology_changed| {
            postprocess(&mut editable.mesh, topology_changed, &config)?;
            Ok(topology_changed)
        }) {
            Ok(topology_changed) => {
                editable.mark_changed(topology_changed);
                target.dirty = true;
                changed.write(MeshTopologyChanged {
                    entity,
                    revision: editable.revision,
                    vertex_count: editable.mesh.vertex_count(),
                    edge_count: editable.mesh.edge_count(),
                    face_count: editable.mesh.face_count(),
                    topology_changed,
                });
            }
            Err(error) => {
                failed.write(MeshOpsFailed {
                    entity,
                    command: request.command.clone(),
                    error,
                });
            }
        }
    }
}

pub(crate) fn finish_async_jobs(
    mut meshes: Query<(Entity, &mut EditableMesh, &mut MeshOpsTarget)>,
    mut changed: MessageWriter<MeshTopologyChanged>,
    mut failed: MessageWriter<MeshOpsFailed>,
    mut pending: ResMut<PendingMeshOpsJobs>,
) {
    let mut finished = Vec::new();
    for (index, job) in pending.jobs.iter_mut().enumerate() {
        let Some(result) = check_ready(&mut job.task) else {
            continue;
        };
        finished.push(index);

        let Ok((entity, mut editable, mut target)) = meshes.get_mut(job.entity) else {
            continue;
        };

        if editable.revision != job.base_revision {
            continue;
        }

        match result {
            Ok((mesh, topology_changed)) => {
                editable.mesh = mesh;
                editable.mark_changed(topology_changed);
                target.dirty = true;
                changed.write(MeshTopologyChanged {
                    entity,
                    revision: editable.revision,
                    vertex_count: editable.mesh.vertex_count(),
                    edge_count: editable.mesh.edge_count(),
                    face_count: editable.mesh.face_count(),
                    topology_changed,
                });
            }
            Err(error) => {
                failed.write(MeshOpsFailed {
                    entity,
                    command: job.command.clone(),
                    error,
                });
            }
        }
    }

    for index in finished.into_iter().rev() {
        pending.jobs.remove(index);
    }
}

pub(crate) fn sync_mesh_assets(
    mut commands: Commands,
    mut assets: ResMut<Assets<Mesh>>,
    config: Res<MeshOpsConfig>,
    mut failed: MessageWriter<MeshOpsFailed>,
    mut query: Query<(
        Entity,
        &mut EditableMesh,
        &mut MeshOpsTarget,
        Option<&Mesh3d>,
    )>,
) {
    for (entity, mut editable, mut target, current_mesh) in &mut query {
        if !target.dirty && target.synced_revision == editable.revision {
            continue;
        }

        match editable.mesh.to_bevy_mesh() {
            Ok(mesh) => {
                if let Some(existing) = assets.get_mut(&target.mesh_handle) {
                    *existing = mesh;
                } else {
                    target.mesh_handle = assets.add(mesh);
                }

                if current_mesh.map(|mesh| mesh.0.id()) != Some(target.mesh_handle.id()) {
                    commands
                        .entity(entity)
                        .insert(Mesh3d(target.mesh_handle.clone()));
                }
                if config.refresh_aabb_on_sync {
                    commands.entity(entity).remove::<Aabb>();
                }

                target.synced_revision = editable.revision;
                target.dirty = false;
                editable.topology_dirty = false;
            }
            Err(error) => {
                failed.write(MeshOpsFailed {
                    entity,
                    command: MeshEditCommand::RecomputeNormals,
                    error,
                });
            }
        }
    }
}

fn should_run_async(
    command: &MeshEditCommand,
    face_count: usize,
    config: &MeshOpsConfig,
    prefer_async: bool,
) -> bool {
    match command {
        MeshEditCommand::SubdivideCatmullClark { .. } | MeshEditCommand::SubdivideLoop { .. } => {
            if !config.allow_async_subdivision {
                return false;
            }
            prefer_async || face_count >= config.async_face_threshold
        }
        MeshEditCommand::Boolean { other, .. } => {
            if !config.allow_async_boolean_ops {
                return false;
            }
            prefer_async || face_count + other.face_count() >= config.boolean_async_face_threshold
        }
        _ => false,
    }
}

fn postprocess(
    mesh: &mut HalfEdgeMesh,
    topology_changed: bool,
    config: &MeshOpsConfig,
) -> Result<(), MeshError> {
    if topology_changed && config.recompute_normals_after_topology_change {
        mesh.recompute_normals()?;
    }
    if topology_changed && config.recompute_tangents_after_topology_change && mesh.has_loop_uvs() {
        mesh.recompute_tangents()?;
    }
    Ok(())
}

fn apply_command(mesh: &mut HalfEdgeMesh, command: &MeshEditCommand) -> Result<bool, MeshError> {
    match command {
        MeshEditCommand::AddFace { vertices } => {
            mesh.add_face(vertices)?;
            Ok(true)
        }
        MeshEditCommand::RemoveFace { face } => {
            mesh.remove_face(*face)?;
            Ok(true)
        }
        MeshEditCommand::SplitFace { face, start, end } => {
            mesh.split_face(*face, *start, *end)?;
            Ok(true)
        }
        MeshEditCommand::PokeFace { face } => {
            mesh.poke_face(*face)?;
            Ok(true)
        }
        MeshEditCommand::FlipEdge { edge } => {
            mesh.flip_edge(*edge)?;
            Ok(true)
        }
        MeshEditCommand::SplitEdge { edge } => {
            mesh.split_edge(*edge)?;
            Ok(true)
        }
        MeshEditCommand::CollapseEdge { edge } => {
            mesh.collapse_edge(*edge)?;
            Ok(true)
        }
        MeshEditCommand::ExtrudeFaces { faces, distance } => {
            mesh.extrude_faces(faces, *distance)?;
            Ok(true)
        }
        MeshEditCommand::BevelEdges { edges, width } => {
            mesh.bevel_edges(edges, *width)?;
            Ok(true)
        }
        MeshEditCommand::SplitEdgeRing { edges, factor } => {
            mesh.split_edge_ring(edges, *factor)?;
            Ok(true)
        }
        MeshEditCommand::SubdivideLoop { levels } => {
            mesh.subdivide_loop(*levels)?;
            Ok(true)
        }
        MeshEditCommand::SubdivideCatmullClark { levels } => {
            mesh.subdivide_catmull_clark(*levels)?;
            Ok(true)
        }
        MeshEditCommand::MergeVertices { tolerance } => Ok(mesh.merge_vertices(*tolerance)? > 0),
        MeshEditCommand::WeldByPositionAndAttributes { tolerance } => {
            Ok(mesh.weld_by_position_and_attributes(*tolerance)? > 0)
        }
        MeshEditCommand::OffsetVertices { vertices, offset } => {
            mesh.offset_vertices(vertices, *offset)?;
            Ok(false)
        }
        MeshEditCommand::PaintVertices { vertices, config } => {
            mesh.paint_vertices(vertices, config)?;
            Ok(false)
        }
        MeshEditCommand::ProjectUvs { projection } => {
            mesh.project_uvs(projection)?;
            if mesh.has_loop_normals() {
                let _ = mesh.recompute_tangents();
            }
            Ok(false)
        }
        MeshEditCommand::BridgeBoundaryLoops {
            first_loop,
            second_loop,
            config,
        } => {
            mesh.bridge_boundary_loops(*first_loop, *second_loop, config)?;
            Ok(true)
        }
        MeshEditCommand::RecomputeNormals => {
            mesh.recompute_normals()?;
            Ok(false)
        }
        MeshEditCommand::RecomputeTangents => {
            mesh.recompute_tangents()?;
            Ok(false)
        }
        MeshEditCommand::TriangulateFaces => {
            mesh.triangulate_faces()?;
            Ok(true)
        }
        MeshEditCommand::Boolean {
            other,
            operation,
            config,
        } => {
            mesh.apply_boolean(other, *operation, config)?;
            Ok(true)
        }
    }
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
