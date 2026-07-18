use crate::game_state::{FactionManager, FactionType};
use crate::map::data::OceanState;
use crate::map::{
    EnemyCamp, EnemyCampBundle, HexCoord, LandscapeFeature, MapData, MapEntity, PoiBundle, PoiType,
    PointOfInterest, HEX_SIZE,
};
use bevy::prelude::*;
use rand::prelude::*;
use std::collections::HashMap;

pub struct NpcGenerationPlugin;

impl Plugin for NpcGenerationPlugin {
    fn build(&self, _app: &mut App) {}
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
        let f_type = faction.map_or(FactionType::Neutral, |f| f.faction_type);

        let mut sum_q = 0;
        let mut sum_r = 0;
        for c in &coords {
            sum_q += c.q;
            sum_r += c.r;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let center = HexCoord::new(sum_q / coords.len() as i32, sum_r / coords.len() as i32);

        let Some(&best_coord) = coords.iter().min_by_key(|c| {
            let mut flatness = 0.0;
            let h = map_data.get_tile(c.q, c.r).map_or(0.0, |t| t.elevation);
            for n in c.neighbors() {
                if let Some(nt) = map_data.get_tile(n.q, n.r) {
                    flatness += (nt.elevation - h).abs();
                }
            }
            #[allow(clippy::cast_possible_truncation)]
            {
                c.distance(center) * 100 + (flatness * 1000.0) as i32
            }
        }) else {
            continue;
        };

        match f_type {
            FactionType::Neutral => {
                commands.spawn(PoiBundle {
                    poi: PointOfInterest {
                        poi_type: PoiType::TradePost,
                        hex_coord: best_coord,
                        linked_objective_id: None,
                    },
                    name: Name::new(format!("Village Center (Faction {f_id})")),
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
                    name: Name::new(format!("Main Camp (Faction {f_id})")),
                    transform: Transform::from_translation(best_coord.to_world(HEX_SIZE)),
                    visibility: Visibility::Visible,
                    inherited_visibility: InheritedVisibility::default(),
                    marker: MapEntity,
                });
            }
            FactionType::Player => {}
        }
    }

    spawn_procedural_pois(commands, map_data, &mut rng);
}

fn spawn_procedural_pois(
    commands: &mut Commands,
    map_data: &MapData,
    rng: &mut rand::rngs::StdRng,
) {
    let free_tiles: Vec<_> = map_data
        .tiles
        .iter()
        .filter(|(_, t)| {
            t.ocean_state == OceanState::Land
                && t.faction_id.is_none()
                && t.landscape_feature == LandscapeFeature::None
        })
        .map(|(c, _)| *c)
        .collect();

    for _ in 0..5 {
        if let Some(coord) = free_tiles.choose(&mut *rng) {
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
                name: Name::new(format!("{p_type:?} (Procedural)")),
                transform: Transform::from_translation(coord.to_world(HEX_SIZE)),
                visibility: Visibility::Visible,
                inherited_visibility: InheritedVisibility::default(),
                marker: MapEntity,
            });
        }
    }
}
