use bevy::{ecs::message::Messages, prelude::*};

use super::*;
use crate::{
    EditableMesh, MeshBooleanConfig, MeshBooleanOperation, MeshOpsPlugin, MeshOpsRequest,
    MeshOpsTarget, VertexId,
};

fn setup_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(Assets::<Mesh>::default());
    app.add_plugins(MeshOpsPlugin::default());

    let entity = spawn_target(
        &mut app,
        "MeshOps Test Entity",
        HalfEdgeMesh::unit_cube().expect("cube"),
    );

    (app, entity)
}

fn spawn_target(app: &mut App, name: &str, mesh: HalfEdgeMesh) -> Entity {
    let handle = {
        let mut assets = app.world_mut().resource_mut::<Assets<Mesh>>();
        assets.add(mesh.to_bevy_mesh().expect("bevy mesh"))
    };

    app.world_mut()
        .spawn((
            Name::new(name.to_owned()),
            EditableMesh::new(mesh),
            Mesh3d(handle.clone()),
            MeshOpsTarget::new(handle),
        ))
        .id()
}

#[test]
fn request_flow_marks_dirty_and_syncs_the_mesh() {
    let (mut app, entity) = setup_app();
    app.update();

    app.world_mut().write_message(MeshOpsRequest {
        entity,
        command: MeshEditCommand::SubdivideCatmullClark { levels: 1 },
        prefer_async: false,
    });
    app.update();

    let target = app
        .world()
        .get::<MeshOpsTarget>(entity)
        .expect("target after update");
    let editable = app
        .world()
        .get::<EditableMesh>(entity)
        .expect("editable after update");

    assert_eq!(editable.mesh.face_count(), 24);
    assert_eq!(target.synced_revision, editable.revision);
    assert!(!target.dirty);
}

#[test]
fn stale_async_result_does_not_overwrite_newer_revision() {
    let (mut app, entity) = setup_app();
    app.update();

    {
        let mut editable = app
            .world_mut()
            .get_mut::<EditableMesh>(entity)
            .expect("editable");
        editable.mark_changed(false);
        editable
            .mesh
            .offset_vertices(&[VertexId(0)], Vec3::new(0.0, 0.2, 0.0))
            .expect("offset");
    }

    let stale_mesh = HalfEdgeMesh::unit_cube().expect("cube");
    let task = AsyncComputeTaskPool::get().spawn(async move { Ok((stale_mesh, true)) });
    app.world_mut()
        .resource_mut::<PendingMeshOpsJobs>()
        .jobs
        .push(PendingMeshJob {
            entity,
            base_revision: 0,
            command: MeshEditCommand::SubdivideCatmullClark { levels: 1 },
            task,
        });

    app.update();

    let editable = app
        .world()
        .get::<EditableMesh>(entity)
        .expect("editable after stale async");
    assert_eq!(editable.revision, 1);
    assert_eq!(editable.mesh.face_count(), 6);
}

#[test]
fn sync_system_skips_rebuilds_when_nothing_changed() {
    let (mut app, entity) = setup_app();
    app.update();

    let initial = app
        .world()
        .get::<MeshOpsTarget>(entity)
        .expect("target")
        .synced_revision;
    app.update();
    let target = app
        .world()
        .get::<MeshOpsTarget>(entity)
        .expect("target after second update");

    assert_eq!(target.synced_revision, initial);
    assert!(!target.dirty);
    assert!(
        app.world()
            .contains_resource::<Messages<MeshTopologyChanged>>()
    );
}

#[test]
fn multiple_entities_edit_independently() {
    let (mut app, entity_a) = setup_app();
    let entity_b = spawn_target(
        &mut app,
        "MeshOps Test Entity B",
        HalfEdgeMesh::unit_cube().expect("cube"),
    );
    app.update();

    app.world_mut().write_message(MeshOpsRequest {
        entity: entity_a,
        command: MeshEditCommand::SubdivideCatmullClark { levels: 1 },
        prefer_async: false,
    });
    app.update();

    let editable_a = app
        .world()
        .get::<EditableMesh>(entity_a)
        .expect("entity a editable");
    let editable_b = app
        .world()
        .get::<EditableMesh>(entity_b)
        .expect("entity b editable");

    assert_eq!(editable_a.mesh.face_count(), 24);
    assert_eq!(editable_b.mesh.face_count(), 6);
    assert!(editable_a.revision > editable_b.revision);
}

#[test]
fn boolean_requests_rebuild_the_target_mesh() {
    let (mut app, entity) = setup_app();
    app.update();

    let mut other = HalfEdgeMesh::unit_cube().expect("cube");
    let vertices = other.vertex_ids().collect::<Vec<_>>();
    other
        .offset_vertices(&vertices, Vec3::new(0.35, 0.2, 0.0))
        .expect("offset");

    app.world_mut().write_message(MeshOpsRequest {
        entity,
        command: MeshEditCommand::Boolean {
            other,
            operation: MeshBooleanOperation::Difference,
            config: MeshBooleanConfig {
                voxel_size: 0.12,
                padding_voxels: 1,
                max_cells_per_axis: 32,
            },
        },
        prefer_async: false,
    });
    app.update();

    let editable = app
        .world()
        .get::<EditableMesh>(entity)
        .expect("editable after boolean");
    assert!(editable.mesh.is_closed());
    assert!(editable.mesh.face_count() > 6);
}
