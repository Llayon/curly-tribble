/// Data types for the hex face topology sub-module.
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct FaceTopologyTypesPlugin;
impl Plugin for FaceTopologyTypesPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VertexId(usize);

impl VertexId {
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HalfEdgeId(usize);

impl HalfEdgeId {
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FaceId(usize);

impl FaceId {
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SharedCornerKey(HexCoord, HexCoord, HexCoord);

impl SharedCornerKey {
    #[must_use]
    pub const fn new(c0: HexCoord, c1: HexCoord, c2: HexCoord) -> Self {
        Self(c0, c1, c2)
    }
    #[must_use]
    pub const fn first(self) -> HexCoord {
        self.0
    }
    #[must_use]
    pub const fn second(self) -> HexCoord {
        self.1
    }
    #[must_use]
    pub const fn third(self) -> HexCoord {
        self.2
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapVertex {
    pub position: Vec2,
    pub canonical_key: SharedCornerKey,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HalfEdge {
    pub origin: VertexId,
    pub destination: VertexId,
    pub next: HalfEdgeId,
    pub prev: HalfEdgeId,
    pub twin: Option<HalfEdgeId>,
    pub incident_face: FaceId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HexFace {
    pub hex: HexCoord,
    pub boundary: HalfEdgeId,
    pub vertices: [VertexId; 6],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TopologyStats {
    pub reduced_displacement_fallbacks: usize,
    pub regular_position_fallbacks: usize,
    pub min_face_area: f32,
    pub max_face_area: f32,
    pub min_edge_length: f32,
    pub half_edge_count: usize,
    pub paired_edge_count: usize,
    pub border_edge_count: usize,
}

#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct HexFaceTopology {
    pub vertices: Vec<MapVertex>,
    pub half_edges: Vec<HalfEdge>,
    pub faces: Vec<HexFace>,
    pub hex_to_face: HashMap<HexCoord, FaceId>,
    pub stats: TopologyStats,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HexFaceTopologyError {
    EmptyMap,
    DuplicateCornerKey(SharedCornerKey),
    CornerKeyMismatch(SharedCornerKey),
    NonConvexFace(FaceId),
    SelfIntersectingFace(FaceId),
    InvalidWinding(FaceId),
    NearZeroEdge { face: FaceId, edge: HalfEdgeId },
    NonPositiveArea(FaceId),
    InconsistentTwin { edge: HalfEdgeId, twin: HalfEdgeId },
    DisplacementFailed(FaceId),
    ValidationFailed(String),
}
