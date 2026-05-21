use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

pub const MAX_HEIGHT: f32 = 12.0;
pub const HEX_SIZE: f32 = 1.0;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ZoneType {
    Empty,
    FoodStockpile,
    Housing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum TerrainType {
    #[default]
    Grass,
    Mud,
    Water,
    Sand,
    Stone,
    CaveFloor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileLayer {
    #[default]
    Ground,
    Roof,
}

#[derive(Clone, Copy, Debug, Default, Reflect)]
pub struct TileData {
    pub terrain: TerrainType,
    pub elevation: f32,
    pub humidity: f32,
    pub temperature: f32,
    pub roofed: bool,
    pub is_ocean: bool,
    pub faction_id: Option<u32>,
}

#[derive(Resource, Default, Clone)]
pub struct MapData {
    pub width: u32,
    pub height: u32,
    pub tiles: HashMap<HexCoord, TileData>,
    pub validation_errors: Vec<String>,
}

impl MapData {
    #[must_use]
    pub fn get_tile(&self, q: i32, r: i32) -> Option<&TileData> {
        self.tiles.get(&HexCoord::new(q, r))
    }

    pub fn get_tile_mut(&mut self, q: i32, r: i32) -> Option<&mut TileData> {
        self.tiles.get_mut(&HexCoord::new(q, r))
    }

    #[must_use]
    pub fn get_hex_height(&self, q: i32, r: i32) -> f32 {
        self.get_tile(q, r).map_or(0.0, |t| t.elevation * MAX_HEIGHT)
    }

    #[must_use]
    pub fn is_too_steep(&self, q: i32, r: i32) -> bool {
        let current_elev = self.get_tile(q, r).map_or(0.0, |t| t.elevation);
        let coord = HexCoord::new(q, r);
        for neighbor_coord in coord.neighbors() {
            if let Some(neighbor) = self.tiles.get(&neighbor_coord) {
                if (neighbor.elevation - current_elev).abs() > 0.3 {
                    return true;
                }
            }
        }
        false
    }

    pub fn run_validation(&mut self) {
        self.validation_errors.clear();
        
        let mut total_land = 0;
        let mut total_ocean = 0;
        let mut first_land = None;
        let mut first_border_ocean = None;
        
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let half_w = (self.width / 2) as i32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let half_h = (self.height / 2) as i32;
        
        for (coord, tile) in &self.tiles {
            if tile.is_ocean {
                total_ocean += 1;
                if first_border_ocean.is_none() && (coord.q <= -half_w + 1 || coord.q >= half_w - 2 || coord.r <= -half_h + 1 || coord.r >= half_h - 2) {
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
            self.validation_errors.push("Остров должен содержать хотя бы один гекс суши.".to_string());
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
                    if let Some(t) = self.tiles.get(&n) {
                        if !t.is_ocean && !visited.contains(&n) {
                            visited.insert(n);
                            queue.push_back(n);
                        }
                    }
                }
            }
            
            if visited.len() < total_land {
                self.validation_errors.push("Остров должен быть единым континентом. Разорванные участки суши не допускаются.".to_string());
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
                    if let Some(t) = self.tiles.get(&n) {
                        if t.is_ocean && !visited.contains(&n) {
                            visited.insert(n);
                            queue.push_back(n);
                        }
                    }
                }
            }
            
            if visited.len() < total_ocean {
                self.validation_errors.push("Внутри континента найдены изолированные озера. Океан должен соединяться с краем карты.".to_string());
            }
        }

        // 3. Проверка непрерывности территорий фракций (Continuous Territory)
        let mut factions_on_map = HashSet::new();
        for tile in self.tiles.values() {
            if let Some(f_id) = tile.faction_id {
                factions_on_map.insert(f_id);
            }
        }

        for f_id in factions_on_map {
            let faction_tiles: Vec<_> = self.tiles.iter()
                .filter(|(_, t)| t.faction_id == Some(f_id))
                .map(|(c, _)| *c)
                .collect();

            if faction_tiles.is_empty() { continue; }

            let mut components = Vec::new();
            let mut unvisited: HashSet<_> = faction_tiles.iter().copied().collect();

            while !unvisited.is_empty() {
                let start = *unvisited.iter().next().unwrap();
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
                // Если территория разорвана: оставляем только самый крупный кусок для любой фракции
                components.sort_by_key(|c| c.len());
                let _largest = components.pop().unwrap(); // Оставляем самый большой
                for fragment in components {
                    for coord in fragment {
                        if let Some(tile) = self.tiles.get_mut(&coord) {
                            tile.faction_id = None;
                        }
                    }
                }
                debug!("VALIDATION: Autocleaned isolated fragments for faction {}.", f_id);
            }
        }

        // 4. Проверка правила '1-Hex Gap' (только для отчета, т.к. кисть сама его соблюдает)
        for (coord, tile) in &self.tiles {
            if let Some(f1) = tile.faction_id {
                for n_coord in coord.neighbors() {
                    if let Some(n_tile) = self.tiles.get(&n_coord) {
                        if let Some(f2) = n_tile.faction_id {
                            if f1 != f2 {
                                // Ошибка возможна только если пользователь как-то обошел кисть или при генерации
                                self.validation_errors.push(format!("Нарушено правило 1 гекса между фракциями {} и {} у координат {:?}.", f1, f2, coord));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Resource)]
pub struct WorldSeed(u32);

impl WorldSeed {
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }
    #[must_use]
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for WorldSeed {
    fn default() -> Self {
        Self(42)
    }
}

#[derive(Component)]
pub struct Tile {
    pub terrain: TerrainType,
}

#[derive(Component)]
pub struct Roof;

use crate::map::MapEntity;

#[derive(Bundle)]
pub struct RoofBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub roof: Roof,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct SmoothTileBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub tile: Tile,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct GlobalTerrainBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub name: Name,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct WaterBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub name: Name,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct MountainRoofBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub roof: Roof,
    pub name: Name,
    pub marker: MapEntity,
}

use crate::game_state::FactionType;

#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct FactionMarker {
    pub faction_type: FactionType,
    pub hex_coord: HexCoord,
}

#[derive(Bundle)]
pub struct FactionMarkerBundle {
    pub marker: FactionMarker,
    pub name: Name,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct Zone(pub ZoneType);

pub struct ZoningPlugin;

impl Plugin for ZoningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldSeed>().init_resource::<MapData>();
    }
}
