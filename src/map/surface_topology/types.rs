// src/map/surface_topology/types.rs
//! Semantic surface topology model, typed IDs, provenance types, and error definitions.

use crate::map::face_topology::{FaceId, VertexId};
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct SurfaceTopologyTypesPlugin;

impl Plugin for SurfaceTopologyTypesPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfaceVertexId(usize);

impl SurfaceVertexId {
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
pub struct SurfaceHalfEdgeId(usize);

impl SurfaceHalfEdgeId {
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
pub struct SurfaceFaceId(usize);

impl SurfaceFaceId {
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceVertexSource {
    HexCorner {
        source_vertex: VertexId,
    },
    HexEdgeMidpoint {
        source_a: VertexId,
        source_b: VertexId,
    },
    HexCenter {
        hex: HexCoord,
    },
    HexRadialMidpoint {
        hex: HexCoord,
        source_corner: VertexId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceFaceSource {
    pub sector: u8,
    pub triangle: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceVertex {
    pub position: Vec2,
    pub source: SurfaceVertexSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceFace {
    pub vertices: [SurfaceVertexId; 3],
    pub boundary: SurfaceHalfEdgeId,
    pub owner_hex: HexCoord,
    pub source: SurfaceFaceSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceHalfEdge {
    pub origin: SurfaceVertexId,
    pub destination: SurfaceVertexId,
    pub next: SurfaceHalfEdgeId,
    pub prev: SurfaceHalfEdgeId,
    pub twin: Option<SurfaceHalfEdgeId>,
    pub incident_face: SurfaceFaceId,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SurfaceTopologyStats {
    pub vertex_count: usize,
    pub face_count: usize,
    pub half_edge_count: usize,
    pub paired_half_edge_count: usize,
    pub boundary_half_edge_count: usize,
}

#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct SurfaceTopology {
    pub vertices: Vec<SurfaceVertex>,
    pub half_edges: Vec<SurfaceHalfEdge>,
    pub faces: Vec<SurfaceFace>,
    pub hex_to_faces: HashMap<HexCoord, Vec<SurfaceFaceId>>,
    pub stats: SurfaceTopologyStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceTopologyError {
    MissingFaceForTile(HexCoord),
    InvalidSourceFace {
        hex: HexCoord,
        face: FaceId,
    },
    InvalidSourceVertex {
        face: HexCoord,
        vertex: VertexId,
    },
    FaceHexMismatch {
        expected: HexCoord,
        actual: HexCoord,
    },
    NonFiniteSourceVertex(VertexId),
    InvalidSurfaceVertex(SurfaceVertexId),
    InvalidSurfaceFace(SurfaceFaceId),
    InvalidSurfaceHalfEdge(SurfaceHalfEdgeId),
    DegenerateTriangle {
        hex: HexCoord,
        sector: u8,
        triangle: u8,
    },
    InvalidFaceBoundary {
        face: SurfaceFaceId,
        edge: SurfaceHalfEdgeId,
    },
    InvalidHalfEdgeCycle {
        face: SurfaceFaceId,
    },
    HalfEdgeFaceMismatch {
        edge: SurfaceHalfEdgeId,
        expected: SurfaceFaceId,
        actual: SurfaceFaceId,
    },
    TwinMismatch {
        edge: SurfaceHalfEdgeId,
        twin: SurfaceHalfEdgeId,
    },
    TwinOrientationMismatch {
        edge: SurfaceHalfEdgeId,
        twin: SurfaceHalfEdgeId,
    },
    NonManifoldEdge {
        v0: SurfaceVertexId,
        v1: SurfaceVertexId,
        count: usize,
    },
    InvalidProvenance {
        vertex: SurfaceVertexId,
    },
}
