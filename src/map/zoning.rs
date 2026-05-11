use bevy::prelude::*;

pub const MAX_HEIGHT: f32 = 4.0;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileLayer {
    #[default]
    Ground,
    Roof,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TileData {
    pub terrain: TerrainType,
    pub elevation: f32,
    pub humidity: f32,
    pub temperature: f32,
    pub roofed: bool,
}

#[derive(Resource, Default, Clone)]
pub struct MapData {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<TileData>,
}

impl MapData {
    pub fn get_tile(&self, x: i32, z: i32) -> Option<&TileData> {
        let ux = (x + (self.width as i32 / 2)) as u32;
        let uz = (z + (self.height as i32 / 2)) as u32;
        if ux < self.width && uz < self.height {
            Some(&self.tiles[(uz * self.width + ux) as usize])
        } else {
            None
        }
    }

    pub fn get_tile_mut(&mut self, x: i32, z: i32) -> Option<&mut TileData> {
        let ux = (x + (self.width as i32 / 2)) as u32;
        let uz = (z + (self.height as i32 / 2)) as u32;
        if ux < self.width && uz < self.height {
            Some(&mut self.tiles[(uz * self.width + ux) as usize])
        } else {
            None
        }
    }

    pub fn get_corner_height(&self, x: i32, z: i32) -> f32 {
        let mut total = 0.0;
        let mut count = 0;
        for dx in -1..=0 {
            for dz in -1..=0 {
                if let Some(tile) = self.get_tile(x + dx, z + dz) {
                    total += tile.elevation;
                    count += 1;
                }
            }
        }
        if count > 0 {
            (total / count as f32) * MAX_HEIGHT
        } else if let Some(tile) = self.get_tile(x, z) {
            tile.elevation * MAX_HEIGHT
        } else {
            0.0
        }
    }

    pub fn is_too_steep(&self, x: i32, z: i32) -> bool {
        let current_elev = self.get_tile(x, z).map(|t| t.elevation).unwrap_or(0.0);
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if let Some(neighbor) = self.get_tile(x + dx, z + dz) {
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
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }
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

#[derive(Bundle)]
pub struct RoofBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub roof: Roof,
}

#[derive(Bundle)]
pub struct SmoothTileBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub tile: Tile,
}

#[derive(Bundle)]
pub struct GlobalTerrainBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub name: Name,
}

#[derive(Bundle)]
pub struct WaterBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub name: Name,
}

#[derive(Bundle)]
pub struct MountainRoofBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub roof: Roof,
    pub name: Name,
}

#[derive(Bundle)]
pub struct LogicTileBundle {
    pub transform: Transform,
    pub tile: Tile,
    pub name: Name,
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
