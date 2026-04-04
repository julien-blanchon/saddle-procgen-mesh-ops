use bevy::prelude::*;
use thiserror::Error;

use crate::{EdgeId, FaceId, HalfEdgeId, VertexId};

#[derive(Debug, Error, Clone, PartialEq, Reflect)]
pub enum MeshError {
    #[error("invalid vertex id {0:?}")]
    InvalidVertex(VertexId),
    #[error("invalid half-edge id {0:?}")]
    InvalidHalfEdge(HalfEdgeId),
    #[error("invalid edge id {0:?}")]
    InvalidEdge(EdgeId),
    #[error("invalid face id {0:?}")]
    InvalidFace(FaceId),
    #[error("face requires at least 3 unique vertices, got {count}")]
    FaceTooSmall { count: usize },
    #[error("face contains a duplicate consecutive vertex at local corner {corner}")]
    DuplicateConsecutiveVertex { corner: usize },
    #[error("duplicate directed edge {from:?}->{to:?} would make the mesh invalid")]
    DuplicateDirectedEdge { from: VertexId, to: VertexId },
    #[error("edge {from:?}<->{to:?} would be non-manifold")]
    NonManifoldEdge { from: VertexId, to: VertexId },
    #[error("unsupported Bevy mesh primitive topology {0}")]
    UnsupportedPrimitiveTopology(String),
    #[error("missing Bevy mesh attribute {0}")]
    MissingAttribute(&'static str),
    #[error("Bevy mesh must be indexed for import")]
    MissingIndices,
    #[error("unsupported Bevy mesh layout: {0}")]
    UnsupportedMesh(String),
    #[error("operation {operation} is not supported on boundary topology in pass 1")]
    BoundaryOperation { operation: &'static str },
    #[error("operation {operation} requires triangle faces")]
    RequiresTriangleFaces { operation: &'static str },
    #[error("operation {operation} requires a closed mesh")]
    RequiresClosedMesh { operation: &'static str },
    #[error("invalid boolean config: {0}")]
    InvalidBooleanConfig(String),
    #[error("boolean voxel grid {x}x{y}x{z} exceeds configured max axis {max_axis}")]
    BooleanGridTooDense {
        x: u32,
        y: u32,
        z: u32,
        max_axis: u32,
    },
    #[error("operation {operation} is not implemented for this selection in pass 1: {detail}")]
    UnsupportedOperation {
        operation: &'static str,
        detail: String,
    },
    #[error("face {0:?} is degenerate")]
    DegenerateFace(FaceId),
    #[error("operation would create invalid topology: {0}")]
    InvalidTopology(&'static str),
    #[error("deterministic triangulation failed for face {0:?}")]
    TriangulationFailed(FaceId),
    #[error("mesh validation failed: {0}")]
    Validation(String),
    #[error("selection for {0} is empty")]
    EmptySelection(&'static str),
    #[error("entity already has a pending async mesh job")]
    PendingAsyncJob,
}
