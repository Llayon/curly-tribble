// src/map/river_gen.rs
use bevy::prelude::*;
use rand::prelude::*;
use std::collections::{BinaryHeap, HashMap};
use crate::map::zoning::{MapData, TerrainType};
use crate::map::terrain_gen::TerrainConfig;

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

pub fn apply_rivers(map_data: &mut MapData, config: &TerrainConfig, seed: u32) {
    let mut rng = StdRng::seed_from_u64(seed as u64 + 500);
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

        let Some(start_pos) = source else { continue; };

        // 2. Dijkstra
        let mut pq = BinaryHeap::new();
        let mut came_from = HashMap::new();
        let mut cost_so_far = HashMap::new();

        pq.push(Node { pos: start_pos, cost: 0 });
        cost_so_far.insert(start_pos, 0);

        let mut target_pos = None;

        while let Some(Node { pos, cost }) = pq.pop() {
            let current_tile = map_data.get_tile(pos.x, pos.y).unwrap();
            
            // Check termination: Edge of map, low elevation (ocean/lake), or existing Water
            if pos.x <= -half_w || pos.x >= half_w - 1 || pos.y <= -half_h || pos.y >= half_h - 1 ||
               current_tile.elevation < 0.2 || current_tile.terrain == TerrainType::Water {
                target_pos = Some(pos);
                break;
            }

            for neighbor in [
                IVec2::new(pos.x + 1, pos.y), IVec2::new(pos.x - 1, pos.y),
                IVec2::new(pos.x, pos.y + 1), IVec2::new(pos.x, pos.y - 1),
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
                    if !cost_so_far.contains_key(&neighbor) || new_cost < *cost_so_far.get(&neighbor).unwrap() {
                        cost_so_far.insert(neighbor, new_cost);
                        came_from.insert(neighbor, pos);
                        pq.push(Node { pos: neighbor, cost: new_cost });
                    }
                }
            }
        }

        // 3. Backtrack and mark water
        if let Some(mut curr) = target_pos {
            while let Some(&prev) = came_from.get(&curr) {
                if let Some(tile) = map_data.get_tile_mut(curr.x, curr.y) {
                    tile.terrain = TerrainType::Water;
                }
                curr = prev;
            }
            // Mark the source itself if needed (the backtrack usually stops at the source's child)
            if let Some(tile) = map_data.get_tile_mut(start_pos.x, start_pos.y) {
                tile.terrain = TerrainType::Water;
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
                if tile.terrain == TerrainType::Water {
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
                                    TerrainType::Grass | TerrainType::Sand | TerrainType::Stone
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
        if let Some(tile) = map_data.get_tile_mut(pos.x, pos.y) {
            tile.terrain = TerrainType::Mud;
        }
    }
}
