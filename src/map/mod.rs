use crate::sets::StartupSet;
use bevy::prelude::*;
use resources::ResourcesPlugin;

pub mod atmosphere;
pub mod resources;
pub mod zoning;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((zoning::ZoningPlugin, ResourcesPlugin))
            .add_systems(Startup, spawn_map.in_set(StartupSet::SpawnEntities));
    }
}

#[derive(Bundle)]
pub struct MapTileBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub tile: zoning::Tile,
}

fn spawn_map(mut commands: Commands, assets: Res<crate::economy::GameAssets>) {
    for x in -5..5 {
        for z in -5..5 {
            commands.spawn(MapTileBundle {
                mesh: Mesh3d(assets.ground_mesh.clone()),
                material: MeshMaterial3d(assets.ground_material.clone()),
                transform: Transform::from_xyz(x as f32, 0.0, z as f32),
                tile: zoning::Tile,
            });
        }
    }
}
