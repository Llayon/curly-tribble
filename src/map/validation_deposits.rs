use crate::map::data::OceanState;
use crate::map::{ForestType, HiddenTreasure, MapData, TerrainType, TreasureDeposit, HEX_SIZE};
use bevy::prelude::*;
use std::collections::HashSet;

pub struct ValidationDepositsPlugin;

impl Plugin for ValidationDepositsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn validate_treasures(
    mut map_data: ResMut<MapData>,
    q_treasures: Query<(Entity, &TreasureDeposit, Option<&HiddenTreasure>)>,
    q_children: Query<&Children>,
    q_contains_artifact: Query<&crate::map::treasures::ContainsArtifact>,
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    if !map_data.is_changed() || *phase.get() < crate::game_state::EditorPhase::Treasures {
        return;
    }

    let mut occupied = HashSet::new();
    let mut errors = Vec::new();

    for (entity, deposit, hidden) in &q_treasures {
        let coord = deposit.hex_coord;

        // 1. Ensure no HiddenTreasure is placed in an is_ocean hex
        if hidden.is_some() {
            if let Some(tile) = map_data.get_tile(coord.q, coord.r) {
                if tile.ocean_state == OceanState::Ocean {
                    errors.push(format!(
                        "Hidden Treasure cannot be placed in the ocean at {coord:?}."
                    ));
                }
            }
        }

        // 2. Ensure no two treasures are on the same hex
        if occupied.contains(&coord) {
            errors.push(format!(
                "Multiple treasures found on the same hex at {coord:?}."
            ));
        }
        // 3. Ensure treasure is not empty
        let has_child_artifact = q_children
            .get(entity)
            .is_ok_and(|children| children.iter().any(|c| q_contains_artifact.get(c).is_ok()));

        if deposit.contents.is_empty() && !has_child_artifact {
            errors.push(format!(
                "Treasure at {coord:?} is empty. It must have at least 1 defined content."
            ));
        }

        occupied.insert(coord);
    }

    for err in errors {
        map_data.validation_errors.push(err);
    }
}

pub fn validate_bio_habitats(
    map_data: Res<MapData>,
    mut q_deposits: Query<&mut crate::map::ResourceDeposit, With<crate::map::ResourceDeposit>>,
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    if !map_data.is_changed() || *phase.get() < crate::game_state::EditorPhase::Plants {
        return;
    }

    for mut deposit in &mut q_deposits {
        let coord = deposit.hex_coord;
        let Some(tile) = map_data.get_tile(coord.q, coord.r) else {
            deposit.habitat_valid = false;
            continue;
        };

        match deposit.deposit_type {
            crate::map::DepositType::Deer | crate::map::DepositType::Boar => {
                let mut has_forest = tile.forest_type != ForestType::None;
                if !has_forest {
                    for neighbor in coord.neighbors() {
                        if let Some(nt) = map_data.get_tile(neighbor.q, neighbor.r) {
                            if nt.forest_type != ForestType::None {
                                has_forest = true;
                                break;
                            }
                        }
                    }
                }
                deposit.habitat_valid = has_forest && tile.ocean_state == OceanState::Land;
            }
            crate::map::DepositType::OceanFish => {
                deposit.habitat_valid = tile.ocean_state == OceanState::Ocean;
            }
            crate::map::DepositType::Rabbit
            | crate::map::DepositType::WildFlax
            | crate::map::DepositType::Raspberries
            | crate::map::DepositType::Pumpkin
            | crate::map::DepositType::WildWheat => {
                deposit.habitat_valid =
                    tile.terrain.allows_plants() && tile.ocean_state == OceanState::Land;
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn validate_mines(
    mut map_data: ResMut<MapData>,
    q_mines: Query<(Entity, &crate::map::mines::MineDeposit)>,
    nav_map: Res<crate::map::navigation::NavigationMap>,
    gameplay: Res<crate::map::surface_gameplay::types::SurfaceGameplayMap>,
    gameplay_state: Res<crate::map::surface_gameplay::runtime::SurfaceGameplayGenerationState>,
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    if !map_data.is_changed() || *phase.get() < crate::game_state::EditorPhase::Mines {
        return;
    }

    let mut errors = Vec::new();
    let mut occupied = HashSet::new();

    let mut start_coord = None;
    for (coord, tile) in &map_data.tiles {
        if tile.faction_id == Some(1) {
            start_coord = Some(*coord);
            break;
        }
    }

    for (_entity, mine) in &q_mines {
        let coord = mine.hex_coord;

        if let Some(tile) = map_data.get_tile(coord.q, coord.r) {
            if tile.ocean_state == OceanState::Ocean {
                errors.push(format!(
                    "Mine of type {:?} cannot be placed in the ocean at {:?}.",
                    mine.mine_type, coord
                ));
            } else {
                match mine.mine_type {
                    crate::game_state::MineType::Stone => {
                        if !matches!(tile.terrain, TerrainType::Stony | TerrainType::Dirt)
                            && tile.landscape_feature != crate::map::LandscapeFeature::Plateau
                            && tile.landscape_feature != crate::map::LandscapeFeature::Mountain
                        {
                            errors.push(format!(
                                "Stone deposit at {:?} is on invalid terrain {:?}. Stone requires Stony, Dirt, Plateau, or Mountain.",
                                coord, tile.terrain
                            ));
                        }
                    }
                    crate::game_state::MineType::Iron
                    | crate::game_state::MineType::Gold
                    | crate::game_state::MineType::Copper => {
                        if tile.landscape_feature != crate::map::LandscapeFeature::Plateau
                            && tile.landscape_feature != crate::map::LandscapeFeature::Mountain
                            && tile.terrain != TerrainType::Stony
                        {
                            errors.push(format!(
                                "{:?} vein at {:?} must be on Mountain, Plateau, or Stony terrain.",
                                mine.mine_type, coord
                            ));
                        }
                    }
                    crate::game_state::MineType::Coal => {
                        if !matches!(
                            tile.terrain,
                            TerrainType::Steppe | TerrainType::Dirt | TerrainType::Grass
                        ) && tile.landscape_feature != crate::map::LandscapeFeature::Plateau
                            && tile.landscape_feature != crate::map::LandscapeFeature::Mountain
                        {
                            errors.push(format!(
                                "Coal vein at {:?} is on invalid terrain {:?}. Coal requires Steppe, Dirt, Grass, Plateau, or Mountain.",
                                coord, tile.terrain
                            ));
                        }
                    }
                }
            }
        }

        if occupied.contains(&coord) {
            errors.push(format!(
                "Multiple mines found on the same hex at {coord:?}."
            ));
        }
        occupied.insert(coord);

        if let Some(start) = start_coord {
            let start_pos = start.to_world(HEX_SIZE);
            let mut is_accessible = false;
            let gameplay_ready = gameplay_state.last_outcome
                == crate::map::surface_gameplay::runtime::SurfaceGameplayGenerationOutcome::Success;
            if gameplay_ready {
                for n in coord.neighbors() {
                    if let Some(tile) = map_data.get_tile(n.q, n.r) {
                        if tile.ocean_state == OceanState::Land {
                            let target_pos = n.to_world(HEX_SIZE);
                            if crate::map::navigation::compute_astar_path(
                                &gameplay,
                                &nav_map.grid,
                                start_pos,
                                target_pos,
                                1.5,
                            )
                            .is_some()
                            {
                                is_accessible = true;
                                break;
                            }
                        }
                    }
                }
            }
            if !is_accessible {
                errors.push(format!(
                    "Mine of type {:?} at {:?} is blocked and inaccessible by land.",
                    mine.mine_type, coord
                ));
            }
        }

        if matches!(
            mine.mine_type,
            crate::game_state::MineType::Gold | crate::game_state::MineType::Iron
        ) {
            let mut has_same_neighbor = false;
            for neighbor in coord.neighbors() {
                for (_, other) in &q_mines {
                    if other.hex_coord == neighbor && other.mine_type == mine.mine_type {
                        has_same_neighbor = true;
                        break;
                    }
                }
                if has_same_neighbor {
                    break;
                }
            }
            if !has_same_neighbor {
                errors.push(format!(
                    "Warning: Isolated {:?} vein at {:?}. Consider clustering 2-4 tiles together.",
                    mine.mine_type, coord
                ));
            }
        }
    }

    for err in errors {
        map_data.validation_errors.push(err);
    }
}
