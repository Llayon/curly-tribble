// src/map/export.rs
use crate::map::MapData;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct MapExportPlugin;

impl Plugin for MapExportPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TilePackageData {
    pub q: i32,
    pub r: i32,
    pub ocean_state: u8,
    pub terrain: u8,
    pub elevation: f32,
    pub faction_id: Option<u32>,
    pub forest_type: u8,
    pub forest_density: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MapPackage {
    pub title: String,
    pub author: String,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<TilePackageData>,
}

#[allow(clippy::missing_errors_doc)]
pub fn export_map_to_json(map_data: &MapData, file_path: &std::path::Path) -> Result<(), String> {
    let mut tiles = Vec::new();
    for (coord, tile) in &map_data.tiles {
        tiles.push(TilePackageData {
            q: coord.q,
            r: coord.r,
            ocean_state: tile.ocean_state as u8,
            terrain: tile.terrain as u8,
            elevation: tile.elevation,
            faction_id: tile.faction_id,
            forest_type: tile.forest_type as u8,
            forest_density: tile.forest_density,
        });
    }

    let package = MapPackage {
        title: "Savage Fantasy Custom Map".to_string(),
        author: "Map Designer".to_string(),
        width: map_data.width,
        height: map_data.height,
        tiles,
    };

    let json = serde_json::to_string_pretty(&package).map_err(|e| e.to_string())?;

    if let Some(parent) = file_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    std::fs::write(file_path, json).map_err(|e| e.to_string())
}

#[allow(clippy::missing_errors_doc)]
pub fn import_map_from_json(file_path: &std::path::Path) -> Result<MapPackage, String> {
    let content = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    serde_json::from_str::<MapPackage>(&content).map_err(|e| e.to_string())
}
