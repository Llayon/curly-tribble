// src/map/face_topology/mod.rs
pub mod corner_key;
pub mod generator;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_stress;

pub use corner_key::{canonical_corner_key, regular_corner_position, seed_for_corner};
pub use generator::generate_hex_face_topology;
pub use types::*;
pub use validation::{signed_area, validate_complete_topology, validate_face_geometry};

use bevy::prelude::*;

pub struct FaceTopologyPlugin;

impl Plugin for FaceTopologyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HexFaceTopology>();
    }
}
