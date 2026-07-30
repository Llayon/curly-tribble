use super::types::{grid_to_world, world_to_grid, COST_BASE, COST_BLOCKER};
use bevy::prelude::*;
use std::collections::HashMap;

pub struct NavigationAlgoPlugin;
impl Plugin for NavigationAlgoPlugin {
    fn build(&self, _app: &mut App) {}
}

#[must_use]
pub fn compute_astar_path<S: std::hash::BuildHasher>(
    grid: &HashMap<IVec2, u8, S>,
    start_pos: Vec3,
    target_pos: Vec3,
    radius: f32,
    map: &crate::map::MapData,
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
            let hex = crate::map::HexCoord::new(p.x, p.y);
            hex.neighbors()
                .into_iter()
                .filter_map(|n_hex| {
                    let n = IVec2::new(n_hex.q, n_hex.r);
                    if n.x.abs_diff(start_cell.x) > search_limit
                        || n.y.abs_diff(start_cell.y) > search_limit
                    {
                        return None;
                    }

                    let cost = *grid.get(&n).unwrap_or(&COST_BASE);
                    if cost == COST_BLOCKER {
                        None
                    } else {
                        Some((n, i32::from(cost)))
                    }
                })
                .collect::<Vec<_>>()
        },
        |&p| {
            let hex_p = crate::map::HexCoord::new(p.x, p.y);
            let hex_target = crate::map::HexCoord::new(target_cell.x, target_cell.y);
            hex_p.distance(hex_target)
        },
        |&p| {
            if p == target_cell {
                return true;
            }
            let world_p = grid_to_world(p, map);
            let p_2d = Vec2::new(world_p.x, world_p.z);
            let t_2d = Vec2::new(target_pos.x, target_pos.z);
            p_2d.distance(t_2d) <= radius + 0.001
        },
    );

    result.map(|(path, _cost)| {
        path.into_iter()
            .map(|p| grid_to_world(p, map))
            .collect::<Vec<_>>()
    })
}
