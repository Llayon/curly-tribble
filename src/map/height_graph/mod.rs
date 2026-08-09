// src/map/height_graph/mod.rs
//! Module entry point for Milestone M4.1 — Height Constraint Graph & Cliff-Seam Height Domains.

pub mod builder;
pub mod diagnostics;
pub mod types;
pub mod validation;

pub use builder::*;
pub use diagnostics::*;
pub use types::*;
pub use validation::*;

use bevy::prelude::*;

pub struct HeightGraphPlugin;

impl Plugin for HeightGraphPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeightConstraintGraph>();
    }
}
