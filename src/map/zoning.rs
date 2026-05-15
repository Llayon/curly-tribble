use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::HashMap;

pub const MAX_HEIGHT: f32 = 12.0;
pub const HEX_SIZE: f32 = 1.0;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ZoneType {
    Empty,
    FoodStockpile,
    Housing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum TerrainType {
    #[default]
    Grass,
    Mud,
    Water,
    Sand,
    Stone,
    CaveFloor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileLayer {
    #[default]
    Ground,
    Roof,
}

#[derive(Clone, Copy, Debug, Default, Reflect)]
pub struct TileData {
    pub terrain: TerrainType,
    pub elevation: f32,
    pub humidity: f32,
    pub temperature: f32,
    pub roofed: bool,
    pub is_ocean: bool,
}

#[derive(Resource, Default, Clone)]
pub struct MapData {
    pub width: u32,
    pub height: u32,
    pub tiles: HashMap<HexCoord, TileData>,
}

impl MapData {
    #[must_use]
    pub fn get_tile(&self, q: i32, r: i32) -> Option<&TileData> {
        self.tiles.get(&HexCoord::new(q, r))
    }

    pub fn get_tile_mut(&mut self, q: i32, r: i32) -> Option<&mut TileData> {
        self.tiles.get_mut(&HexCoord::new(q, r))
    }

    #[must_use]
    pub fn get_hex_height(&self, q: i32, r: i32) -> f32 {
        self.get_tile(q, r).map_or(0.0, |t| t.elevation * MAX_HEIGHT)
    }

    #[must_use]
    pub fn is_too_steep(&self, q: i32, r: i32) -> bool {
        let current_elev = self.get_tile(q, r).map_or(0.0, |t| t.elevation);
        let coord = HexCoord::new(q, r);
        for neighbor_coord in coord.neighbors() {
            if let Some(neighbor) = self.tiles.get(&neighbor_coord) {
                if (neighbor.elevation - current_elev).abs() > 0.3 {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Resource)]
pub struct WorldSeed(u32);

impl WorldSeed {
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }
    #[must_use]
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for WorldSeed {
    fn default() -> Self {
        Self(42)
    }
}

#[derive(Component)]
pub struct Tile {
    pub terrain: TerrainType,
}

#[derive(Component)]
pub struct Roof;

use crate::map::MapEntity;

#[derive(Bundle)]
pub struct RoofBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub roof: Roof,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct SmoothTileBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub tile: Tile,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct GlobalTerrainBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub name: Name,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct WaterBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub name: Name,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct MountainRoofBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub roof: Roof,
    pub name: Name,
    pub marker: MapEntity,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct Zone(pub ZoneType);

pub struct ZoningPlugin;

impl Plugin for ZoningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldSeed>().init_resource::<MapData>();
    }
}
