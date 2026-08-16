// src/map/surface_gameplay/mod.rs
//! Milestone M6 — `SurfaceGameplayMap`: deterministic navigation and buildability
//! authority derived from the solved surface (M5 `SurfaceHeightLayer` + M5.1
//! `SurfaceTerrainBake`), not from legacy `MapData` elevation.

pub mod config;
pub mod types;

pub use config::*;
pub use types::*;

use bevy::prelude::*;

pub struct SurfaceGameplayPlugin;

impl Plugin for SurfaceGameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<config::SurfaceGameplayConfig>()
            .init_resource::<types::SurfaceMetricField>()
            .init_resource::<types::SurfaceGameplayMap>()
            .register_type::<config::SurfaceGameplayConfig>()
            .add_plugins((
                config::SurfaceGameplayConfigPlugin,
                types::SurfaceGameplayTypesPlugin,
            ));
    }
}
