#![doc = include_str!("../README.md")]

mod attributes;
mod boolean;
pub mod components;
mod conversion;
pub mod debug;
mod error;
mod ids;
mod iterators;
mod mesh;
pub mod messages;
mod operations;
pub mod resources;
mod simplify;
mod systems;
mod topology;

pub use attributes::{FacePayload, LoopAttributes, VertexPayload};
pub use boolean::{MeshBooleanConfig, MeshBooleanOperation};
pub use components::{EditableMesh, MeshOpsDebugSettings, MeshOpsDebugView, MeshOpsTarget};
pub use error::MeshError;
pub use ids::{EdgeId, FaceId, HalfEdgeId, VertexId};
pub use mesh::{FaceKind, HalfEdgeMesh};
pub use messages::{MeshEditCommand, MeshOpsFailed, MeshOpsRequest, MeshTopologyChanged};
pub use operations::{
    MeshBridgeConfig, MeshUvProjection, MeshUvProjectionMode, VertexColorPaintConfig,
};
pub use resources::MeshOpsConfig;
pub use simplify::{MeshDecimationConfig, MeshLodConfig, MeshLodLevel};
pub use topology::{MeshSnapshot, PolygonFace};

use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MeshOpsSystems {
    ProcessRequests,
    FinishAsyncJobs,
    SyncMeshes,
    DebugDraw,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

#[derive(Default, Resource)]
struct MeshOpsRuntimeState {
    active: bool,
}

pub struct MeshOpsPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
    pub config: MeshOpsConfig,
    pub debug_settings: MeshOpsDebugSettings,
}

impl MeshOpsPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
            config: MeshOpsConfig::default(),
            debug_settings: MeshOpsDebugSettings::default(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }

    pub fn with_config(mut self, config: MeshOpsConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_debug_settings(mut self, debug_settings: MeshOpsDebugSettings) -> Self {
        self.debug_settings = debug_settings;
        self
    }
}

impl Default for MeshOpsPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for MeshOpsPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        if !app.world().contains_resource::<MeshOpsConfig>() {
            app.insert_resource(self.config.clone());
        }
        if !app.world().contains_resource::<MeshOpsDebugSettings>() {
            app.insert_resource(self.debug_settings.clone());
        }

        app.init_resource::<MeshOpsRuntimeState>()
            .init_resource::<systems::PendingMeshOpsJobs>()
            .add_message::<MeshOpsRequest>()
            .add_message::<MeshTopologyChanged>()
            .add_message::<MeshOpsFailed>()
            .register_type::<EdgeId>()
            .register_type::<EditableMesh>()
            .register_type::<FaceId>()
            .register_type::<HalfEdgeId>()
            .register_type::<HalfEdgeMesh>()
            .register_type::<LoopAttributes>()
            .register_type::<MeshBooleanConfig>()
            .register_type::<MeshBooleanOperation>()
            .register_type::<MeshBridgeConfig>()
            .register_type::<MeshEditCommand>()
            .register_type::<MeshOpsConfig>()
            .register_type::<MeshOpsDebugSettings>()
            .register_type::<MeshOpsDebugView>()
            .register_type::<MeshOpsFailed>()
            .register_type::<MeshOpsRequest>()
            .register_type::<MeshOpsTarget>()
            .register_type::<MeshTopologyChanged>()
            .register_type::<MeshUvProjection>()
            .register_type::<MeshUvProjectionMode>()
            .register_type::<VertexId>()
            .register_type::<VertexColorPaintConfig>()
            .register_type::<VertexPayload>()
            .add_systems(self.activate_schedule, systems::activate_runtime)
            .add_systems(self.deactivate_schedule, systems::deactivate_runtime)
            .configure_sets(
                self.update_schedule,
                (
                    MeshOpsSystems::ProcessRequests,
                    MeshOpsSystems::FinishAsyncJobs,
                    MeshOpsSystems::SyncMeshes,
                    MeshOpsSystems::DebugDraw,
                )
                    .chain(),
            )
            .add_systems(
                self.update_schedule,
                systems::process_requests
                    .in_set(MeshOpsSystems::ProcessRequests)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                systems::finish_async_jobs
                    .in_set(MeshOpsSystems::FinishAsyncJobs)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                systems::sync_mesh_assets
                    .in_set(MeshOpsSystems::SyncMeshes)
                    .run_if(systems::runtime_is_active),
            );

        if app
            .world()
            .contains_resource::<bevy::gizmos::config::GizmoConfigStore>()
        {
            app.add_systems(
                self.update_schedule,
                debug::draw_debug
                    .in_set(MeshOpsSystems::DebugDraw)
                    .run_if(systems::runtime_is_active),
            );
        }
    }
}
