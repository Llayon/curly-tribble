use crate::map::TerrainType;
use bevy::prelude::*;

pub struct ClimateGenerationPlugin;

impl Plugin for ClimateGenerationPlugin {
    fn build(&self, _app: &mut App) {}
}

#[must_use]
pub fn get_terrain_from_climate(temp: f32, humid: f32, elev: f32) -> TerrainType {
    if elev > 0.8 {
        return TerrainType::Stony;
    }
    if humid > 0.7 {
        if temp < 0.3 {
            TerrainType::Swamp
        } else {
            TerrainType::Mossy
        }
    } else if humid < 0.3 {
        if temp > 0.7 {
            TerrainType::Dusty
        } else {
            TerrainType::Steppe
        }
    } else if temp < 0.3 {
        TerrainType::Stony
    } else {
        TerrainType::Grass
    }
}
