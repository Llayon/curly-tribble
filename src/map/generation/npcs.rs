use crate::game_state::{FactionManager, FactionType};
use crate::map::{
    DepositType, EnemyCamp, EnemyCampBundle, ForestType, HexCoord, LandscapeFeature, MapData,
    MapEntity, PoiBundle, PoiType, PointOfInterest, ResourceDeposit, ResourceDepositBundle,
    TerrainType, HEX_SIZE,
};
use bevy::prelude::*;
use rand::prelude::*;
use std::collections::HashMap;

pub fn auto_spawn_bio_deposits(commands: &mut Commands, map_data: &MapData, seed: u32) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(seed + 200));

    // 1. OceanFish: Coastal ocean hexes
    let mut coastal_ocean = Vec::new();
    for (coord, tile) in &map_data.tiles {
        if tile.is_ocean {
            let mut near_land = false;
            for neighbor in coord.neighbors() {
                if let Some(nt) = map_data.get_tile(neighbor.q, neighbor.r) {
                    if !nt.is_ocean {
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
            spawn_deposit(commands, *coord, DepositType::OceanFish, rng.gen_range(5..15));
        }
    }

    // 2. Deer/Boar: Forest clusters
    let mut forest_tiles = Vec::new();
    for (coord, tile) in &map_data.tiles {
        if !tile.is_ocean && tile.forest_type != ForestType::None {
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
        if !tile.is_ocean
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
        name: Name::new(format!("{:?} Deposit", d_type)),
        transform: Transform::from_translation(coord.to_world(HEX_SIZE)),
        visibility: Visibility::Visible,
        inherited_visibility: InheritedVisibility::default(),
        marker: MapEntity,
    });
}

pub fn auto_spawn_npcs(
    commands: &mut Commands,
    map_data: &MapData,
    faction_manager: &FactionManager,
    seed: u32,
) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(seed + 100));
    let mut faction_territories: HashMap<u32, Vec<HexCoord>> = HashMap::new();
    for (coord, tile) in &map_data.tiles {
        if let Some(f_id) = tile.faction_id {
            faction_territories.entry(f_id).or_default().push(*coord);
        }
    }
    for (f_id, coords) in faction_territories {
        if f_id == 1 {
            continue;
        }
        let faction = faction_manager.factions.iter().find(|f| f.id == f_id);
        let f_type = faction
            .map(|f| f.faction_type)
            .unwrap_or(FactionType::Neutral);
        let mut sum_q = 0;
        let mut sum_r = 0;
        for c in &coords {
            sum_q += c.q;
            sum_r += c.r;
        }
        let center = HexCoord::new(sum_q / coords.len() as i32, sum_r / coords.len() as i32);
        let best_coord = match coords.iter().min_by_key(|c| {
            let mut flatness = 0.0;
            let h = map_data.get_tile(c.q, c.r).map_or(0.0, |t| t.elevation);
            for n in c.neighbors() {
                if let Some(nt) = map_data.get_tile(n.q, n.r) {
                    flatness += (nt.elevation - h).abs();
                }
            }
            c.distance(center) * 100 + (flatness * 1000.0) as i32
        }) {
            Some(&c) => c,
            None => continue,
        };
        match f_type {
            FactionType::Neutral => {
                commands.spawn(PoiBundle {
                    poi: PointOfInterest {
                        poi_type: PoiType::TradePost,
                        hex_coord: best_coord,
                        linked_objective_id: None,
                    },
                    name: Name::new(format!("Village Center (Faction {})", f_id)),
                    transform: Transform::from_translation(best_coord.to_world(HEX_SIZE)),
                    visibility: Visibility::Visible,
                    inherited_visibility: InheritedVisibility::default(),
                    marker: MapEntity,
                });
            }
            FactionType::Enemy => {
                commands.spawn(EnemyCampBundle {
                    camp: EnemyCamp {
                        hex_coord: best_coord,
                        sub_faction: "Bandits".to_string(),
                        difficulty: 0.5,
                        combat_power: 100,
                        camp_count: 3,
                    },
                    name: Name::new(format!("Main Camp (Faction {})", f_id)),
                    transform: Transform::from_translation(best_coord.to_world(HEX_SIZE)),
                    visibility: Visibility::Visible,
                    inherited_visibility: InheritedVisibility::default(),
                    marker: MapEntity,
                });
            }
            _ => {}
        }
    }
    let free_tiles: Vec<_> = map_data
        .tiles
        .iter()
        .filter(|(_, t)| {
            !t.is_ocean && t.faction_id.is_none() && t.landscape_feature == LandscapeFeature::None
        })
        .map(|(c, _)| *c)
        .collect();
    for _ in 0..5 {
        if let Some(coord) = free_tiles.choose(&mut rng) {
            let p_type = if rng.gen_bool(0.7) {
                PoiType::Treasure
            } else {
                PoiType::Ruins
            };
            commands.spawn(PoiBundle {
                poi: PointOfInterest {
                    poi_type: p_type,
                    hex_coord: *coord,
                    linked_objective_id: None,
                },
                name: Name::new(format!("{:?} (Procedural)", p_type)),
                transform: Transform::from_translation(coord.to_world(HEX_SIZE)),
                visibility: Visibility::Visible,
                inherited_visibility: InheritedVisibility::default(),
                marker: MapEntity,
            });
        }
    }
}
