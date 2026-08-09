// src/map/surface_height/solver.rs
//! Deterministic Jacobi height solver for Milestone M5 — SurfaceHeightLayer.

use crate::map::height_graph::types::{HeightConstraintGraph, HeightNodeId};
use crate::map::surface_height::guide::LegacyHeightGuide;
use crate::map::surface_height::hard_constraints::CompiledHeightHardConstraints;
use crate::map::surface_height::targets::HeightTargetField;
use crate::map::surface_height::types::{
    HeightSolveReport, HeightSolverConfig, SurfaceHeightLayer, SurfaceHeightStats,
};
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum HeightSolveError {
    TargetCountMismatch,
    BoundCountMismatch,
    InvalidNeighborNode(HeightNodeId),
    NonFiniteTarget(HeightNodeId),
    NonFiniteResult(HeightNodeId),
    HardProjectionViolation {
        node: HeightNodeId,
        value: f32,
        bound: f32,
    },
}

/// Solves deterministic semantic scalar height field using Jacobi relaxation and topological hard cliff projections.
///
/// # Errors
/// Returns `HeightSolveError` if input vector lengths mismatch or numeric values become non-finite.
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
pub fn solve_surface_heights(
    graph: &HeightConstraintGraph,
    guide: &LegacyHeightGuide,
    targets: &HeightTargetField,
    constraints: &CompiledHeightHardConstraints,
    config: &HeightSolverConfig,
) -> Result<SurfaceHeightLayer, HeightSolveError> {
    let node_count = graph.nodes.len();
    if targets.samples.len() != node_count {
        return Err(HeightSolveError::TargetCountMismatch);
    }
    if constraints.lower_bounds.len() != node_count || constraints.upper_bounds.len() != node_count
    {
        return Err(HeightSolveError::BoundCountMismatch);
    }

    // 1. Initial solver state contract: height[i] = target[i].clamp(lower[i], upper[i])
    let mut heights = Vec::with_capacity(node_count);
    for (i, sample) in targets.samples.iter().enumerate() {
        if !sample.target.is_finite() {
            return Err(HeightSolveError::NonFiniteTarget(HeightNodeId::new(i)));
        }
        let initial_val = sample
            .target
            .clamp(constraints.lower_bounds[i], constraints.upper_bounds[i]);
        heights.push(initial_val);
    }

    // Pre-compute canonical neighbor vectors from continuity_edges (sorted & deduped per node)
    let mut neighbors = vec![Vec::new(); node_count];
    for edge in &graph.continuity_edges {
        if edge.a.index() >= node_count {
            return Err(HeightSolveError::InvalidNeighborNode(edge.a));
        }
        if edge.b.index() >= node_count {
            return Err(HeightSolveError::InvalidNeighborNode(edge.b));
        }
        neighbors[edge.a.index()].push(edge.b);
        neighbors[edge.b.index()].push(edge.a);
    }
    for nbr_list in &mut neighbors {
        nbr_list.sort_by_key(|n| n.index());
        nbr_list.dedup();
    }

    // 2. Initial hard projection pass
    project_hard_constraints(&mut heights, guide, constraints);

    // 3. Deterministic weighted Jacobi iteration loop
    let mut converged = false;
    let mut iterations_run = 0u32;
    let mut final_max_delta = 0.0f32;

    for iteration in 0..config.max_iterations {
        iterations_run = iteration + 1;
        let previous = heights.clone();
        let mut max_delta = 0.0f32;

        for (i, sample) in targets.samples.iter().enumerate() {
            let nbrs = &neighbors[i];
            let degree = nbrs.len() as f32;

            let mut sum_nbr = 0.0f32;
            for &nbr_id in nbrs {
                sum_nbr += previous[nbr_id.index()];
            }

            let weighted_target = (sample.weight * sample.target
                + config.smoothness_weight * sum_nbr)
                / (sample.weight + config.smoothness_weight * degree);

            let prev_val = previous[i];
            let mut next_val = prev_val + config.relaxation * (weighted_target - prev_val);

            // Feasible interval clamp
            next_val = next_val.clamp(constraints.lower_bounds[i], constraints.upper_bounds[i]);
            heights[i] = next_val;
        }

        // Hard cliff projection pass & pin re-assertion
        project_hard_constraints(&mut heights, guide, constraints);

        for (i, &val) in heights.iter().enumerate() {
            let delta = (val - previous[i]).abs();
            if delta > max_delta {
                max_delta = delta;
            }
        }

        final_max_delta = max_delta;
        if max_delta <= config.convergence_epsilon {
            converged = true;
            break;
        }
    }

    // 4. Validate output finiteness and hard bounds
    let mut min_h = 1.0f32;
    let mut max_h = 0.0f32;
    let mut sum_h = 0.0f64;

    for (i, &val) in heights.iter().enumerate() {
        if !val.is_finite() {
            return Err(HeightSolveError::NonFiniteResult(HeightNodeId::new(i)));
        }
        if val < constraints.lower_bounds[i] - 1e-4 {
            return Err(HeightSolveError::HardProjectionViolation {
                node: HeightNodeId::new(i),
                value: val,
                bound: constraints.lower_bounds[i],
            });
        }
        if val > constraints.upper_bounds[i] + 1e-4 {
            return Err(HeightSolveError::HardProjectionViolation {
                node: HeightNodeId::new(i),
                value: val,
                bound: constraints.upper_bounds[i],
            });
        }

        min_h = min_h.min(val);
        max_h = max_h.max(val);
        sum_h += f64::from(val);
    }

    let mean_h = if node_count > 0 {
        (sum_h / (node_count as f64)) as f32
    } else {
        0.0
    };

    let hard_pinned_count = guide
        .samples
        .iter()
        .filter(|s| s.hard_pin.is_some())
        .count();
    let unresolved_cliff_count = graph
        .cliff_relations
        .iter()
        .filter(|r| r.lower_side == crate::map::data::CliffLowerSide::Unresolved)
        .count();

    let mut max_cliff_violation = 0.0f32;
    for edge in &constraints.edges {
        let low = heights[edge.lower_node.index()];
        let high = heights[edge.higher_node.index()];
        let violation = (edge.min_drop - (high - low)).max(0.0);
        max_cliff_violation = max_cliff_violation.max(violation);
    }

    let stats = SurfaceHeightStats {
        node_count,
        min_height: if node_count > 0 { min_h } else { 0.0 },
        max_height: if node_count > 0 { max_h } else { 0.0 },
        mean_height: mean_h,
        hard_pinned_node_count: hard_pinned_count,
        resolved_cliff_constraint_count: constraints.edges.len(),
        unresolved_cliff_relation_count: unresolved_cliff_count,
        max_cliff_violation,
    };

    let report = HeightSolveReport {
        iterations_run,
        converged,
        final_max_delta,
    };

    Ok(SurfaceHeightLayer {
        heights,
        stats,
        report,
    })
}

fn project_hard_constraints(
    heights: &mut [f32],
    guide: &LegacyHeightGuide,
    constraints: &CompiledHeightHardConstraints,
) {
    // 1. Clamp to lower & upper bounds
    for (i, val) in heights.iter_mut().enumerate() {
        *val = val.clamp(constraints.lower_bounds[i], constraints.upper_bounds[i]);
    }

    // 2. Forward topological cliff pass: high >= low + min_drop
    for &node_id in &constraints.topological_order {
        let low_val = heights[node_id.index()];
        for edge in &constraints.edges {
            if edge.lower_node == node_id {
                let high_idx = edge.higher_node.index();
                heights[high_idx] = heights[high_idx].max(low_val + edge.min_drop);
            }
        }
    }

    // 3. Re-assert hard ocean pins
    for (i, sample) in guide.samples.iter().enumerate() {
        if let Some(pin) = sample.hard_pin {
            heights[i] = pin;
        }
    }
}

#[allow(dead_code)]
pub struct HeightSolverPlugin;

impl Plugin for HeightSolverPlugin {
    fn build(&self, _app: &mut App) {}
}
