// src/map/river_gen.rs
use crate::map::terrain_gen::TerrainConfig;
use crate::map::{LandscapeFeature, MapData, TerrainType};
use bevy::prelude::*;
use rand::prelude::*;
use std::collections::{BinaryHeap, HashMap};

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    pos: IVec2,
    cost: u32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost.cmp(&self.cost) // Min-heap
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct RiverGenPlugin;

impl Plugin for RiverGenPlugin {
    fn build(&self, _app: &mut App) {}
}

#[allow(clippy::cast_possible_wrap)]
pub fn apply_rivers(map_data: &mut MapData, config: &TerrainConfig, seed: u32) {
    let mut rng = StdRng::seed_from_u64(u64::from(seed) + 500);
    let half_w = (map_data.width / 2) as i32;
    let half_h = (map_data.height / 2) as i32;

    for _ in 0..config.river_count {
        // 1. Find source
        let mut source = None;
        for _ in 0..100 {
            let x = rng.gen_range(-half_w..half_w);
            let z = rng.gen_range(-half_h..half_h);
            if let Some(tile) = map_data.get_tile(x, z) {
                if tile.elevation > config.river_start_elevation {
                    source = Some(IVec2::new(x, z));
                    break;
                }
            }
        }

        let Some(start_pos) = source else {
            continue;
        };

        // 2. Dijkstra
        let mut pq = BinaryHeap::new();
        let mut came_from = HashMap::new();
        let mut cost_so_far = HashMap::new();

        pq.push(Node {
            pos: start_pos,
            cost: 0,
        });
        cost_so_far.insert(start_pos, 0);

        let mut target_pos = None;

        while let Some(Node { pos, cost }) = pq.pop() {
            let Some(current_tile) = map_data.get_tile(pos.x, pos.y) else {
                continue;
            };

            // Check termination: Edge of map, low elevation (ocean/lake), or existing Water
            if pos.x <= -half_w
                || pos.x >= half_w - 1
                || pos.y <= -half_h
                || pos.y >= half_h - 1
                || current_tile.elevation < 0.2
                || current_tile.landscape_feature == LandscapeFeature::River
                || current_tile.landscape_feature == LandscapeFeature::Lake
            {
                target_pos = Some(pos);
                break;
            }

            for neighbor in [
                IVec2::new(pos.x + 1, pos.y),
                IVec2::new(pos.x - 1, pos.y),
                IVec2::new(pos.x, pos.y + 1),
                IVec2::new(pos.x, pos.y - 1),
            ] {
                if let Some(n_tile) = map_data.get_tile(neighbor.x, neighbor.y) {
                    let step_cost = if n_tile.elevation < current_tile.elevation {
                        1 // Downhill
                    } else if (n_tile.elevation - current_tile.elevation).abs() < 0.001 {
                        5 // Flat (plateau)
                    } else {
                        1000 // Uphill (prevent)
                    };

                    let new_cost = cost + step_cost;
                    let current_best = cost_so_far.get(&neighbor).copied().unwrap_or(u32::MAX);

                    if new_cost < current_best {
                        cost_so_far.insert(neighbor, new_cost);
                        came_from.insert(neighbor, pos);
                        pq.push(Node {
                            pos: neighbor,
                            cost: new_cost,
                        });
                    }
                }
            }
        }

        // 3. Backtrack and mark water
        if let Some(target) = target_pos {
            let mut path = Vec::new();
            let mut curr = target;
            while let Some(&prev) = came_from.get(&curr) {
                path.push(curr);
                curr = prev;
            }
            path.push(start_pos);
            path.reverse();

            let mut prev_elev = 1.0; // Start high for normalized elevation
            for pos in path {
                if let Some(tile) = map_data.get_tile_mut(pos.x, pos.y) {
                    tile.landscape_feature = LandscapeFeature::River;
                    // Carve: Lower elevation and ensure it never goes up (monotonically decreasing to sea)
                    tile.elevation = (tile.elevation - config.river_depth)
                        .min(prev_elev)
                        .max(0.0);
                    prev_elev = tile.elevation;
                }
            }
        }
    }
}

pub fn apply_mud_banks(map_data: &mut MapData) {
    let half_w = (map_data.width / 2) as i32;
    let half_h = (map_data.height / 2) as i32;
    let mut mud_to_add = Vec::new();

    for x in -half_w..half_w {
        for z in -half_h..half_h {
            if let Some(tile) = map_data.get_tile(x, z) {
                if tile.landscape_feature == LandscapeFeature::River {
                    for dx in -1..=1 {
                        for dz in -1..=1 {
                            if dx == 0 && dz == 0 {
                                continue;
                            }
                            let nx = x + dx;
                            let nz = z + dz;
                            if let Some(n_tile) = map_data.get_tile(nx, nz) {
                                if matches!(
                                    n_tile.terrain,
                                    TerrainType::Grass | TerrainType::Steppe | TerrainType::Stony
                                ) {
                                    mud_to_add.push(IVec2::new(nx, nz));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for pos in mud_to_add {
        let mut water_elevs = Vec::new();
        let mut land_elevs = Vec::new();

        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                if let Some(n_tile) = map_data.get_tile(pos.x + dx, pos.y + dz) {
                    if n_tile.landscape_feature == LandscapeFeature::River {
                        water_elevs.push(n_tile.elevation);
                    } else if matches!(
                        n_tile.terrain,
                        TerrainType::Grass | TerrainType::Steppe | TerrainType::Stony
                    ) {
                        land_elevs.push(n_tile.elevation);
                    }
                }
            }
        }

        if let Some(tile) = map_data.get_tile_mut(pos.x, pos.y) {
            tile.terrain = TerrainType::Swamp;
            if !water_elevs.is_empty() && !land_elevs.is_empty() {
                let avg_water: f32 = water_elevs.iter().sum::<f32>() / water_elevs.len() as f32;
                let avg_land: f32 = land_elevs.iter().sum::<f32>() / land_elevs.len() as f32;
                tile.elevation = f32::midpoint(avg_water, avg_land);
            }
        }
    }
}
