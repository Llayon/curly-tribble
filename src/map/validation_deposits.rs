use crate::map::data::OceanState;
use crate::map::{ForestType, HiddenTreasure, MapData, TreasureDeposit};
use bevy::prelude::*;
use std::collections::HashSet;

pub struct ValidationDepositsPlugin;

impl Plugin for ValidationDepositsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn validate_treasures(
    mut map_data: ResMut<MapData>,
    q_treasures: Query<(&TreasureDeposit, Option<&HiddenTreasure>)>,
) {
    if !map_data.is_changed() {
        return;
    }

    let mut occupied = HashSet::new();
    let mut errors = Vec::new();

    for (deposit, hidden) in &q_treasures {
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
        if deposit.contents.is_empty() {
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
) {
    if !map_data.is_changed() {
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
