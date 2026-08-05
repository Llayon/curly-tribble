// src/map/face_topology/mod.rs
pub mod acceptance;
pub mod acceptance_issues;
pub mod blend;
pub mod blend_diagnostics;
pub mod blend_policy;
pub mod cache;
pub mod corner_key;
pub mod debug;
pub mod fingerprint;
pub mod generator;
pub mod logical_adjacency;
pub mod metrics;
pub mod profiles;
pub mod runtime;
pub mod separation;
pub mod types;
pub mod validation;
pub mod validation_complete;
pub mod validation_twins;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_acceptance;
#[cfg(test)]
mod tests_compat;
#[cfg(test)]
mod tests_debug;
#[cfg(test)]
mod tests_mutation;
#[cfg(test)]
mod tests_profiles;
#[cfg(test)]
mod tests_quality;
#[cfg(test)]
mod tests_quality_blend;
#[cfg(test)]
mod tests_quality_blend_direction;
#[cfg(test)]
mod tests_quality_shared;
#[cfg(test)]
mod tests_quality_stress;
#[cfg(test)]
mod tests_stress;

#[cfg(test)]
mod tests_blend_boundary;
#[cfg(test)]
mod tests_blend_candidate_adjacency;
#[cfg(test)]
mod tests_blend_candidate_geometry;
#[cfg(test)]
mod tests_blend_candidate_shared;
#[cfg(test)]
mod tests_blend_separation;
#[cfg(test)]
mod tests_blend_stress;

pub use corner_key::{
    canonical_corner_key, corner_displacement, regular_corner_position, seed_for_corner,
};
pub use generator::generate_hex_face_topology;
pub use generator::generate_hex_face_topology_with_profile;
pub use profiles::HexDeformationProfile;
pub use types::*;
pub use validation::{min_edge_length, segments_intersect, signed_area, validate_face_geometry};
pub use validation_complete::validate_complete_topology;

use bevy::prelude::*;

pub struct FaceTopologyPlugin;

impl Plugin for FaceTopologyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HexFaceTopology>().add_plugins((
            runtime::FaceTopologyRuntimePlugin,
            debug::FaceTopologyDebugPlugin,
        ));
    }
}
