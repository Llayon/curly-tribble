use crate::map::data::OceanState;
use crate::map::{
    DepositType, ForestType, HexCoord, MapData, MapEntity, ResourceDeposit, ResourceDepositBundle,
    TerrainType, HEX_SIZE,
};
use bevy::prelude::*;
use rand::prelude::*;

pub struct BioGenerationPlugin;

impl Plugin for BioGenerationPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn auto_spawn_bio_deposits(commands: &mut Commands, map_data: &MapData, seed: u32) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(seed + 200));

    // 1. OceanFish: Coastal ocean hexes
    let mut coastal_ocean = Vec::new();
    for (coord, tile) in &map_data.tiles {
        if tile.ocean_state == OceanState::Ocean {
            let mut near_land = false;
            for neighbor in coord.neighbors() {
                if let Some(nt) = map_data.get_tile(neighbor.q, neighbor.r) {
                    if nt.ocean_state == OceanState::Land {
                        near_land = true;
                        break;
                    }
                }
            }
            if near_land {
                coastal_ocean.push(*coord);
            }
        }
    }

    for _ in 0..15 {
        if let Some(coord) = coastal_ocean.choose(&mut rng) {
            spawn_deposit(
                commands,
                *coord,
                DepositType::OceanFish,
                rng.gen_range(5..15),
            );
        }
    }

    // 2. Deer/Boar: Forest clusters
    let mut forest_tiles = Vec::new();
    for (coord, tile) in &map_data.tiles {
        if tile.ocean_state == OceanState::Land && tile.forest_type != ForestType::None {
            forest_tiles.push(*coord);
        }
    }

    for _ in 0..10 {
        if let Some(coord) = forest_tiles.choose(&mut rng) {
            let d_type = if rng.gen_bool(0.6) {
                DepositType::Deer
            } else {
                DepositType::Boar
            };
            spawn_deposit(commands, *coord, d_type, rng.gen_range(3..8));
        }
    }

    // 3. WildFlax/Raspberries: Fertile or Grass areas
    let mut plant_tiles = Vec::new();
    for (coord, tile) in &map_data.tiles {
        if tile.ocean_state == OceanState::Land
            && (tile.terrain == TerrainType::Fertile || tile.terrain == TerrainType::Grass)
        {
            plant_tiles.push(*coord);
        }
    }

    for _ in 0..20 {
        if let Some(coord) = plant_tiles.choose(&mut rng) {
            let d_type = if rng.gen_bool(0.5) {
                DepositType::WildFlax
            } else {
                DepositType::Raspberries
            };
            spawn_deposit(commands, *coord, d_type, rng.gen_range(10..30));
        }
    }
}

fn spawn_deposit(commands: &mut Commands, coord: HexCoord, d_type: DepositType, amount: u32) {
    commands.spawn(ResourceDepositBundle {
        deposit: ResourceDeposit {
            deposit_type: d_type,
            amount,
            hex_coord: coord,
            habitat_valid: true, // Will be validated by system
        },
        name: Name::new(format!("{d_type:?} Deposit")),
        transform: Transform::from_translation(coord.to_world(HEX_SIZE)),
        visibility: Visibility::Visible,
        inherited_visibility: InheritedVisibility::default(),
        marker: MapEntity,
    });
}
