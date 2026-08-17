// src/map/surface_gameplay/mod.rs
//! Milestone M6 — `SurfaceGameplayMap`: deterministic navigation and buildability
//! authority derived from the solved surface (M5 `SurfaceHeightLayer` + M5.1
//! `SurfaceTerrainBake`), not from legacy `MapData` elevation.

pub mod compiler;
pub mod config;
pub mod edges;
pub mod metrics;
pub mod types;

pub use config::*;
pub use metrics::*;
pub use types::*;

#[cfg(test)]
pub mod tests_compiler;
#[cfg(test)]
pub mod tests_metrics;
#[cfg(test)]
pub mod tests_shared;

use bevy::prelude::*;

pub struct SurfaceGameplayPlugin;

impl Plugin for SurfaceGameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<config::SurfaceGameplayConfig>()
            .init_resource::<types::SurfaceMetricField>()
            .init_resource::<types::SurfaceGameplayMap>()
            .register_type::<config::SurfaceGameplayConfig>()
            .add_plugins((
                compiler::SurfaceGameplayCompilerPlugin,
                config::SurfaceGameplayConfigPlugin,
                edges::SurfaceGameplayEdgesPlugin,
                metrics::SurfaceGameplayMetricsPlugin,
                types::SurfaceGameplayTypesPlugin,
            ));
    }
}
