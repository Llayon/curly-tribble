// src/map/surface_height/types.rs
//! Normalized domain models, configuration, and solver reports for Milestone M5 — `SurfaceHeightLayer`.

use bevy::prelude::*;

pub const NORMALIZED_HEIGHT_MIN: f32 = 0.0;
pub const NORMALIZED_HEIGHT_MAX: f32 = 1.0;

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceHeightStats {
    pub node_count: usize,
    pub min_height: f32,
    pub max_height: f32,
    pub mean_height: f32,
    pub hard_pinned_node_count: usize,
    pub resolved_cliff_constraint_count: usize,
    pub unresolved_cliff_relation_count: usize,
    pub max_cliff_violation: f32,
}

impl Default for SurfaceHeightStats {
    fn default() -> Self {
        Self {
            node_count: 0,
            min_height: 0.0,
            max_height: 0.0,
            mean_height: 0.0,
            hard_pinned_node_count: 0,
            resolved_cliff_constraint_count: 0,
            unresolved_cliff_relation_count: 0,
            max_cliff_violation: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeightSolveReport {
    pub iterations_run: u32,
    pub converged: bool,
    pub final_max_delta: f32,
}

impl Default for HeightSolveReport {
    fn default() -> Self {
        Self {
            iterations_run: 0,
            converged: true,
            final_max_delta: 0.0,
        }
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct SurfaceHeightLayer {
    pub heights: Vec<f32>,
    pub stats: SurfaceHeightStats,
    pub report: HeightSolveReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeightSolverConfigError {
    NonFiniteValue,
    GuideWeightNotPositive,
    NegativeRegionWeight,
    NegativeSmoothnessWeight,
    InvalidCliffMinDrop,
    InvalidRelaxation,
    ZeroIterations,
    InvalidConvergenceEpsilon,
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct HeightSolverConfig {
    pub guide_weight: f32,
    pub region_weight: f32,
    pub smoothness_weight: f32,
    pub mountain_bias: f32,
    pub plateau_bias: f32,
    pub lake_bias: f32,
    pub river_bias: f32,
    pub cliff_min_drop: f32,
    pub relaxation: f32,
    pub max_iterations: u32,
    pub convergence_epsilon: f32,
}

impl Default for HeightSolverConfig {
    fn default() -> Self {
        Self {
            guide_weight: 4.0,
            region_weight: 2.0,
            smoothness_weight: 0.35,
            mountain_bias: 0.12,
            plateau_bias: 0.05,
            lake_bias: -0.10,
            river_bias: -0.06,
            cliff_min_drop: 0.10,
            relaxation: 0.60,
            max_iterations: 32,
            convergence_epsilon: 1e-5,
        }
    }
}

impl HeightSolverConfig {
    /// Validates configuration bounds and finiteness.
    ///
    /// # Errors
    /// Returns `HeightSolverConfigError` if any weight or parameter is invalid.
    pub fn validate_config(&self) -> Result<(), HeightSolverConfigError> {
        let floats = [
            self.guide_weight,
            self.region_weight,
            self.smoothness_weight,
            self.mountain_bias,
            self.plateau_bias,
            self.lake_bias,
            self.river_bias,
            self.cliff_min_drop,
            self.relaxation,
            self.convergence_epsilon,
        ];
        for f in floats {
            if !f.is_finite() {
                return Err(HeightSolverConfigError::NonFiniteValue);
            }
        }

        if self.guide_weight <= 0.0 {
            return Err(HeightSolverConfigError::GuideWeightNotPositive);
        }
        if self.region_weight < 0.0 {
            return Err(HeightSolverConfigError::NegativeRegionWeight);
        }
        if self.smoothness_weight < 0.0 {
            return Err(HeightSolverConfigError::NegativeSmoothnessWeight);
        }
        if !(0.0..=1.0).contains(&self.cliff_min_drop) {
            return Err(HeightSolverConfigError::InvalidCliffMinDrop);
        }
        if self.relaxation <= 0.0 || self.relaxation > 1.0 {
            return Err(HeightSolverConfigError::InvalidRelaxation);
        }
        if self.max_iterations == 0 {
            return Err(HeightSolverConfigError::ZeroIterations);
        }
        if self.convergence_epsilon <= 0.0 {
            return Err(HeightSolverConfigError::InvalidConvergenceEpsilon);
        }

        Ok(())
    }
}

#[allow(dead_code)]
pub struct SurfaceHeightTypesPlugin;

impl Plugin for SurfaceHeightTypesPlugin {
    fn build(&self, _app: &mut App) {}
}
