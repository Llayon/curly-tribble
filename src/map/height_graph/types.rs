// src/map/height_graph/types.rs
//! Discrete height-domain types, IDs, and resource definitions for Milestone M4.1.

use crate::map::data::{CliffLowerSide, EdgeCoord};
use crate::map::height_constraints::types::RegionHeightIntent;
use crate::map::height_graph::diagnostics::HeightGraphDiagnostic;
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceHalfEdgeId, SurfaceVertexId};
use crate::map::HexCoord;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
pub struct HeightNodeId(usize);

impl HeightNodeId {
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
pub struct HeightNode {
    pub surface_vertex: SurfaceVertexId,
    pub incident_faces: Vec<SurfaceFaceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HeightContinuityEdge {
    pub a: HeightNodeId,
    pub b: HeightNodeId,
}

impl HeightContinuityEdge {
    #[must_use]
    pub fn new(a: HeightNodeId, b: HeightNodeId) -> Self {
        if a <= b {
            Self { a, b }
        } else {
            Self { a: b, b: a }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
pub struct HeightSheetComponentId(usize);

impl HeightSheetComponentId {
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
pub struct HeightSheetComponent {
    pub nodes: Vec<HeightNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionNodeConstraint {
    pub hex: HexCoord,
    pub intent: RegionHeightIntent,
    pub nodes: Vec<HeightNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliffNodeRelation {
    pub logical_edge: EdgeCoord,
    pub surface_vertex: SurfaceVertexId,
    pub node_a: HeightNodeId,
    pub node_b: HeightNodeId,
    pub lower_side: CliffLowerSide,
}

impl CliffNodeRelation {
    #[must_use]
    pub const fn resolved_order(&self) -> Option<(HeightNodeId, HeightNodeId)> {
        match self.lower_side {
            CliffLowerSide::A => Some((self.node_a, self.node_b)),
            CliffLowerSide::B => Some((self.node_b, self.node_a)),
            CliffLowerSide::Unresolved => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeightGraphStats {
    pub node_count: usize,
    pub split_surface_vertex_count: usize,
    pub continuity_edge_count: usize,
    pub component_count: usize,
    pub region_constraint_count: usize,
    pub cliff_relation_count: usize,
    pub unresolved_cliff_count: usize,
    pub diagnostic_count: usize,
    pub error_diagnostic_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeightGraphBuildError {
    EmptySurfaceOnConstraints,
    PartialEmptySurface {
        vertex_count: usize,
        face_count: usize,
    },
    FaceNodeCountMismatch {
        expected: usize,
        actual: usize,
    },
    InvalidSurfaceFace(SurfaceFaceId),
    InvalidSurfaceVertex(SurfaceVertexId),
    InvalidSurfaceHalfEdge(SurfaceHalfEdgeId),
    MissingTwin(SurfaceHalfEdgeId),
    NonReciprocalTwin {
        a: SurfaceHalfEdgeId,
        b: SurfaceHalfEdgeId,
    },
    TwinOrientationMismatch {
        a: SurfaceHalfEdgeId,
        b: SurfaceHalfEdgeId,
    },
    FaceMissingVertex {
        face: SurfaceFaceId,
        vertex: SurfaceVertexId,
    },
    MixedSurfaceVerticesInNode {
        node: HeightNodeId,
    },
    MissingFaceCornerMapping {
        face: SurfaceFaceId,
        corner: u8,
    },
    DuplicateFaceCornerMapping {
        face: SurfaceFaceId,
        corner: u8,
    },
    RegionNodeMismatch {
        hex: HexCoord,
    },
    CliffRelationMismatch {
        edge: EdgeCoord,
        vertex: SurfaceVertexId,
    },
    InconsistentCliffVertexRelation {
        edge: EdgeCoord,
        vertex: SurfaceVertexId,
    },
    InvalidComponent(HeightSheetComponentId),
}

#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct HeightConstraintGraph {
    pub nodes: Vec<HeightNode>,
    pub face_nodes: Vec<[HeightNodeId; 3]>,
    pub continuity_edges: Vec<HeightContinuityEdge>,
    pub node_components: Vec<HeightSheetComponentId>,
    pub components: Vec<HeightSheetComponent>,
    pub regions: Vec<RegionNodeConstraint>,
    pub cliff_relations: Vec<CliffNodeRelation>,
    pub diagnostics: Vec<HeightGraphDiagnostic>,
    pub stats: HeightGraphStats,
}

impl HeightConstraintGraph {
    #[must_use]
    pub fn region_nodes_for_hex(&self, hex: HexCoord) -> Option<&[HeightNodeId]> {
        self.regions
            .iter()
            .find(|r| r.hex == hex)
            .map(|r| r.nodes.as_slice())
    }

    pub fn cliff_relations_for_edge(
        &self,
        edge: EdgeCoord,
    ) -> impl Iterator<Item = &CliffNodeRelation> {
        self.cliff_relations
            .iter()
            .filter(move |r| r.logical_edge == edge)
    }

    #[must_use]
    pub fn height_nodes_for_surface_vertex(&self, vertex: SurfaceVertexId) -> Vec<HeightNodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.surface_vertex == vertex)
            .map(|(idx, _)| HeightNodeId::new(idx))
            .collect()
    }
}

#[allow(dead_code)]
pub struct HeightGraphTypesPlugin;

impl Plugin for HeightGraphTypesPlugin {
    fn build(&self, _app: &mut App) {}
}
