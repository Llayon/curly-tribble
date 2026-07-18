use crate::map::data::OceanState;
use crate::map::{FactionMarker, MapData, HEX_SIZE};
use bevy::prelude::*;
use std::collections::{HashSet, VecDeque};

pub struct ValidationPlugin;

impl Plugin for ValidationPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn run_map_validation(map_data: &mut MapData) {
    map_data.validation_errors.clear();

    let mut total_land = 0;
    let mut total_ocean = 0;
    let mut first_land = None;
    let mut first_border_ocean = None;

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let half_w = (map_data.width / 2) as i32;
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let half_h = (map_data.height / 2) as i32;

    for (coord, tile) in &map_data.tiles {
        if tile.ocean_state == OceanState::Ocean {
            total_ocean += 1;
            if first_border_ocean.is_none()
                && (coord.q <= -half_w + 1
                    || coord.q >= half_w - 2
                    || coord.r <= -half_h + 1
                    || coord.r >= half_h - 2)
            {
                first_border_ocean = Some(*coord);
            }
        } else {
            total_land += 1;
            if first_land.is_none() {
                first_land = Some(*coord);
            }
        }
    }

    if total_land == 0 {
        map_data
            .validation_errors
            .push("Остров должен содержать хотя бы один гекс суши.".to_string());
        return;
    }

    if !region_is_connected(map_data, first_land, total_land, OceanState::Land) {
        map_data.validation_errors.push(
            "Остров должен быть единым континентом. Разорванные участки суши не допускаются."
                .to_string(),
        );
    }
    if !region_is_connected(map_data, first_border_ocean, total_ocean, OceanState::Ocean) {
        map_data.validation_errors.push("Внутри континента найдены изолированные озера. Океан должен соединяться с краем карты.".to_string());
    }

    clear_isolated_faction_territories(map_data);

    // 4. Проверка правила '1-Hex Gap'
    for (coord, tile) in &map_data.tiles {
        if let Some(f1) = tile.faction_id {
            for n_coord in coord.neighbors() {
                if let Some(n_tile) = map_data.tiles.get(&n_coord) {
                    if let Some(f2) = n_tile.faction_id {
                        if f1 != f2 {
                            map_data.validation_errors.push(format!("Нарушено правило 1 гекса между фракциями {f1} и {f2} у координат {coord:?}."));
                        }
                    }
                }
            }
        }
    }
}

fn clear_isolated_faction_territories(map_data: &mut MapData) {
    let faction_ids: HashSet<_> = map_data
        .tiles
        .values()
        .filter_map(|tile| tile.faction_id)
        .collect();
    for faction_id in faction_ids {
        let mut unvisited: HashSet<_> = map_data
            .tiles
            .iter()
            .filter(|(_, tile)| tile.faction_id == Some(faction_id))
            .map(|(coord, _)| *coord)
            .collect();
        let mut components = Vec::new();
        while let Some(&start) = unvisited.iter().next() {
            let mut component = vec![start];
            let mut queue = VecDeque::from([start]);
            unvisited.remove(&start);
            while let Some(current) = queue.pop_front() {
                for neighbor in current.neighbors() {
                    if unvisited.remove(&neighbor) {
                        component.push(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
            components.push(component);
        }
        if components.len() > 1 {
            components.sort_by_key(Vec::len);
            let Some(_largest) = components.pop() else {
                continue;
            };
            for coord in components.into_iter().flatten() {
                if let Some(tile) = map_data.tiles.get_mut(&coord) {
                    tile.faction_id = None;
                }
            }
            debug!("VALIDATION: Autocleaned isolated fragments for faction {faction_id}.");
        }
    }
}

fn region_is_connected(
    map_data: &MapData,
    start: Option<crate::map::HexCoord>,
    expected_count: usize,
    target_state: OceanState,
) -> bool {
    let Some(start) = start else {
        return expected_count == 0;
    };
    let mut visited = HashSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(current) = queue.pop_front() {
        for neighbor in current.neighbors() {
            if map_data
                .get_tile(neighbor.q, neighbor.r)
                .is_some_and(|tile| tile.ocean_state == target_state)
                && visited.insert(neighbor)
            {
                queue.push_back(neighbor);
            }
        }
    }
    visited.len() == expected_count
}

pub fn validate_faction_placements(
    map_data: Res<MapData>,
    mut q_factions: Query<(&mut FactionMarker, &mut Transform), With<FactionMarker>>,
) {
    if map_data.is_changed() {
        for (mut marker, mut transform) in &mut q_factions {
            let coord = marker.hex_coord;
            let is_invalid = map_data
                .get_tile(coord.q, coord.r)
                .is_none_or(|t| t.ocean_state == OceanState::Ocean);

            if is_invalid {
                let mut visited = HashSet::new();
                let mut queue = VecDeque::new();
                queue.push_back(coord);
                visited.insert(coord);

                let mut found_coord = None;
                while let Some(curr) = queue.pop_front() {
                    if let Some(tile) = map_data.get_tile(curr.q, curr.r) {
                        if tile.ocean_state == OceanState::Land {
                            found_coord = Some(curr);
                            break;
                        }
                    }
                    if visited.len() > 400 {
                        break;
                    }
                    for neighbor in curr.neighbors() {
                        if !visited.contains(&neighbor) {
                            visited.insert(neighbor);
                            queue.push_back(neighbor);
                        }
                    }
                }

                if let Some(new_coord) = found_coord {
                    marker.hex_coord = new_coord;
                    transform.translation = new_coord.to_world(HEX_SIZE);
                    debug!("FACTION: Relocated faction to {:?}", new_coord);
                }
            } else {
                transform.translation = coord.to_world(HEX_SIZE);
            }
        }
    }
}
