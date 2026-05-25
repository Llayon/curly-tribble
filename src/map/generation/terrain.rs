use crate::game_state::EditorPhase;
use crate::map::navigation::NavigationMap;
use crate::map::terrain_gen::{TerrainConfig, TerrainGenerator};
use crate::map::{
    EdgeCoord, EdgeData, ForestType, HexCoord, LandscapeFeature, MapData, TerrainType, TileData,
    WorldSeed, HEX_SIZE, MAX_HEIGHT,
};
use bevy::prelude::*;
use noise::{Fbm, NoiseFn, OpenSimplex};
use std::collections::{HashMap, VecDeque};

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
                    |t| t.is_ocean,
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
                    get_terrain_from_climate(temp, humid, norm_elev)
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

            let tile_data = TileData {
                terrain,
                forest_type: f_type,
                forest_density: f_density,
                elevation: normalized_elevation,
                temperature: temp_val,
                humidity: humid_val,
                roofed: false,
                is_ocean,
                faction_id,
                landscape_feature: feature,
            };
            new_tiles.insert(coord, tile_data);
        }
    }
    map_data.tiles = new_tiles;

    let mut distance_field: HashMap<HexCoord, u32> = HashMap::new();
    let mut queue = VecDeque::new();
    for (coord, tile) in &map_data.tiles {
        if tile.is_ocean {
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

    let plateau_noise = Fbm::<OpenSimplex>::new(seed.value() + 60);

    if force_reset || auto_fill == Some(EditorPhase::Landscape) {
        for (coord, tile) in map_data.tiles.iter_mut() {
            if tile.is_ocean || tile.faction_id.is_some() {
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
    }

    if force_reset || auto_fill == Some(EditorPhase::Landscape) {
        map_data.edges.clear();
        let mut new_cliffs = Vec::new();

        {
            let coords: Vec<_> = map_data.tiles.keys().copied().collect();
            for coord in coords {
                if let Some(tile_a) = map_data.get_tile(coord.q, coord.r) {
                    let feat_a = tile_a.landscape_feature;
                    for n in coord.neighbors() {
                        if let Some(tile_b) = map_data.get_tile(n.q, n.r) {
                            let feat_b = tile_b.landscape_feature;
                            let mut is_cliff = false;
                            let mut direction = true;

                            if (feat_a != feat_b)
                                && (feat_a == LandscapeFeature::Mountain
                                    || feat_a == LandscapeFeature::Plateau
                                    || feat_b == LandscapeFeature::Mountain
                                    || feat_b == LandscapeFeature::Plateau)
                            {
                                is_cliff = true;
                                direction = feat_a == LandscapeFeature::Mountain
                                    || feat_a == LandscapeFeature::Plateau;
                            } else if !tile_a.is_ocean
                                && !tile_b.is_ocean
                                && tile_a.faction_id.is_none()
                                && tile_b.faction_id.is_none()
                            {
                                let d_a = *distance_field.get(&coord).unwrap_or(&0) as i32;
                                let d_b = *distance_field.get(&n).unwrap_or(&0) as i32;
                                if d_a != d_b && (d_a % 12 == 0 || d_b % 12 == 0) {
                                    let fault_noise = plateau_noise.get([
                                        f64::from(coord.q) * 0.05,
                                        f64::from(coord.r) * 0.05,
                                    ]);
                                    if fault_noise > 0.4 {
                                        is_cliff = true;
                                        direction = d_a > d_b;
                                    }
                                }
                            }

                            if is_cliff {
                                let edge = EdgeCoord::new(coord, n);
                                new_cliffs.push((
                                    edge,
                                    EdgeData {
                                        is_cliff: true,
                                        direction,
                                    },
                                ));
                            }
                        }
                    }
                }
            }
        }

        for (edge, data) in new_cliffs {
            map_data.edges.insert(edge, data);
        }
    }

    let has_player_start = map_data.tiles.values().any(|t| t.faction_id == Some(1));
    if force_reset || !has_player_start {
        super::factions::auto_spawn_player_territory(map_data, seed.value());
    }

    for q in -half_w..half_w {
        for r in -half_h..half_h {
            let tile_data = map_data.get_tile(q, r).copied().unwrap_or_default();
            let terrain = tile_data.terrain;

            let mut cost = crate::map::navigation::COST_BASE;
            if tile_data.is_ocean {
                cost = crate::map::navigation::COST_BLOCKER;
            } else if map_data.is_too_steep(q, r) {
                cost = crate::map::navigation::COST_BLOCKER;
            } else {
                match terrain {
                    TerrainType::Swamp => {
                        cost = 50;
                    }
                    TerrainType::Stony => {
                        cost = 80;
                    }
                    _ => {}
                }
            }
            nav_map.grid.insert(IVec2::new(q, r), cost);
        }
    }

    commands.queue(crate::economy::mesh_gen::SpawnGlobalTerrainCommand {
        map_data: map_data.clone(),
        phase,
        faction_manager: faction_manager.clone(),
        config: (*terrain_config).clone(),
    });
}

pub fn get_terrain_from_climate(temp: f32, humid: f32, elev: f32) -> TerrainType {
    if elev > 0.8 {
        return TerrainType::Stony;
    }
    if humid > 0.7 {
        if temp < 0.3 {
            TerrainType::Swamp
        } else {
            TerrainType::Mossy
        }
    } else if humid < 0.3 {
        if temp > 0.7 {
            TerrainType::Dusty
        } else {
            TerrainType::Steppe
        }
    } else if temp < 0.3 {
        TerrainType::Stony
    } else {
        TerrainType::Grass
    }
}
