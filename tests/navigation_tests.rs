use bevy::prelude::*;
use savage_fantasy::map::{
    navigation::{compute_astar_path, world_to_grid, AGENT_HEIGHT, COST_BASE, COST_BLOCKER},
    MapData,
};
use std::collections::HashMap;

/// Хелпер для создания карты из ASCII. Окружает карту блокерами для изоляции теста.
fn parse_ascii_map(_lines: Vec<&str>) -> (HashMap<IVec2, u8>, Vec3, Vec3, MapData) {
    /*
    let mut grid = HashMap::new();
    let mut start = Vec3::ZERO;
    let mut target = Vec3::ZERO;

    // Определяем размеры
    let height = lines.len() as i32;
    let width = lines.iter().map(|l| l.len()).max().unwrap_or(0) as i32;

    let mut map_data = MapData {
        width: width as u32,
        height: height as u32,
        tiles: vec![Default::default(); (width * height) as usize],
    };

    // Добавляем границы, чтобы путь не шел "в обход" через бесконечность
    for x in -1..=(width) {
        grid.insert(IVec2::new(x, -1), COST_BLOCKER);
        grid.insert(IVec2::new(x, height), COST_BLOCKER);
    }
    for z in -1..=(height) {
        grid.insert(IVec2::new(-1, z), COST_BLOCKER);
        grid.insert(IVec2::new(width, z), COST_BLOCKER);
    }

    for (z, line) in lines.iter().enumerate() {
        for (x, char) in line.chars().enumerate() {
            let pos = IVec2::new(x as i32, z as i32);
            let world_pos = Vec3::new(x as f32, AGENT_HEIGHT, z as f32);

            match char {
                'S' => {
                    start = world_pos;
                    grid.insert(pos, COST_BASE);
                }
                'T' => {
                    target = world_pos;
                    grid.insert(pos, COST_BASE);
                }
                '#' => {
                    grid.insert(pos, COST_BLOCKER);
                }
                '.' => {
                    grid.insert(pos, COST_BASE);
                }
                _ => {}
            }
        }
    }

    (grid, start, target, map_data)
    */
    unimplemented!()
}

#[test]
#[ignore]
fn test_straight_path() {
    /*
    let (grid, start, target, map) = parse_ascii_map(vec!["S..T"]);
    let path = compute_astar_path(&grid, start, target, 0.1, &map).expect("Path found");
    assert!(path.len() >= 2);
    assert_eq!(world_to_grid(*path.last().unwrap()), world_to_grid(target));
    */
}

#[test]
#[ignore]
fn test_u_obstacle() {
    /*
    let (grid, start, target, map) = parse_ascii_map(vec!["S....", "####.", "T...."]);

    let path = compute_astar_path(&grid, start, target, 0.1, &map).expect("Path found");
    // (0,0)->(4,0)->(4,1)->(4,2)->(0,2) = 11 точек
    assert_eq!(path.len(), 11, "Must go around the wall");
    assert_eq!(world_to_grid(*path.last().unwrap()), world_to_grid(target));
    */
}

#[test]
#[ignore]
fn test_blocked_target_with_radius() {
    /*
    let (grid, start, target, map) = parse_ascii_map(vec!["S..", "...", "..T"]);

    let mut grid = grid;
    let target_grid = world_to_grid(target);
    grid.insert(target_grid, COST_BLOCKER);

    // С радиусом 0.1 путь невозможен (T - стена)
    let path_no_radius = compute_astar_path(&grid, start, target, 0.1, &map);
    assert!(path_no_radius.is_none());

    // С радиусом 1.1 путь возможен в соседнюю ячейку (например, 1,2 или 2,1)
    let path_with_radius =
        compute_astar_path(&grid, start, target, 1.1, &map).expect("Path found with radius");
    let last_point = *path_with_radius.last().unwrap();

    assert!(last_point.distance(target) <= 1.101);
    assert_ne!(world_to_grid(last_point), target_grid);
    */
}

#[test]
#[ignore]
fn test_unreachable() {
    /*
    let (grid, start, target, map) = parse_ascii_map(vec!["###", "#S#", "###", "...", ".T."]);

    let path = compute_astar_path(&grid, start, target, 0.1, &map);
    assert!(path.is_none(), "Should be unreachable");
    */
}
