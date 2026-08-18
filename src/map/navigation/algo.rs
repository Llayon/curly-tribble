// src/map/navigation/algo.rs
//! A* pathfinding over the authoritative `SurfaceGameplayMap` with a
//! dynamic-only overlay grid (`NavObstacle` costs). Never reads legacy
//! `MapData` elevation or the old static navigation grid.

use super::types::{world_to_grid, COST_BLOCKER};
use crate::map::surface_gameplay::types::SurfaceGameplayMap;
use crate::map::surface_gameplay::world::gameplay_center_world_pos;
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct NavigationAlgoPlugin;

impl Plugin for NavigationAlgoPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Effective movement cost of one cell: dynamic overlay wins, otherwise the
/// gameplay cell (walkable → terrain cost, unwalkable → blocker).
fn effective_cost<S: std::hash::BuildHasher>(
    dynamic: &HashMap<IVec2, u8, S>,
    gameplay: &SurfaceGameplayMap,
    cell: IVec2,
) -> u8 {
    if let Some(cost) = dynamic.get(&cell) {
        return *cost;
    }
    let hex = HexCoord::new(cell.x, cell.y);
    gameplay
        .cells
        .get(&hex)
        .filter(|c| c.walkable)
        .map_or(COST_BLOCKER, |c| c.movement_cost)
}

#[must_use]
pub fn compute_astar_path<S: std::hash::BuildHasher>(
    gameplay: &SurfaceGameplayMap,
    dynamic: &HashMap<IVec2, u8, S>,
    start_pos: Vec3,
    target_pos: Vec3,
    radius: f32,
) -> Option<Vec<Vec3>> {
    use pathfinding::prelude::astar;

    let start_cell = world_to_grid(start_pos);
    let target_cell = world_to_grid(target_pos);

    if start_cell == target_cell || start_pos.distance(target_pos) <= radius + 0.001 {
        return Some(vec![start_pos]);
    }

    let search_limit = 100;

    let result = astar(
        &start_cell,
        |&p| {
            let hex = HexCoord::new(p.x, p.y);
            hex.neighbors()
                .into_iter()
                .filter_map(|n_hex| {
                    let n = IVec2::new(n_hex.q, n_hex.r);
                    if n.x.abs_diff(start_cell.x) > search_limit
                        || n.y.abs_diff(start_cell.y) > search_limit
                    {
                        return None;
                    }

                    let cost = effective_cost(dynamic, gameplay, n);
                    if cost == COST_BLOCKER {
                        None
                    } else {
                        Some((n, i32::from(cost)))
                    }
                })
                .collect::<Vec<_>>()
        },
        |&p| {
            let hex_p = HexCoord::new(p.x, p.y);
            let hex_target = HexCoord::new(target_cell.x, target_cell.y);
            hex_p.distance(hex_target)
        },
        |&p| {
            if p == target_cell {
                return true;
            }
            let world_p = gameplay_center_world_pos(HexCoord::new(p.x, p.y), gameplay);
            let p_2d = Vec2::new(world_p.x, world_p.z);
            let t_2d = Vec2::new(target_pos.x, target_pos.z);
            p_2d.distance(t_2d) <= radius + 0.001
        },
    );

    result.map(|(path, _cost)| {
        path.into_iter()
            .map(|p| gameplay_center_world_pos(HexCoord::new(p.x, p.y), gameplay))
            .collect::<Vec<_>>()
    })
}
