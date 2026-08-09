// src/map/surface_height/hard_constraints.rs
//! Feasible DAG compilation and interval bound calculations for Milestone M5 — `SurfaceHeightLayer`.

use crate::map::height_graph::diagnostics::HeightDiagnosticSeverity;
use crate::map::height_graph::types::{HeightConstraintGraph, HeightNodeId};
use crate::map::surface_height::guide::LegacyHeightGuide;
use crate::map::surface_height::types::{
    HeightSolverConfig, NORMALIZED_HEIGHT_MAX, NORMALIZED_HEIGHT_MIN,
};
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CliffHardConstraint {
    pub lower_node: HeightNodeId,
    pub higher_node: HeightNodeId,
    pub min_drop: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledHeightHardConstraints {
    pub edges: Vec<CliffHardConstraint>,
    pub topological_order: Vec<HeightNodeId>,
    pub lower_bounds: Vec<f32>,
    pub upper_bounds: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeightHardConstraintError {
    NodeCountMismatch,
    InvalidHeightNode(HeightNodeId),
    BlockingGraphDiagnostics {
        count: usize,
    },
    DirectedConstraintCycle,
    InfeasibleHardConstraints {
        node: HeightNodeId,
        lower: f32,
        upper: f32,
    },
    NonFiniteBound {
        node: HeightNodeId,
    },
}

/// Compiles resolved cliff hard constraints into a canonical DAG and computes feasible interval bounds.
///
/// # Errors
/// Returns `HeightHardConstraintError` if graph has error diagnostics, cycles, or infeasible ocean/cliff bounds.
#[allow(clippy::too_many_lines)]
pub fn compile_hard_constraints(
    graph: &HeightConstraintGraph,
    guide: &LegacyHeightGuide,
    config: &HeightSolverConfig,
) -> Result<CompiledHeightHardConstraints, HeightHardConstraintError> {
    if guide.samples.len() != graph.nodes.len() {
        return Err(HeightHardConstraintError::NodeCountMismatch);
    }

    // 1. Check blocking M4.1 error diagnostics
    let error_diag_count = graph
        .diagnostics
        .iter()
        .filter(|d| d.severity == HeightDiagnosticSeverity::Error)
        .count();
    if error_diag_count > 0 {
        return Err(HeightHardConstraintError::BlockingGraphDiagnostics {
            count: error_diag_count,
        });
    }

    // 2. Extract resolved cliff relations and canonical dedup: (lower, higher) -> max(min_drop)
    let mut edge_map: BTreeMap<(HeightNodeId, HeightNodeId), f32> = BTreeMap::new();
    for rel in &graph.cliff_relations {
        if let Some((lower, higher)) = rel.resolved_order() {
            if lower == higher {
                continue; // Collapsed cliff sample taper point
            }
            if lower.index() >= graph.nodes.len() {
                return Err(HeightHardConstraintError::InvalidHeightNode(lower));
            }
            if higher.index() >= graph.nodes.len() {
                return Err(HeightHardConstraintError::InvalidHeightNode(higher));
            }

            let entry = edge_map.entry((lower, higher)).or_insert(0.0);
            *entry = entry.max(config.cliff_min_drop);
        }
    }

    let edges: Vec<CliffHardConstraint> = edge_map
        .into_iter()
        .map(
            |((lower_node, higher_node), min_drop)| CliffHardConstraint {
                lower_node,
                higher_node,
                min_drop,
            },
        )
        .collect();

    // 3. Build adjacency & in-degree maps for Kahn's topological sort
    let mut in_degree = vec![0usize; graph.nodes.len()];
    let mut adj = vec![Vec::new(); graph.nodes.len()];
    for edge in &edges {
        adj[edge.lower_node.index()].push(edge.higher_node);
        in_degree[edge.higher_node.index()] += 1;
    }

    // Zero-indegree queue using BTreeSet for canonical deterministic lowest HeightNodeId selection
    let mut zero_indegree: BTreeSet<HeightNodeId> = BTreeSet::new();
    for (node_idx, &deg) in in_degree.iter().enumerate() {
        if deg == 0 {
            zero_indegree.insert(HeightNodeId::new(node_idx));
        }
    }

    let mut topological_order = Vec::with_capacity(graph.nodes.len());
    let mut visited = HashSet::with_capacity(graph.nodes.len());

    while let Some(node_id) = zero_indegree.pop_first() {
        topological_order.push(node_id);
        visited.insert(node_id);

        for &nbr_id in &adj[node_id.index()] {
            let deg = &mut in_degree[nbr_id.index()];
            *deg -= 1;
            if *deg == 0 {
                zero_indegree.insert(nbr_id);
            }
        }
    }

    if topological_order.len() != graph.nodes.len() {
        return Err(HeightHardConstraintError::DirectedConstraintCycle);
    }

    // 4. Compute initial feasible interval bounds [lower_bounds, upper_bounds]
    let mut lower_bounds = vec![NORMALIZED_HEIGHT_MIN; graph.nodes.len()];
    let mut upper_bounds = vec![NORMALIZED_HEIGHT_MAX; graph.nodes.len()];

    for (node_idx, sample) in guide.samples.iter().enumerate() {
        if let Some(pin) = sample.hard_pin {
            lower_bounds[node_idx] = pin;
            upper_bounds[node_idx] = pin;
        }
    }

    // Forward topological pass: lower[high] = max(lower[high], lower[low] + min_drop)
    for &node_id in &topological_order {
        let low_val = lower_bounds[node_id.index()];
        for edge in &edges {
            if edge.lower_node == node_id {
                let high_idx = edge.higher_node.index();
                lower_bounds[high_idx] = lower_bounds[high_idx].max(low_val + edge.min_drop);
            }
        }
    }

    // Reverse topological pass: upper[low] = min(upper[low], upper[high] - min_drop)
    for &node_id in topological_order.iter().rev() {
        let high_val = upper_bounds[node_id.index()];
        for edge in &edges {
            if edge.higher_node == node_id {
                let low_idx = edge.lower_node.index();
                upper_bounds[low_idx] = upper_bounds[low_idx].min(high_val - edge.min_drop);
            }
        }
    }

    // 5. Feasibility and finiteness checks
    for node_idx in 0..graph.nodes.len() {
        let low = lower_bounds[node_idx];
        let high = upper_bounds[node_idx];
        if !low.is_finite() || !high.is_finite() {
            return Err(HeightHardConstraintError::NonFiniteBound {
                node: HeightNodeId::new(node_idx),
            });
        }
        if low > high + 1e-6 {
            return Err(HeightHardConstraintError::InfeasibleHardConstraints {
                node: HeightNodeId::new(node_idx),
                lower: low,
                upper: high,
            });
        }
    }

    Ok(CompiledHeightHardConstraints {
        edges,
        topological_order,
        lower_bounds,
        upper_bounds,
    })
}

#[allow(dead_code)]
pub struct HeightHardConstraintsPlugin;

impl Plugin for HeightHardConstraintsPlugin {
    fn build(&self, _app: &mut App) {}
}
