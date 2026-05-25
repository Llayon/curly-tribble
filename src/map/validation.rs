use crate::game_state::EditorPhase;
use crate::map::GenerateMapEvent;
use crate::map::{FactionMarker, ForestType, LandscapeFeature, MapData, TerrainType, HEX_SIZE};
use bevy::prelude::*;
use std::collections::{HashSet, VecDeque};

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
        if tile.is_ocean {
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

    // 1. Проверка на разорванность (Единый континент)
    if let Some(start) = first_land {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited.insert(start);

        while let Some(curr) = queue.pop_front() {
            for n in curr.neighbors() {
                if let Some(t) = map_data.tiles.get(&n) {
                    if !t.is_ocean && !visited.contains(&n) {
                        visited.insert(n);
                        queue.push_back(n);
                    }
                }
            }
        }

        if visited.len() < total_land {
            map_data.validation_errors.push(
                "Остров должен быть единым континентом. Разорванные участки суши не допускаются."
                    .to_string(),
            );
        }
    }

    // 2. Проверка на изолированные озера из океана
    if let Some(start) = first_border_ocean {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited.insert(start);

        while let Some(curr) = queue.pop_front() {
            for n in curr.neighbors() {
                if let Some(t) = map_data.tiles.get(&n) {
                    if t.is_ocean && !visited.contains(&n) {
                        visited.insert(n);
                        queue.push_back(n);
                    }
                }
            }
        }

        if visited.len() < total_ocean {
            map_data.validation_errors.push("Внутри континента найдены изолированные озера. Океан должен соединяться с краем карты.".to_string());
        }
    }

    // 3. Проверка непрерывности территорий фракций (Continuous Territory)
    let mut factions_on_map = HashSet::new();
    for tile in map_data.tiles.values() {
        if let Some(f_id) = tile.faction_id {
            factions_on_map.insert(f_id);
        }
    }

    for f_id in factions_on_map {
        let faction_tiles: Vec<_> = map_data
            .tiles
            .iter()
            .filter(|(_, t)| t.faction_id == Some(f_id))
            .map(|(c, _)| *c)
            .collect();

        if faction_tiles.is_empty() {
            continue;
        }

        let mut components = Vec::new();
        let mut unvisited: HashSet<_> = faction_tiles.iter().copied().collect();

        while !unvisited.is_empty() {
            let Some(&start) = unvisited.iter().next() else {
                break;
            };
            let mut component = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(start);
            unvisited.remove(&start);
            component.push(start);

            while let Some(curr) = queue.pop_front() {
                for n in curr.neighbors() {
                    if unvisited.contains(&n) {
                        unvisited.remove(&n);
                        component.push(n);
                        queue.push_back(n);
                    }
                }
            }
            components.push(component);
        }

        if components.len() > 1 {
            components.sort_by_key(|c| c.len());
            let Some(_largest) = components.pop() else {
                continue;
            };
            for fragment in components {
                for coord in fragment {
                    if let Some(tile) = map_data.tiles.get_mut(&coord) {
                        tile.faction_id = None;
                    }
                }
            }
            debug!(
                "VALIDATION: Autocleaned isolated fragments for faction {}.",
                f_id
            );
        }
    }

    // 4. Проверка правила '1-Hex Gap'
    for (coord, tile) in &map_data.tiles {
        if let Some(f1) = tile.faction_id {
            for n_coord in coord.neighbors() {
                if let Some(n_tile) = map_data.tiles.get(&n_coord) {
                    if let Some(f2) = n_tile.faction_id {
                        if f1 != f2 {
                            map_data.validation_errors.push(format!("Нарушено правило 1 гекса между фракциями {} и {} у координат {:?}.", f1, f2, coord));
                        }
                    }
                }
            }
        }
    }
}

pub fn validate_faction_placements(
    map_data: Res<MapData>,
    mut q_factions: Query<(&mut FactionMarker, &mut Transform)>,
) {
    if map_data.is_changed() {
        for (mut marker, mut transform) in &mut q_factions {
            let coord = marker.hex_coord;
            let is_invalid = map_data
                .get_tile(coord.q, coord.r)
                .map_or(true, |t| t.is_ocean);

            if is_invalid {
                let mut visited = HashSet::new();
                let mut queue = VecDeque::new();
                queue.push_back(coord);
                visited.insert(coord);

                let mut found_coord = None;
                while let Some(curr) = queue.pop_front() {
                    if let Some(tile) = map_data.get_tile(curr.q, curr.r) {
                        if !tile.is_ocean {
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

pub fn rebuild_map_on_phase_change(
    mut ev_gen: MessageWriter<GenerateMapEvent>,
    phase: Res<State<EditorPhase>>,
    map_data: Res<MapData>,
    q_pois: Query<&crate::map::zoning::PointOfInterest>,
    q_camps: Query<&crate::map::zoning::EnemyCamp>,
    q_deposits: Query<&crate::map::ResourceDeposit>,
) {
    debug!(
        "MAP_GEN: Phase changed to {:?}. Checking for auto-fill...",
        *phase.get()
    );

    let current_phase = *phase.get();
    let needs_auto_fill = match current_phase {
        EditorPhase::Landscape => !map_data
            .tiles
            .values()
            .any(|t| t.landscape_feature != LandscapeFeature::None),
        EditorPhase::Sediments => !map_data
            .tiles
            .values()
            .any(|t| t.terrain != TerrainType::Grass || t.forest_type != ForestType::None),
        EditorPhase::NPCs => q_pois.is_empty() && q_camps.is_empty(),
        EditorPhase::Plants => q_deposits.is_empty(),
        _ => false,
    };

    ev_gen.write(GenerateMapEvent {
        force_reset: false,
        auto_fill_phase: if needs_auto_fill {
            Some(current_phase)
        } else {
            None
        },
    });
}

pub fn validate_bio_habitats(
    map_data: Res<MapData>,
    mut q_deposits: Query<&mut crate::map::ResourceDeposit>,
) {
    if !map_data.is_changed() {
        return;
    }

    for mut deposit in &mut q_deposits {
        let coord = deposit.hex_coord;
        let Some(tile) = map_data.get_tile(coord.q, coord.r) else {
            deposit.habitat_valid = false;
            continue;
        };

        match deposit.deposit_type {
            crate::map::DepositType::Deer | crate::map::DepositType::Boar => {
                let mut has_forest = tile.forest_type != ForestType::None;
                if !has_forest {
                    for neighbor in coord.neighbors() {
                        if let Some(nt) = map_data.get_tile(neighbor.q, neighbor.r) {
                            if nt.forest_type != ForestType::None {
                                has_forest = true;
                                break;
                            }
                        }
                    }
                }
                deposit.habitat_valid = has_forest && !tile.is_ocean;
            }
            crate::map::DepositType::OceanFish => {
                deposit.habitat_valid = tile.is_ocean;
            }
            crate::map::DepositType::Rabbit
            | crate::map::DepositType::WildFlax
            | crate::map::DepositType::Raspberries
            | crate::map::DepositType::Pumpkin
            | crate::map::DepositType::WildWheat => {
                deposit.habitat_valid = tile.terrain.traits().allow_plants && !tile.is_ocean;
            }
        }
    }
}
