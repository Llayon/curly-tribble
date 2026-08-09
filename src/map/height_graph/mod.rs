// src/map/height_graph/mod.rs
//! Module entry point for Milestone M4.1 — Height Constraint Graph & Cliff-Seam Height Domains.

pub mod builder;
pub mod builder_diagnostics;
pub mod builder_dsu;
pub mod diagnostics;
pub mod runtime;
pub mod types;
pub mod validation;

pub use builder::*;
pub use builder_diagnostics::*;
pub use builder_dsu::*;
pub use diagnostics::*;
pub use runtime::*;
pub use types::*;
pub use validation::*;

use bevy::prelude::*;

pub struct HeightGraphPlugin;

impl Plugin for HeightGraphPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeightConstraintGraph>()
            .add_plugins(HeightGraphRuntimePlugin);
    }
}

#[cfg(test)]
mod tests_baseline;
#[cfg(test)]
mod tests_cliff_seams;
#[cfg(test)]
mod tests_determinism;
#[cfg(test)]
mod tests_diagnostics;
#[cfg(test)]
mod tests_matrix;
#[cfg(test)]
mod tests_regions;
#[cfg(test)]
mod tests_runtime;
