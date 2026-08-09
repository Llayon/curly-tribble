// src/map/surface_height/mod.rs
//! Module entry point for Milestone M5 — SurfaceHeightLayer & Deterministic HeightSolver.

pub mod guide;
pub mod types;

pub use guide::*;
pub use types::*;

use bevy::prelude::*;

pub struct SurfaceHeightPlugin;

impl Plugin for SurfaceHeightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeightSolverConfig>()
            .init_resource::<SurfaceHeightLayer>();
    }
}
