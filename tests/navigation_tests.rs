// tests/navigation_tests.rs
//! End-to-end A* over the authoritative `SurfaceGameplayMap` with a
//! dynamic-only `NavObstacle` overlay. These tests deliberately avoid
//! `MapData` entirely: costs come from gameplay cells, obstacles from the
//! dynamic grid, and world positions from solved center heights.

use bevy::prelude::*;
use savage_fantasy::map::navigation::{compute_astar_path, world_to_grid, COST_BASE, COST_BLOCKER};
use savage_fantasy::map::surface_gameplay::types::{SurfaceGameplayCell, SurfaceGameplayMap};
use savage_fantasy::map::HexCoord;
use std::collections::HashMap;

fn walkable_cell() -> SurfaceGameplayCell {
    SurfaceGameplayCell {
        walkable: true,
        movement_cost: COST_BASE,
        buildable: true,
        center_xz: Vec2::ZERO,
        center_height: 0.0,
        relief: 0.0,
    }
}

/// Builds a flat gameplay layer from an ASCII map. `S`/`T`/`.` are walkable
/// cells, `#` are absent from the layer (unwalkable) and additionally added
/// as dynamic blockers for the test's isolation border.
fn parse_ascii_gameplay(lines: Vec<&str>) -> (SurfaceGameplayMap, HashMap<IVec2, u8>, Vec3, Vec3) {
    let mut gameplay = SurfaceGameplayMap::default();
    let mut grid = HashMap::new();
    let mut start = Vec3::ZERO;
    let mut target = Vec3::ZERO;

    let height = lines.len() as i32;
    let width = lines.iter().map(|l| l.len()).max().unwrap_or(0) as i32;

    for x in -1..=width {
        grid.insert(IVec2::new(x, -1), COST_BLOCKER);
        grid.insert(IVec2::new(x, height), COST_BLOCKER);
    }
    for z in -1..=height {
        grid.insert(IVec2::new(-1, z), COST_BLOCKER);
        grid.insert(IVec2::new(width, z), COST_BLOCKER);
    }

    for (z, line) in lines.iter().enumerate() {
        for (x, char) in line.chars().enumerate() {
            let cell = IVec2::new(x as i32, z as i32);
            let hex = HexCoord::new(cell.x, cell.y);
            let world_pos = hex.to_world(savage_fantasy::map::HEX_SIZE);
            match char {
                'S' => {
                    start = world_pos;
                    gameplay.cells.insert(hex, walkable_cell());
                }
                'T' => {
                    target = world_pos;
                    gameplay.cells.insert(hex, walkable_cell());
                }
                '.' => {
                    gameplay.cells.insert(hex, walkable_cell());
                }
                '#' => {
                    grid.insert(cell, COST_BLOCKER);
                }
                _ => {}
            }
        }
    }

    (gameplay, grid, start, target)
}

#[test]
fn test_straight_path() {
    let (gameplay, grid, start, target) = parse_ascii_gameplay(vec!["S..T"]);
    let path = compute_astar_path(&gameplay, &grid, start, target, 0.1).expect("Path found");
    assert!(path.len() >= 2);
    assert_eq!(
        world_to_grid(*path.last().unwrap()),
        world_to_grid(target),
        "path must end on the target cell"
    );
}

#[test]
fn test_u_obstacle() {
    let (gameplay, grid, start, target) = parse_ascii_gameplay(vec!["S....", "####.", "T...."]);
    let path = compute_astar_path(&gameplay, &grid, start, target, 0.1).expect("Path found");
    assert_eq!(
        path.len(),
        10,
        "Must go around the wall: (0,0)-(4,0)-(4,1)-(3,2)-(0,2)"
    );
    assert_eq!(world_to_grid(*path.last().unwrap()), world_to_grid(target));
}

#[test]
fn test_blocked_target_with_radius() {
    let (gameplay, mut grid, start, target) = parse_ascii_gameplay(vec!["S..", "...", "..T"]);
    let target_grid = world_to_grid(target);
    grid.insert(target_grid, COST_BLOCKER);

    let path_no_radius = compute_astar_path(&gameplay, &grid, start, target, 0.1);
    assert!(
        path_no_radius.is_none(),
        "blocked target must be unreachable"
    );

    // With radius 2.0 the path can stop in a neighbor cell (hex neighbor
    // centers are sqrt(3) ~ 1.732 apart with HEX_SIZE = 1.0).
    let path_with_radius =
        compute_astar_path(&gameplay, &grid, start, target, 2.0).expect("Path found with radius");
    let last_point = *path_with_radius.last().unwrap();
    assert!(last_point.distance(target) <= 2.001);
    assert_ne!(world_to_grid(last_point), target_grid);
}

#[test]
fn test_unreachable() {
    let (gameplay, grid, start, target) =
        parse_ascii_gameplay(vec!["###", "#S#", "###", "...", ".T."]);
    let path = compute_astar_path(&gameplay, &grid, start, target, 0.1);
    assert!(path.is_none(), "Should be unreachable");
}
