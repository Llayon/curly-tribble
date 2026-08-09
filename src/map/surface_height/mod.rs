// src/map/surface_height/mod.rs
//! Module entry point for Milestone M5 — `SurfaceHeightLayer` & Deterministic `HeightSolver`.

pub mod guide;
pub mod hard_constraints;
pub mod runtime;
pub mod solver;
pub mod targets;
pub mod types;
pub mod validation;

#[cfg(test)]
pub mod tests_cliffs;
#[cfg(test)]
pub mod tests_determinism;
#[cfg(test)]
pub mod tests_guide;
#[cfg(test)]
pub mod tests_matrix;
#[cfg(test)]
pub mod tests_matrix_synthetic;
#[cfg(test)]
pub mod tests_runtime;
#[cfg(test)]
pub mod tests_runtime_transactional;
#[cfg(test)]
pub mod tests_solver;
#[cfg(test)]
pub mod tests_targets;

pub use guide::*;
pub use hard_constraints::*;
pub use runtime::*;
pub use solver::*;
pub use targets::*;
pub use types::*;
pub use validation::*;

use bevy::prelude::*;

pub struct SurfaceHeightPlugin;

impl Plugin for SurfaceHeightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeightSolverConfig>()
            .init_resource::<LegacyHeightGuide>()
            .init_resource::<HeightTargetField>()
            .init_resource::<SurfaceHeightLayer>()
            .add_plugins(SurfaceHeightRuntimePlugin);
    }
}
