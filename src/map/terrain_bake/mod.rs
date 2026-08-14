// src/map/terrain_bake/mod.rs
//! Module entry point for Milestone M5.1 — `SurfaceTerrainBake` production terrain geometry.
//!
//! `SurfaceTerrainBake` is the authoritative source of ground render geometry in M5.1+.
//! Legacy `TerrainTopology` is derived from bake for compatibility only and is not the source of Y.

pub mod builder;
pub mod compat;
pub mod runtime;
pub mod types;
pub mod validation;
pub mod walls;

pub use builder::*;
pub use compat::*;
pub use runtime::*;
pub use types::*;
pub use validation::*;

#[cfg(test)]
pub mod tests_builder;
#[cfg(test)]
pub mod tests_compat;
#[cfg(test)]
pub mod tests_runtime;
#[cfg(test)]
pub mod tests_walls;

use bevy::prelude::*;

pub struct TerrainBakePlugin;

impl Plugin for TerrainBakePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SurfaceTerrainBake>()
            .init_resource::<TerrainBakeGenerationState>()
            .add_plugins(runtime::TerrainBakeRuntimePlugin);
    }
}
