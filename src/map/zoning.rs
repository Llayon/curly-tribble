use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ZoneType {
    Empty,
    FoodStockpile,
    Housing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainType {
    #[default]
    Grass,
    Mud,
    Water,
    Sand,
    Stone,
    CaveFloor,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TileData {
    pub terrain: TerrainType,
    pub elevation: f32,
    pub humidity: f32,
    pub temperature: f32,
    pub roofed: bool,
}

#[derive(Component)]
pub struct Tile {
    pub terrain: TerrainType,
}

#[derive(Component)]
pub struct Roof;

#[derive(Bundle)]
pub struct RoofBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub roof: Roof,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct Zone(pub ZoneType);

pub struct ZoningPlugin;

impl Plugin for ZoningPlugin {
    fn build(&self, _app: &mut App) {
        // Here we could register types for reflection if needed
        // app.register_type::<Zone>();
    }
}
