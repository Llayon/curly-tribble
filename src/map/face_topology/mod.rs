// src/map/face_topology/mod.rs
pub mod corner_key;
pub mod generator;
pub mod tests;
pub mod types;
pub mod validation;

pub use corner_key::*;
pub use generator::*;
pub use types::*;
pub use validation::*;

use bevy::prelude::*;

pub struct FaceTopologyPlugin;

impl Plugin for FaceTopologyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            types::FaceTopologyTypesPlugin,
            corner_key::CornerKeyPlugin,
            validation::ValidationPlugin,
            generator::GeneratorPlugin,
            tests::TestsPlugin,
        ))
        .init_resource::<HexFaceTopology>();
    }
}
