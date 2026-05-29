use crate::game_state::{FactionManager, FactionType};
use crate::map::data::OceanState;
use crate::map::{FactionMarker, FactionMarkerBundle, HexCoord, MapData, HEX_SIZE};
use bevy::prelude::*;
use rand::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct FactionGenerationPlugin;

impl Plugin for FactionGenerationPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn spawn_factions(
    commands: &mut Commands,
    map_data: &MapData,
    faction_manager: &FactionManager,
) {
    let mut faction_territories: HashMap<u32, Vec<HexCoord>> = HashMap::new();
    for (coord, tile) in &map_data.tiles {
        if let Some(f_id) = tile.faction_id {
            faction_territories.entry(f_id).or_default().push(*coord);
        }
    }
    for (f_id, coords) in faction_territories {
        if coords.is_empty() {
            continue;
        }
        let faction = faction_manager.factions.iter().find(|f| f.id == f_id);
        let f_type = faction
            .map(|f| f.faction_type)
            .unwrap_or(FactionType::Neutral);
        let f_name = faction
            .map(|f| f.name.clone())
            .unwrap_or_else(|| format!("Faction {}", f_id));
        let mut sum_q = 0;
        let mut sum_r = 0;
        for c in &coords {
            sum_q += c.q;
            sum_r += c.r;
        }
        let center = HexCoord::new(sum_q / coords.len() as i32, sum_r / coords.len() as i32);
        let candidates: Vec<_> = coords.iter().filter(|c| c.distance(center) <= 3).collect();
        let best_coord = if candidates.is_empty() {
            match coords.iter().min_by_key(|c| c.distance(center)) {
                Some(&c) => c,
                None => continue,
            }
        } else {
            **candidates
                .iter()
                .min_by_key(|c| {
                    let mut flatness = 0.0;
                    let h_center = map_data.get_tile(c.q, c.r).map_or(0.0, |t| t.elevation);
                    for n in c.neighbors() {
                        if let Some(nt) = map_data.get_tile(n.q, n.r) {
                            flatness += (nt.elevation - h_center).abs();
                        }
                    }
                    (flatness * 1000.0) as i32
                })
                .unwrap_or(&&center)
        };
        commands.spawn(FactionMarkerBundle {
            marker: FactionMarker {
                faction_type: f_type,
                hex_coord: best_coord,
            },
            name: Name::new(f_name),
            transform: Transform::from_translation(best_coord.to_world(HEX_SIZE)),
            visibility: Visibility::Visible,
            inherited_visibility: InheritedVisibility::default(),
        });
    }
}

pub fn auto_spawn_player_territory(map_data: &mut MapData, seed: u32) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(seed));
    let mut coastal_tiles = Vec::new();
    for (coord, tile) in &map_data.tiles {
        if tile.ocean_state == OceanState::Land {
            for neighbor_coord in coord.neighbors() {
                if let Some(neighbor) = map_data.tiles.get(&neighbor_coord) {
                    if neighbor.ocean_state == OceanState::Ocean {
                        coastal_tiles.push(*coord);
                        break;
                    }
                }
            }
        }
    }
    if let Some(start_coord) = coastal_tiles.choose(&mut rng) {
        let mut queue = VecDeque::new();
        queue.push_back(*start_coord);
        let mut territory = HashSet::new();
        territory.insert(*start_coord);
        let target_size = rng.gen_range(15..=25);
        while let Some(curr) = queue.pop_front() {
            if territory.len() >= target_size {
                break;
            }
            let mut neighbors: Vec<_> = curr.neighbors().into_iter().collect();
            neighbors.shuffle(&mut rng);
            for n in neighbors {
                if territory.len() >= target_size {
                    break;
                }
                if let Some(tile) = map_data.tiles.get(&n) {
                    if tile.ocean_state == OceanState::Land && !territory.contains(&n) {
                        territory.insert(n);
                        queue.push_back(n);
                    }
                }
            }
        }
        for coord in territory {
            if let Some(tile) = map_data.tiles.get_mut(&coord) {
                tile.faction_id = Some(1);
            }
        }
        debug!("FACTION: Auto-spawned Player Start at {:?}", start_coord);
    }
}

pub fn auto_spawn_npc_territory(map_data: &mut MapData, faction_id: u32, seed: u32) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(seed));
    let free_land: Vec<_> = map_data
        .tiles
        .iter()
        .filter(|(_, t)| t.ocean_state == OceanState::Land && t.faction_id.is_none())
        .map(|(c, _)| *c)
        .collect();
    if let Some(start_coord) = free_land.choose(&mut rng) {
        let mut queue = VecDeque::new();
        queue.push_back(*start_coord);
        let mut territory = HashSet::new();
        territory.insert(*start_coord);
        let target_size = rng.gen_range(20..=30);
        while let Some(curr) = queue.pop_front() {
            if territory.len() >= target_size {
                break;
            }
            let mut neighbors: Vec<_> = curr.neighbors().into_iter().collect();
            neighbors.shuffle(&mut rng);
            for n in neighbors {
                if territory.len() >= target_size {
                    break;
                }
                if let Some(tile) = map_data.tiles.get(&n) {
                    let mut can_paint =
                        tile.ocean_state == OceanState::Land && tile.faction_id.is_none();
                    if can_paint {
                        for nn in n.neighbors() {
                            if let Some(nt) = map_data.tiles.get(&nn) {
                                if let Some(fid) = nt.faction_id {
                                    if fid != faction_id {
                                        can_paint = false;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if can_paint && !territory.contains(&n) {
                        territory.insert(n);
                        queue.push_back(n);
                    }
                }
            }
        }
        for coord in territory {
            if let Some(tile) = map_data.tiles.get_mut(&coord) {
                tile.faction_id = Some(faction_id);
            }
        }
    }
}
