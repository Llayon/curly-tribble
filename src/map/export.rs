// src/map/export.rs
use crate::game_state::{MineDepth, MineType};
use crate::map::deposits::DepositType;
use crate::map::poi::PoiType;
use crate::map::treasures::{ArtifactType, TreasureItem};
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
pub struct MinePackageData {
    pub q: i32,
    pub r: i32,
    pub mine_type: MineType,
    pub amount: u32,
    pub depth: MineDepth,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreasurePackageData {
    pub q: i32,
    pub r: i32,
    pub contents: Vec<TreasureItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArtifactPackageData {
    pub q: i32,
    pub r: i32,
    pub artifact_type: ArtifactType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubHexPackageData {
    pub deposit_type: DepositType,
    pub pos: [f32; 3],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CampPackageData {
    pub q: i32,
    pub r: i32,
    pub sub_faction: String,
    pub difficulty: f32,
    pub combat_power: u32,
    pub camp_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildingPackageData {
    pub q: i32,
    pub r: i32,
    pub building_type: crate::map::buildings::BuildingType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropPackageData {
    pub prop_type: crate::map::props::PropType,
    pub pos: [f32; 3],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PoiPackageData {
    pub q: i32,
    pub r: i32,
    pub poi_type: PoiType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MapPackage {
    pub title: String,
    pub author: String,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<TilePackageData>,
    pub mines: Vec<MinePackageData>,
    pub treasures: Vec<TreasurePackageData>,
    pub artifacts: Vec<ArtifactPackageData>,
    pub subhex_deposits: Vec<SubHexPackageData>,
    pub enemy_camps: Vec<CampPackageData>,
    pub buildings: Vec<BuildingPackageData>,
    pub props: Vec<PropPackageData>,
    pub pois: Vec<PoiPackageData>,
}

pub struct ExportMapCommand {
    pub file_path: std::path::PathBuf,
}

impl Command for ExportMapCommand {
    fn apply(self, world: &mut World) {
        let _ = export_world_to_json(world, &self.file_path);
    }
}

pub trait ExportMapExt {
    fn export_map_package(&mut self, path: impl Into<std::path::PathBuf>);
}

impl ExportMapExt for Commands<'_, '_> {
    fn export_map_package(&mut self, path: impl Into<std::path::PathBuf>) {
        self.queue(ExportMapCommand {
            file_path: path.into(),
        });
    }
}

#[allow(clippy::missing_errors_doc, clippy::too_many_lines)]
pub fn export_world_to_json(world: &mut World, file_path: &std::path::Path) -> Result<(), String> {
    let (width, height, tiles) = {
        let map_data = world
            .get_resource::<MapData>()
            .ok_or_else(|| "MapData resource missing".to_string())?;

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
        (map_data.width, map_data.height, tiles)
    };

    let mut mines = Vec::new();
    let mut q_mines = world.query::<&crate::map::mines::MineDeposit>();
    for mine in q_mines.iter(world) {
        mines.push(MinePackageData {
            q: mine.hex_coord.q,
            r: mine.hex_coord.r,
            mine_type: mine.mine_type,
            amount: mine.amount,
            depth: mine.depth,
        });
    }

    let mut treasures = Vec::new();
    let mut q_treasures = world.query::<&crate::map::treasures::TreasureDeposit>();
    for tr in q_treasures.iter(world) {
        treasures.push(TreasurePackageData {
            q: tr.hex_coord.q,
            r: tr.hex_coord.r,
            contents: tr.contents.clone(),
        });
    }

    let mut artifacts = Vec::new();
    let mut q_artifacts = world.query::<&crate::map::artifacts::Artifact>();
    for art in q_artifacts.iter(world) {
        if let crate::map::artifacts::ArtifactLocation::OnGround(coord) = art.location {
            artifacts.push(ArtifactPackageData {
                q: coord.q,
                r: coord.r,
                artifact_type: art.artifact_type,
            });
        }
    }

    let mut subhex_deposits = Vec::new();
    let mut q_subhex = world.query::<&crate::map::subhex::SubHexDeposit>();
    for sub in q_subhex.iter(world) {
        subhex_deposits.push(SubHexPackageData {
            deposit_type: sub.deposit_type,
            pos: [sub.world_pos.x, sub.world_pos.y, sub.world_pos.z],
        });
    }

    let mut enemy_camps = Vec::new();
    let mut q_camps = world.query::<&crate::map::camps::EnemyCamp>();
    for camp in q_camps.iter(world) {
        enemy_camps.push(CampPackageData {
            q: camp.hex_coord.q,
            r: camp.hex_coord.r,
            sub_faction: camp.sub_faction.clone(),
            difficulty: camp.difficulty,
            combat_power: camp.combat_power,
            camp_count: camp.camp_count,
        });
    }

    let mut buildings = Vec::new();
    let mut q_bldg = world.query::<&crate::map::buildings::BuildingStructure>();
    for b in q_bldg.iter(world) {
        buildings.push(BuildingPackageData {
            q: b.hex_coord.q,
            r: b.hex_coord.r,
            building_type: b.building_type,
        });
    }

    let mut props = Vec::new();
    let mut q_props = world.query::<&crate::map::props::DecorativeProp>();
    for p in q_props.iter(world) {
        props.push(PropPackageData {
            prop_type: p.prop_type,
            pos: [p.world_pos.x, p.world_pos.y, p.world_pos.z],
        });
    }

    let mut pois = Vec::new();
    let mut q_poi = world.query::<&crate::map::poi::PointOfInterest>();
    for poi in q_poi.iter(world) {
        pois.push(PoiPackageData {
            q: poi.hex_coord.q,
            r: poi.hex_coord.r,
            poi_type: poi.poi_type,
        });
    }

    let package = MapPackage {
        title: "Savage Fantasy Custom Map".to_string(),
        author: "Map Designer".to_string(),
        width,
        height,
        tiles,
        mines,
        treasures,
        artifacts,
        subhex_deposits,
        enemy_camps,
        buildings,
        props,
        pois,
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
