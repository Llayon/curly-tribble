// src/map/height_graph/diagnostics.rs
//! Diagnostics model and severity definitions for Milestone M4.1.

use crate::map::data::EdgeCoord;
use crate::map::height_graph::types::HeightNodeId;
use crate::map::surface_topology::types::SurfaceVertexId;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HeightDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum HeightGraphDiagnosticKind {
    UnresolvedCliff {
        edge: EdgeCoord,
    },
    CollapsedCliffSample {
        edge: EdgeCoord,
        vertex: SurfaceVertexId,
    },
    UnsplittableCliff {
        edge: EdgeCoord,
    },
    OpposedCliffOrdering {
        a: HeightNodeId,
        b: HeightNodeId,
    },
    DirectedCliffCycle {
        component_nodes: Vec<HeightNodeId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeightGraphDiagnostic {
    pub severity: HeightDiagnosticSeverity,
    pub kind: HeightGraphDiagnosticKind,
}

#[allow(dead_code)]
pub struct HeightGraphDiagnosticsPlugin;

impl Plugin for HeightGraphDiagnosticsPlugin {
    fn build(&self, _app: &mut App) {}
}
