use super::types::*;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct NavigationAlgoPlugin;
impl Plugin for NavigationAlgoPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn compute_astar_path(
    grid: &HashMap<IVec2, u8>,
    start_pos: Vec3,
    target_pos: Vec3,
    radius: f32,
) -> Option<Vec<Vec3>> {
    use pathfinding::prelude::astar;

    let start_cell = world_to_grid(start_pos);
    let target_cell = world_to_grid(target_pos);

    if start_pos.distance(target_pos) <= radius + 0.001 {
        return Some(vec![start_pos]);
    }

    let search_limit = 50;

    let result = astar(
        &start_cell,
        |&p| {
            let neighbors = [
                IVec2::new(p.x + 1, p.y),
                IVec2::new(p.x - 1, p.y),
                IVec2::new(p.x, p.y + 1),
                IVec2::new(p.x, p.y - 1),
            ];

            neighbors
                .into_iter()
                .filter_map(|n| {
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
            #[allow(clippy::cast_possible_wrap)]
            {
                (p.x.abs_diff(target_cell.x) + p.y.abs_diff(target_cell.y)) as i32
            }
        },
        |&p| {
            let world_p = grid_to_world(p);
            world_p.distance(target_pos) <= radius + 0.001
        },
    );

    result.map(|(path, _cost)| path.into_iter().map(grid_to_world).collect::<Vec<_>>())
}
