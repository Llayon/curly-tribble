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
    Dirt,
    Dusty,
    Fertile,
    Mossy,
    Steppe,
    Stony,
    Swamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum LandscapeFeature {
    #[default]
    None,
    Mountain,
    Lake,
    River,
    Plateau,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum ForestType {
    #[default]
    None,
    Deciduous,
    Coniferous,
}

pub struct SedimentTraits {
    pub allow_buildings: bool,
    pub allow_mining: bool,
    pub allow_forests: bool,
    pub allow_plants: bool,
    pub allow_ores: bool,
}

impl TerrainType {
    pub fn traits(&self) -> SedimentTraits {
        match self {
            TerrainType::Grass => SedimentTraits {
                allow_buildings: true,
                allow_mining: true,
                allow_forests: true,
                allow_plants: true,
                allow_ores: true,
            },
            TerrainType::Dirt => SedimentTraits {
                allow_buildings: true,
                allow_mining: true,
                allow_forests: false,
                allow_plants: true,
                allow_ores: true,
            },
            TerrainType::Dusty => SedimentTraits {
                allow_buildings: true,
                allow_mining: true,
                allow_forests: false,
                allow_plants: true,
                allow_ores: true,
            },
            TerrainType::Fertile => SedimentTraits {
                allow_buildings: true,
                allow_mining: true,
                allow_forests: true,
                allow_plants: true,
                allow_ores: true,
            },
            TerrainType::Mossy => SedimentTraits {
                allow_buildings: true,
                allow_mining: true,
                allow_forests: true,
                allow_plants: true,
                allow_ores: true,
            },
            TerrainType::Steppe => SedimentTraits {
                allow_buildings: true,
                allow_mining: true,
                allow_forests: false,
                allow_plants: true,
                allow_ores: true,
            },
            TerrainType::Stony => SedimentTraits {
                allow_buildings: true,
                allow_mining: true,
                allow_forests: false,
                allow_plants: false,
                allow_ores: true,
            },
            TerrainType::Swamp => SedimentTraits {
                allow_buildings: false,
                allow_mining: true,
                allow_forests: false,
                allow_plants: false,
                allow_ores: true,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Reflect)]
pub struct TileData {
    pub terrain: TerrainType,
    pub forest_type: ForestType,
    pub forest_density: f32, // 0.0 to 1.0
    pub elevation: f32,
    pub humidity: f32,
    pub temperature: f32,
    pub roofed: bool,
    pub is_ocean: bool,
    pub faction_id: Option<u32>,
    pub landscape_feature: LandscapeFeature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct EdgeCoord {
    pub a: HexCoord,
    pub b: HexCoord,
}

impl EdgeCoord {
    pub fn new(mut a: HexCoord, mut b: HexCoord) -> Self {
        // Гарантируем стабильный порядок для HashMap
        if a.q > b.q || (a.q == b.q && a.r > b.r) {
            std::mem::swap(&mut a, &mut b);
        }
        Self { a, b }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub struct EdgeData {
    pub is_cliff: bool,
    /// Если true, направление "вниз" от a к b. Если false - от b к a.
    pub direction: bool,
}

#[derive(Resource, Default, Clone)]
pub struct MapData {
    pub width: u32,
    pub height: u32,
    pub tiles: HashMap<HexCoord, TileData>,
    pub edges: HashMap<EdgeCoord, EdgeData>,
    pub validation_errors: Vec<String>,
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
        self.get_tile(q, r)
            .map_or(0.0, |t| t.elevation * MAX_HEIGHT)
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

#[derive(Resource, Clone, Copy)]
pub struct WorldSeed(pub u32);

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
