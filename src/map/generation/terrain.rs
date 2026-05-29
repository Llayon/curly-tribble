use crate::game_state::EditorPhase;
use crate::map::data::{OceanState, RoofState};
use crate::map::navigation::NavigationMap;
use crate::map::terrain_gen::{TerrainConfig, TerrainGenerator};
use crate::map::{
    ForestType, HexCoord, LandscapeFeature, MapData, TerrainType, TileData, WorldSeed, HEX_SIZE,
    MAX_HEIGHT,
};
use bevy::prelude::*;
use noise::{Fbm, NoiseFn, OpenSimplex};
use std::collections::{HashMap, VecDeque};

pub struct TerrainGenerationPlugin;

impl Plugin for TerrainGenerationPlugin {
    fn build(&self, _app: &mut App) {}
}

#[allow(clippy::cast_possible_truncation)]
pub fn spawn_map_internal(
    commands: &mut Commands,
    terrain_gen: &TerrainGenerator,
    terrain_config: &TerrainConfig,
    seed: &WorldSeed,
    map_data: &mut MapData,
    nav_map: &mut NavigationMap,
    faction_manager: &crate::game_state::FactionManager,
    phase: EditorPhase,
    force_reset: bool,
    auto_fill: Option<EditorPhase>,
) {
    let temp_noise = Fbm::<OpenSimplex>::new(seed.value() + 1);
    let humid_noise = Fbm::<OpenSimplex>::new(seed.value() + 2);

    let width = terrain_config.map_width;
    let height = terrain_config.map_height;
    let half_w = (width / 2).cast_signed();
    let half_h = (height / 2).cast_signed();

    map_data.width = width;
    map_data.height = height;

    let mut new_tiles = HashMap::new();

    for q in -half_w..half_w {
        for r in -half_h..half_h {
            let coord = HexCoord::new(q, r);
            let world_pos = coord.to_world(HEX_SIZE);
            let is_border =
                q <= -half_w + 1 || q >= half_w - 2 || r <= -half_h + 1 || r >= half_h - 2;

            let mut is_ocean = if force_reset {
                let shape_val =
                    terrain_gen.get_shape_value(terrain_config, world_pos.x, world_pos.z);
                is_border || shape_val <= 0.0
            } else {
                map_data.get_tile(q, r).map_or_else(
                    || {
                        let shape_val =
                            terrain_gen.get_shape_value(terrain_config, world_pos.x, world_pos.z);
                        is_border || shape_val <= 0.0
                    },
                    |t| t.ocean_state == OceanState::Ocean,
                )
            };

            if is_border {
                is_ocean = true;
            }

            let faction_id = if force_reset {
                None
            } else {
                map_data.get_tile(q, r).and_then(|t| t.faction_id)
            };

            let (terrain, temp_val, humid_val, normalized_elevation, feature) = if phase
                == EditorPhase::Shape
            {
                (TerrainType::Grass, 0.5, 0.5, 0.1, LandscapeFeature::None)
            } else {
                let elevation = terrain_gen.get_elevation(terrain_config, world_pos.x, world_pos.z);
                let norm_elev = (elevation / MAX_HEIGHT).clamp(0.0, 1.0);
                let temp = ((temp_noise
                    .get([f64::from(world_pos.x) * 0.05, f64::from(world_pos.z) * 0.05])
                    as f32)
                    + 1.0)
                    * 0.5;
                let humid = ((humid_noise
                    .get([f64::from(world_pos.x) * 0.05, f64::from(world_pos.z) * 0.05])
                    as f32)
                    + 1.0)
                    * 0.5;

                let t = if is_ocean {
                    TerrainType::Grass
                } else if force_reset || auto_fill == Some(EditorPhase::Sediments) {
                    super::climate::get_terrain_from_climate(temp, humid, norm_elev)
                } else {
                    map_data
                        .get_tile(q, r)
                        .map_or(TerrainType::Grass, |old| old.terrain)
                };

                let feat = if force_reset {
                    LandscapeFeature::None
                } else {
                    map_data
                        .get_tile(q, r)
                        .map_or(LandscapeFeature::None, |t| t.landscape_feature)
                };
                (t, temp, humid, norm_elev, feat)
            };

            let (f_type, f_density) = if !is_ocean && feature == LandscapeFeature::None {
                if force_reset || auto_fill == Some(EditorPhase::Sediments) {
                    if humid_val > 0.6 && temp_val > 0.3 {
                        let density = (humid_val - 0.4).max(0.0) * 0.8;
                        if temp_val < 0.5 || normalized_elevation > 0.6 {
                            (ForestType::Coniferous, density)
                        } else {
                            (ForestType::Deciduous, density)
                        }
                    } else {
                        (ForestType::None, 0.0)
                    }
                } else {
                    map_data
                        .get_tile(q, r)
                        .map_or((ForestType::None, 0.0), |old| {
                            (old.forest_type, old.forest_density)
                        })
                }
            } else {
                (ForestType::None, 0.0)
            };

            new_tiles.insert(
                coord,
                TileData {
                    terrain,
                    forest_type: f_type,
                    forest_density: f_density,
                    elevation: normalized_elevation,
                    temperature: temp_val,
                    humidity: humid_val,
                    roof_state: RoofState::Open,
                    ocean_state: if is_ocean {
                        OceanState::Ocean
                    } else {
                        OceanState::Land
                    },
                    faction_id,
                    landscape_feature: feature,
                },
            );
        }
    }
    map_data.tiles = new_tiles;

    let mut distance_field = HashMap::new();
    let mut queue = VecDeque::new();
    for (coord, tile) in &map_data.tiles {
        if tile.ocean_state == OceanState::Ocean {
            distance_field.insert(*coord, 0);
            queue.push_back(*coord);
        }
    }
    while let Some(curr) = queue.pop_front() {
        if let Some(&curr_dist) = distance_field.get(&curr) {
            for n in curr.neighbors() {
                if map_data.tiles.contains_key(&n) && !distance_field.contains_key(&n) {
                    distance_field.insert(n, curr_dist + 1);
                    queue.push_back(n);
                }
            }
        }
    }

    if force_reset || auto_fill == Some(EditorPhase::Landscape) {
        let plateau_noise = Fbm::<OpenSimplex>::new(seed.value() + 60);
        for (coord, tile) in map_data.tiles.iter_mut() {
            if tile.ocean_state == OceanState::Ocean || tile.faction_id.is_some() {
                continue;
            }
            let dist = *distance_field.get(coord).unwrap_or(&0);
            let world_pos = coord.to_world(HEX_SIZE);
            let p_noise = ((plateau_noise
                .get([f64::from(world_pos.x) * 0.1, f64::from(world_pos.z) * 0.1])
                as f32)
                + 1.0)
                * 0.5;
            if dist > 8 && p_noise > 0.7 {
                tile.landscape_feature = LandscapeFeature::Mountain;
            } else if dist > 4 && p_noise > 0.5 {
                tile.landscape_feature = LandscapeFeature::Plateau;
            } else if dist > 3 && p_noise < 0.15 {
                tile.landscape_feature = LandscapeFeature::Lake;
            }
        }
        super::cliffs::generate_cliffs(map_data, &distance_field, seed.value());
    }

    if force_reset || !map_data.tiles.values().any(|t| t.faction_id == Some(1)) {
        super::factions::auto_spawn_player_territory(map_data, seed.value());
    }

    for (coord, tile_data) in &map_data.tiles {
        let mut cost = crate::map::navigation::COST_BASE;
        if tile_data.ocean_state == OceanState::Ocean || map_data.is_too_steep(coord.q, coord.r) {
            cost = crate::map::navigation::COST_BLOCKER;
        } else {
            match tile_data.terrain {
                TerrainType::Swamp => cost = 50,
                TerrainType::Stony => cost = 80,
                _ => {}
            }
        }
        nav_map.grid.insert(IVec2::new(coord.q, coord.r), cost);
    }

    commands.queue(crate::economy::mesh_gen::SpawnGlobalTerrainCommand {
        map_data: map_data.clone(),
        phase,
        faction_manager: faction_manager.clone(),
        config: (*terrain_config).clone(),
    });
}
