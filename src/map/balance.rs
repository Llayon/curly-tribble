// src/map/balance.rs
use crate::map::deposits::{DepositType, ResourceDeposit};
use crate::map::resources::BerryBush;
use crate::map::{ForestType, HexCoord, MapData, HEX_SIZE};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterResource {
    Wood,
    Food,
    Flax,
}

pub struct BalancePlugin;

impl Plugin for BalancePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            validate_starter_resources
                .run_if(resource_changed::<MapData>)
                .in_set(crate::sets::GameSet::Logic),
        );
    }
}

#[must_use]
pub fn get_starter_deficiencies(
    map_data: &MapData,
    q_deposits: &Query<&ResourceDeposit>,
    q_bushes: &Query<&Transform, With<BerryBush>>,
) -> Vec<StarterResource> {
    let mut deficiencies = Vec::new();

    // 1. Check Wood
    let mut wood_count = 0;
    for tile in map_data.tiles.values() {
        if tile.faction_id == Some(1)
            && tile.forest_type != ForestType::None
            && tile.forest_density > 0.1
        {
            wood_count += 1;
        }
    }
    if wood_count < 5 {
        deficiencies.push(StarterResource::Wood);
    }

    // 2. Check Food
    let mut has_food = false;
    for deposit in q_deposits {
        if let Some(tile) = map_data.tiles.get(&deposit.hex_coord) {
            if tile.faction_id == Some(1)
                && matches!(
                    deposit.deposit_type,
                    DepositType::Raspberries
                        | DepositType::Rabbit
                        | DepositType::Deer
                        | DepositType::Boar
                        | DepositType::Pumpkin
                        | DepositType::WildWheat
                )
            {
                has_food = true;
                break;
            }
        }
    }
    if !has_food {
        for transform in q_bushes {
            let coord = HexCoord::from_world(transform.translation, HEX_SIZE);
            if let Some(tile) = map_data.tiles.get(&coord) {
                if tile.faction_id == Some(1) {
                    has_food = true;
                    break;
                }
            }
        }
    }
    if !has_food {
        deficiencies.push(StarterResource::Food);
    }

    // 3. Check Flax
    let mut has_flax = false;
    for deposit in q_deposits {
        if let Some(tile) = map_data.tiles.get(&deposit.hex_coord) {
            if tile.faction_id == Some(1) && deposit.deposit_type == DepositType::WildFlax {
                has_flax = true;
                break;
            }
        }
    }
    if !has_flax {
        deficiencies.push(StarterResource::Flax);
    }

    deficiencies
}

pub fn validate_starter_resources(
    mut map_data: ResMut<MapData>,
    q_deposits: Query<&ResourceDeposit>,
    q_bushes: Query<&Transform, With<BerryBush>>,
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    let has_faction_1 = map_data.tiles.values().any(|t| t.faction_id == Some(1));
    if !has_faction_1 || *phase.get() < crate::game_state::EditorPhase::Balance {
        return;
    }

    let deficiencies = get_starter_deficiencies(&map_data, &q_deposits, &q_bushes);
    if !deficiencies.is_empty() {
        let mut def_names = Vec::new();
        for def in &deficiencies {
            match def {
                StarterResource::Wood => def_names.push("Wood"),
                StarterResource::Food => def_names.push("Food"),
                StarterResource::Flax => def_names.push("Flax"),
            }
        }
        let err_msg = format!(
            "Faction 1 territory is deficient in {}. Use Auto-Balance to fix.",
            def_names.join("/")
        );
        if !map_data.validation_errors.contains(&err_msg) {
            map_data.validation_errors.push(err_msg);
        }
    }
}
