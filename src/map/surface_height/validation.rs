// src/map/surface_height/validation.rs
//! Pure structural and numeric validator for Milestone M5 — SurfaceHeightLayer.

use crate::map::height_graph::types::{HeightConstraintGraph, HeightNodeId};
use crate::map::surface_height::guide::LegacyHeightGuide;
use crate::map::surface_height::hard_constraints::{
    CliffHardConstraint, CompiledHeightHardConstraints,
};
use crate::map::surface_height::types::{
    HeightSolverConfig, SurfaceHeightLayer, SurfaceHeightStats, NORMALIZED_HEIGHT_MAX,
    NORMALIZED_HEIGHT_MIN,
};
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceHeightValidationError {
    NodeCountMismatch {
        expected: usize,
        actual: usize,
    },
    NonFiniteHeight {
        node: HeightNodeId,
    },
    HeightOutOfRange {
        node: HeightNodeId,
        height: f32,
    },
    HardPinViolation {
        node: HeightNodeId,
        expected: f32,
        actual: f32,
    },
    CliffDropViolation {
        edge: CliffHardConstraint,
        drop: f32,
    },
    MaxCliffViolationExceeded {
        max_violation: f32,
    },
    StatsMismatch,
}

/// Independently validates derived `SurfaceHeightLayer` bounds, hard pins, cliff drops, and stats.
///
/// # Errors
/// Returns `SurfaceHeightValidationError` if layer fails numerical invariants or stats mismatch.
#[allow(clippy::too_many_lines)]
pub fn validate_surface_height_layer(
    layer: &SurfaceHeightLayer,
    graph: &HeightConstraintGraph,
    guide: &LegacyHeightGuide,
    constraints: &CompiledHeightHardConstraints,
    _config: &HeightSolverConfig,
) -> Result<(), SurfaceHeightValidationError> {
    let node_count = graph.nodes.len();
    if layer.heights.len() != node_count {
        return Err(SurfaceHeightValidationError::NodeCountMismatch {
            expected: node_count,
            actual: layer.heights.len(),
        });
    }

    let eps = 1e-4f32;
    let mut min_h = 1.0f32;
    let mut max_h = 0.0f32;
    let mut sum_h = 0.0f64;

    for (i, &val) in layer.heights.iter().enumerate() {
        let node_id = HeightNodeId::new(i);
        if !val.is_finite() {
            return Err(SurfaceHeightValidationError::NonFiniteHeight { node: node_id });
        }
        if val < NORMALIZED_HEIGHT_MIN - eps || val > NORMALIZED_HEIGHT_MAX + eps {
            return Err(SurfaceHeightValidationError::HeightOutOfRange {
                node: node_id,
                height: val,
            });
        }

        min_h = min_h.min(val);
        max_h = max_h.max(val);
        sum_h += f64::from(val);
    }

    // 2. Validate hard pins
    for (i, sample) in guide.samples.iter().enumerate() {
        if let Some(pin) = sample.hard_pin {
            let val = layer.heights[i];
            if (val - pin).abs() > eps {
                return Err(SurfaceHeightValidationError::HardPinViolation {
                    node: HeightNodeId::new(i),
                    expected: pin,
                    actual: val,
                });
            }
        }
    }

    // 3. Validate resolved cliff minimum drops and maximum violation
    let mut max_cliff_violation = 0.0f32;
    for edge in &constraints.edges {
        let low = layer.heights[edge.lower_node.index()];
        let high = layer.heights[edge.higher_node.index()];
        let drop = high - low;
        let violation = (edge.min_drop - drop).max(0.0);
        max_cliff_violation = max_cliff_violation.max(violation);

        if drop < edge.min_drop - eps {
            return Err(SurfaceHeightValidationError::CliffDropViolation { edge: *edge, drop });
        }
    }

    if max_cliff_violation > eps {
        return Err(SurfaceHeightValidationError::MaxCliffViolationExceeded {
            max_violation: max_cliff_violation,
        });
    }

    // 4. Re-calculate independent SurfaceHeightStats and verify exact match
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

    let expected_stats = SurfaceHeightStats {
        node_count,
        min_height: if node_count > 0 { min_h } else { 0.0 },
        max_height: if node_count > 0 { max_h } else { 0.0 },
        mean_height: mean_h,
        hard_pinned_node_count: hard_pinned_count,
        resolved_cliff_constraint_count: constraints.edges.len(),
        unresolved_cliff_relation_count: unresolved_cliff_count,
        max_cliff_violation,
    };

    if layer.stats != expected_stats {
        return Err(SurfaceHeightValidationError::StatsMismatch);
    }

    Ok(())
}

#[allow(dead_code)]
pub struct HeightValidationPlugin;

impl Plugin for HeightValidationPlugin {
    fn build(&self, _app: &mut App) {}
}
