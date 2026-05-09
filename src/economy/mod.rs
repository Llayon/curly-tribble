use bevy::prelude::*;

pub mod assets;
pub mod global;
pub mod mesh_gen;

pub use assets::GameAssets;
pub use global::GlobalResources;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((assets::AssetsPlugin, global::GlobalEconomyPlugin));
    }
}
