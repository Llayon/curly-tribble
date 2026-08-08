// src/map/height_constraints/types.rs
//! Semantic landscape height constraint data model and compile error types.

use crate::map::data::{CliffLowerSide, EdgeCoord};
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceHalfEdgeId};
use crate::map::HexCoord;
use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightConstraintTypesPlugin;

impl Plugin for HeightConstraintTypesPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Logical intent for a region height constraint derived from `LandscapeFeature`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RegionHeightIntent {
    Mountain,
    Plateau,
    Lake,
    River,
}

/// Semantic region height constraint binding a logical tile intent to its surface faces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionHeightConstraint {
    pub hex: HexCoord,
    pub intent: RegionHeightIntent,
    pub faces: Vec<SurfaceFaceId>,
}

/// Boundary segment formed by a pair of reciprocal surface half-edges spanning two logical hexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SurfaceBoundarySegment {
    pub half_edge_a: SurfaceHalfEdgeId,
    pub half_edge_b: SurfaceHalfEdgeId,
}

/// Semantic cliff height constraint binding a logical cliff edge to its surface boundary segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliffHeightConstraint {
    pub logical_edge: EdgeCoord,
    pub lower_side: CliffLowerSide,
    pub segments: Vec<SurfaceBoundarySegment>,
}

/// Summary statistics for compiled height constraints.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeightConstraintStats {
    pub region_count: usize,
    pub cliff_count: usize,
    pub referenced_surface_faces: usize,
    pub referenced_boundary_segments: usize,
}

/// Resource holding derived landscape height constraints on `SurfaceTopology`.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct HeightConstraintSet {
    pub regions: Vec<RegionHeightConstraint>,
    pub cliffs: Vec<CliffHeightConstraint>,
    pub stats: HeightConstraintStats,
}

/// Errors occurring during landscape height constraint compilation or validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeightConstraintCompileError {
    MissingSurfaceRegion(HexCoord),
    InvalidSurfaceFace(SurfaceFaceId),
    RegionOwnerMismatch {
        hex: HexCoord,
        face: SurfaceFaceId,
        actual: HexCoord,
    },
    InvalidSurfaceHalfEdge(SurfaceHalfEdgeId),
    MissingTwin(SurfaceHalfEdgeId),
    NonReciprocalTwin {
        a: SurfaceHalfEdgeId,
        b: SurfaceHalfEdgeId,
    },
    MissingSurfaceBoundary(EdgeCoord),
    BoundaryOwnerMismatch {
        edge: EdgeCoord,
        half_edge: SurfaceHalfEdgeId,
        expected: HexCoord,
        actual: HexCoord,
    },
    IncompleteRegionFaces {
        hex: HexCoord,
    },
    IncompleteCliffSegments {
        edge: EdgeCoord,
    },
    UnauthoredRegionConstraint(HexCoord),
    UnauthoredCliffConstraint(EdgeCoord),
}
