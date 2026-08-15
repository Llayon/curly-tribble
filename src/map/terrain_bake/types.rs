// src/map/terrain_bake/types.rs
//! Production render geometry types for Milestone M5.1 — `SurfaceTerrainBake`.
//! One ground render vertex per `HeightNodeId` (invariant).

use crate::map::height_graph::types::HeightNodeId;
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceHalfEdgeId, SurfaceVertexId};
use crate::map::HexCoord;
use bevy::prelude::*;

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct TerrainBakeTypesPlugin;

impl Plugin for TerrainBakeTypesPlugin {
    fn build(&self, _app: &mut App) {}
}

// ─── Vertex ──────────────────────────────────────────────────────────────────

/// One ground render vertex, 1-to-1 with a `HeightNodeId`.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainBakeVertex {
    pub surface_vertex: SurfaceVertexId,
    pub height_node: HeightNodeId,
    pub position_xz: Vec2,
    pub normalized_height: f32,
    pub owner_hexes: Vec<HexCoord>,
}

// ─── Face ────────────────────────────────────────────────────────────────────

/// One ground triangle — indices into `SurfaceTerrainBake::vertices` via `HeightNodeId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainBakeFace {
    pub surface_face: SurfaceFaceId,
    pub nodes: [HeightNodeId; 3],
    pub owner_hex: HexCoord,
}

// ─── Cliff wall ───────────────────────────────────────────────────────────────

/// One endpoint of a cliff wall segment.
/// `primary_node` is from the primary face; `twin_node` from the adjacent face.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliffWallEndpoint {
    pub surface_vertex: SurfaceVertexId,
    pub primary_node: HeightNodeId,
    pub twin_node: HeightNodeId,
}

/// One cliff wall segment — canonical pair where `primary.index()` < `twin.index()`.
///
/// Represents a topological seam where two adjacent faces share a surface edge
/// but their height-domain nodes diverge (cliff split).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliffWallSegment {
    pub primary_half_edge: SurfaceHalfEdgeId,
    pub twin_half_edge: SurfaceHalfEdgeId,
    pub primary_face: SurfaceFaceId,
    pub twin_face: SurfaceFaceId,
    /// endpoints[0] = origin side, endpoints[1] = destination side.
    pub endpoints: [CliffWallEndpoint; 2],
}

// ─── Stats ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerrainBakeStats {
    pub ground_vertex_count: usize,
    pub ground_face_count: usize,
    pub cliff_wall_segment_count: usize,
    pub split_surface_vertex_count: usize,
}

// ─── Bake resource ────────────────────────────────────────────────────────────

/// Authoritative production render geometry — source of truth for M5.1+.
///
/// `vertices[i]` corresponds to `HeightNodeId::new(i)`.
/// `faces[i]` corresponds to `SurfaceFaceId::new(i)`.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct SurfaceTerrainBake {
    pub vertices: Vec<TerrainBakeVertex>,
    pub faces: Vec<TerrainBakeFace>,
    pub cliff_walls: Vec<CliffWallSegment>,
    pub stats: TerrainBakeStats,
}

// ─── Build errors ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TerrainBakeBuildError {
    PartialEmptyInputs {
        surface_vertices: usize,
        surface_faces: usize,
        graph_nodes: usize,
        height_count: usize,
    },
    FaceNodeCountMismatch {
        expected: usize,
        actual: usize,
    },
    HeightNodeCountMismatch {
        expected: usize,
        actual: usize,
    },
    InvalidSurfaceVertex(SurfaceVertexId),
    InvalidSurfaceFace(SurfaceFaceId),
    InvalidSurfaceHalfEdge(SurfaceHalfEdgeId),
    InvalidHeightNode(HeightNodeId),
    InvalidIncidentFace {
        node: HeightNodeId,
        face: SurfaceFaceId,
    },
    IncidentFaceMissingNode {
        node: HeightNodeId,
        face: SurfaceFaceId,
    },
    NonFiniteHeight(HeightNodeId),
    HeightOutOfRange {
        node: HeightNodeId,
        height: f32,
    },
}

// ─── Validation errors ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TerrainBakeValidationError {
    VertexCountMismatch {
        expected: usize,
        actual: usize,
    },
    FaceCountMismatch {
        expected: usize,
        actual: usize,
    },
    HeightCountMismatch {
        expected: usize,
        actual: usize,
    },
    FaceNodeCountMismatch {
        expected: usize,
        actual: usize,
    },
    NodeIndexMismatch {
        vertex_idx: usize,
        expected: HeightNodeId,
    },
    VertexSurfaceVertexMismatch {
        node: HeightNodeId,
    },
    PositionMismatch {
        node: HeightNodeId,
    },
    HeightOutOfRange {
        node: HeightNodeId,
        height: f32,
    },
    HeightMismatch {
        node: HeightNodeId,
    },
    OwnerHexesMismatch {
        node: HeightNodeId,
    },
    InvalidIncidentFace {
        node: HeightNodeId,
        face: SurfaceFaceId,
    },
    FaceNodesMismatch {
        face: SurfaceFaceId,
    },
    FaceOwnerMismatch {
        face: SurfaceFaceId,
    },
    WallDetectionFailed,
    WallSegmentMismatch {
        expected: usize,
        actual: usize,
    },
    WallNodeOutOfRange {
        node: HeightNodeId,
    },
    StatsMismatch,
}

// ─── Compat errors ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainBakeCompatError {
    VertexIndexOverflow(HeightNodeId),
    FaceNodeIndexOverflow {
        face: SurfaceFaceId,
        node: HeightNodeId,
    },
}
