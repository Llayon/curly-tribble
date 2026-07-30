// src/map/buildings.rs
use crate::economy::assets::GameAssets;
use crate::map::navigation::{NavObstacle, COST_BLOCKER};
use crate::map::{HexCoord, MapData, MapEntity, HEX_SIZE};
use bevy::prelude::*;

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum BuildingType {
    #[default]
    TradePost,
    Shrine,
    Ruins,
    EnemyTent,
    EnemyWatchtower,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct BuildingStructure {
    pub building_type: BuildingType,
    pub hex_coord: HexCoord,
    pub radius_hexes: u32,
    pub target_elevation: f32,
}

#[derive(Bundle)]
pub struct BuildingBundle {
    pub structure: BuildingStructure,
    pub name: Name,
    pub scene: SceneRoot,
    pub transform: Transform,
    pub obstacle: NavObstacle,
    pub marker: MapEntity,
}

pub fn flatten_building_foundation(map_data: &mut MapData, center: HexCoord, radius: u32) {
    let target_elev = if let Some(t) = map_data.get_tile(center.q, center.r) {
        t.elevation
    } else {
        0.0
    };

    let Ok(r_i32) = i32::try_from(radius) else {
        return;
    };

    let mut affected_coords = Vec::new();
    for q in -r_i32..=r_i32 {
        for r in (-r_i32).max(-q - r_i32)..=(r_i32).min(-q + r_i32) {
            affected_coords.push(HexCoord::new(center.q + q, center.r + r));
        }
    }

    for coord in &affected_coords {
        if let Some(tile) = map_data.get_tile_mut(coord.q, coord.r) {
            tile.elevation = target_elev;
        }
    }

    // Blend 1-ring outer neighbors for smooth slopes
    let outer_r = r_i32 + 1;
    for q in -outer_r..=outer_r {
        for r in (-outer_r).max(-q - outer_r)..=(outer_r).min(-q + outer_r) {
            let coord = HexCoord::new(center.q + q, center.r + r);
            if !affected_coords.contains(&coord) {
                if let Some(tile) = map_data.get_tile_mut(coord.q, coord.r) {
                    tile.elevation = (tile.elevation + target_elev) * 0.5;
                }
            }
        }
    }
}

pub fn spawn_building_structure(
    commands: &mut Commands,
    center: HexCoord,
    b_type: BuildingType,
    radius: u32,
    map_data: &mut MapData,
    assets: &GameAssets,
) -> Entity {
    flatten_building_foundation(map_data, center, radius);

    let height = map_data.get_hex_height(center.q, center.r);
    let mut pos = center.to_world(HEX_SIZE);
    pos.y = height;

    let scene_handle = match b_type {
        BuildingType::TradePost | BuildingType::Shrine | BuildingType::Ruins => {
            assets.house_scene.clone()
        }
        BuildingType::EnemyTent | BuildingType::EnemyWatchtower => assets.house_scene.clone(),
    };

    let entity = commands
        .spawn(BuildingBundle {
            structure: BuildingStructure {
                building_type: b_type,
                hex_coord: center,
                radius_hexes: radius,
                target_elevation: height,
            },
            name: Name::new(format!("{b_type:?} Building")),
            scene: SceneRoot(scene_handle),
            transform: Transform::from_translation(pos),
            obstacle: NavObstacle { cost: COST_BLOCKER },
            marker: MapEntity,
        })
        .id();

    entity
}
