// src/map/height_constraints/mod.rs
//! Semantic landscape height constraint compilation, validation, and runtime lifecycle.

pub mod compiler;
pub mod runtime;
pub mod types;
pub mod validation;

pub use compiler::*;
pub use runtime::*;
pub use types::*;
pub use validation::*;

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightConstraintsPlugin;

impl Plugin for HeightConstraintsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeightConstraintSet>()
            .add_plugins(HeightConstraintRuntimePlugin);
    }
}

#[cfg(test)]
mod tests_cliffs;

#[cfg(test)]
mod tests_determinism;

#[cfg(test)]
mod tests_matrix;

#[cfg(test)]
mod tests_regions;

#[cfg(test)]
mod tests_runtime;

#[cfg(test)]
mod tests_runtime_lifecycle;
