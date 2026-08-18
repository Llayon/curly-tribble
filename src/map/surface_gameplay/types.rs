// src/map/surface_gameplay/types.rs
//! Milestone M6 — normalized domain models for surface gameplay metrics and
//! the authoritative `SurfaceGameplayMap`.
//!
//! All heights in this module are NORMALIZED `[0, 1]` (M5 domain). No
//! `MAX_HEIGHT`, `Mesh`, `MapData`, `TileData`, or legacy navigation types.

use crate::map::data::EdgeCoord;
use crate::map::height_graph::types::HeightNodeId;
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceHalfEdgeId, SurfaceVertexId};
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::BTreeMap;

pub struct SurfaceGameplayTypesPlugin;

impl Plugin for SurfaceGameplayTypesPlugin {
    fn build(&self, _app: &mut App) {}
}

// ─── Per-hex metrics (pure geometry, stage 1) ────────────────────────────────

/// Normalized per-hex surface metrics derived from `SurfaceTopology` + `SurfaceTerrainBake`.
#[derive(Debug, Clone, PartialEq)]
pub struct HexSurfaceMetrics {
    /// Exact XZ of the hex center vertex (from `SurfaceVertexSource::HexCenter`).
    pub center_xz: Vec2,
    /// Normalized solved height at the center node, `[0, 1]`.
    pub center_height: f32,
    /// `max - min` over all height nodes owned by this hex, `[0, 1]`.
    pub relief: f32,
    /// Largest height delta across internal hex edges, `[0, 1]`.
    pub max_internal_step: f32,
}

// ─── Per-edge metrics (pure geometry, stage 1) ───────────────────────────────

/// Normalized metrics for one logical hex boundary (all segments aggregated).
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeSurfaceMetrics {
    /// Largest absolute height delta observed across any segment of this edge
    /// (diagnostic; does not gate traversal on its own).
    pub max_boundary_jump: f32,
    /// True when any segment of this edge is an authored cliff wall (resolved,
    /// unresolved, or taper) or a divergence seam with unequal node heights.
    pub height_seam: bool,
}

// ─── Metric field resource (stage 1 output) ──────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SurfaceMetricFieldStats {
    pub cell_count: usize,
    pub edge_count: usize,
    pub seam_edge_count: usize,
}

/// Pure-geometry metric layer. Derived once per bake generation.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct SurfaceMetricField {
    pub cells: BTreeMap<HexCoord, HexSurfaceMetrics>,
    pub edges: BTreeMap<EdgeCoord, EdgeSurfaceMetrics>,
    pub stats: SurfaceMetricFieldStats,
}

// ─── Policy types (stage 2) ───────────────────────────────────────────────────

/// Why a gameplay edge blocks traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalBlockReason {
    /// The edge is an authored cliff seam (resolved, unresolved, or taper).
    CliffSeam,
    /// The edge crosses a height step exceeding `max_walk_step`.
    HeightStep,
}

/// Authoritative gameplay cell policy.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SurfaceGameplayCell {
    /// Land cell with finite solved geometry.
    pub walkable: bool,
    /// Static movement cost: 20 base / 50 swamp / 80 stony.
    pub movement_cost: u8,
    /// Land + `TerrainType::allows_buildings()` + relief and neighbor-step relief.
    pub buildable: bool,
    /// Solved center XZ (mirrored from metrics for convenience).
    pub center_xz: Vec2,
    /// Solved center height (normalized).
    pub center_height: f32,
    /// Solved relief (normalized).
    pub relief: f32,
}

/// Authoritative gameplay edge policy.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceGameplayEdge {
    pub traversable: bool,
    pub block_reason: Option<TraversalBlockReason>,
    /// Absolute solved height delta across the primary segment (`[0, 1]`).
    pub center_height_delta: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SurfaceGameplayStats {
    pub cell_count: usize,
    pub walkable_cell_count: usize,
    pub buildable_cell_count: usize,
    pub edge_count: usize,
    pub traversable_edge_count: usize,
    pub cliff_seam_edge_count: usize,
    pub height_step_edge_count: usize,
}

/// The authoritative gameplay layer consumed by navigation, buildability, and
/// world anchoring in M6+. Never built from legacy `TileData.elevation`.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct SurfaceGameplayMap {
    pub cells: BTreeMap<HexCoord, SurfaceGameplayCell>,
    pub edges: BTreeMap<EdgeCoord, SurfaceGameplayEdge>,
    pub stats: SurfaceGameplayStats,
}

// ─── Metrics derivation errors (stage 1) ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceMetricsError {
    MissingHexCenter(HexCoord),
    DuplicateHexCenter(HexCoord),
    MissingCenterHeightNode(HexCoord),
    AmbiguousCenterHeightNode(HexCoord),
    PartialEmptyInputs {
        surface_vertices: usize,
        surface_faces: usize,
        bake_vertices: usize,
        bake_faces: usize,
    },
    NonFiniteHeight(HexCoord),
    InvalidSurfaceVertex(SurfaceVertexId),
    InvalidSurfaceFace(SurfaceFaceId),
    InvalidSurfaceHalfEdge(SurfaceHalfEdgeId),
    InvalidHeightNode(HeightNodeId),
}

// ─── Compile errors (stage 2) ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceGameplayCompileError {
    InvalidConfig(crate::map::surface_gameplay::config::SurfaceGameplayConfigError),
    MissingMetricsForTile(HexCoord),
    MetricWithoutTile(HexCoord),
}
