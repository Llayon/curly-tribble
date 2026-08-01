// src/map/face_topology/mod.rs
pub mod corner_key;
pub mod generator;
pub mod logical_adjacency;
pub mod types;
pub mod validation;
pub mod validation_complete;
pub mod validation_twins;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_mutation;
#[cfg(test)]
mod tests_stress;

pub use corner_key::{
    canonical_corner_key, corner_displacement, regular_corner_position, seed_for_corner,
};
pub use generator::generate_hex_face_topology;
pub use types::*;
pub use validation::{min_edge_length, segments_intersect, signed_area, validate_face_geometry};
pub use validation_complete::validate_complete_topology;

use bevy::prelude::*;

pub struct FaceTopologyPlugin;

impl Plugin for FaceTopologyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HexFaceTopology>();
    }
}
