use bevy::prelude::*;

use crate::{
    EdgeId, FaceId, HalfEdgeMesh, MeshBooleanConfig, MeshBooleanOperation, MeshBridgeConfig,
    MeshError, MeshUvProjection, VertexColorPaintConfig, VertexId,
};

#[derive(Debug, Clone, Reflect)]
pub enum MeshEditCommand {
    AddFace {
        vertices: Vec<VertexId>,
    },
    RemoveFace {
        face: FaceId,
    },
    SplitFace {
        face: FaceId,
        start: VertexId,
        end: VertexId,
    },
    PokeFace {
        face: FaceId,
    },
    FlipEdge {
        edge: EdgeId,
    },
    SplitEdge {
        edge: EdgeId,
    },
    CollapseEdge {
        edge: EdgeId,
    },
    ExtrudeFaces {
        faces: Vec<FaceId>,
        distance: f32,
    },
    BevelEdges {
        edges: Vec<EdgeId>,
        width: f32,
    },
    SplitEdgeRing {
        edges: Vec<EdgeId>,
        factor: f32,
    },
    SubdivideLoop {
        levels: u32,
    },
    SubdivideCatmullClark {
        levels: u32,
    },
    MergeVertices {
        tolerance: f32,
    },
    WeldByPositionAndAttributes {
        tolerance: f32,
    },
    OffsetVertices {
        vertices: Vec<VertexId>,
        offset: Vec3,
    },
    PaintVertices {
        vertices: Vec<VertexId>,
        config: VertexColorPaintConfig,
    },
    ProjectUvs {
        projection: MeshUvProjection,
    },
    BridgeBoundaryLoops {
        first_loop: usize,
        second_loop: usize,
        config: MeshBridgeConfig,
    },
    RecomputeNormals,
    RecomputeTangents,
    TriangulateFaces,
    Boolean {
        other: HalfEdgeMesh,
        operation: MeshBooleanOperation,
        config: MeshBooleanConfig,
    },
}

#[derive(Message, Debug, Clone, Reflect)]
pub struct MeshOpsRequest {
    pub entity: Entity,
    pub command: MeshEditCommand,
    pub prefer_async: bool,
}

#[derive(Message, Debug, Clone, Reflect)]
pub struct MeshTopologyChanged {
    pub entity: Entity,
    pub revision: u64,
    pub vertex_count: usize,
    pub edge_count: usize,
    pub face_count: usize,
    pub topology_changed: bool,
}

#[derive(Message, Debug, Clone, Reflect)]
pub struct MeshOpsFailed {
    pub entity: Entity,
    pub command: MeshEditCommand,
    pub error: MeshError,
}
