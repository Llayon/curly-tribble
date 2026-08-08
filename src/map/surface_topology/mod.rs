// src/map/surface_topology/mod.rs
//! Semantic `SurfaceTopology` model, generator, validator, and runtime lifecycle.

pub mod generator;
pub mod runtime;
pub mod twins;
pub mod types;
pub mod validation;

pub use generator::*;
pub use runtime::*;
pub use twins::*;
pub use types::*;
pub use validation::*;

use bevy::prelude::*;

#[allow(dead_code)]
pub struct SurfaceTopologyPlugin;

impl Plugin for SurfaceTopologyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SurfaceTopology>()
            .add_plugins(SurfaceTopologyRuntimePlugin);
    }
}

#[cfg(test)]
mod tests_compatibility;

#[cfg(test)]
mod tests_determinism;

#[cfg(test)]
mod tests_manifold;

#[cfg(test)]
mod tests_matrix;

#[cfg(test)]
mod tests_provenance;

#[cfg(test)]
mod tests_shared_boundary;
