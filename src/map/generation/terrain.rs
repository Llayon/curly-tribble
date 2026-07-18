use crate::game_state::EditorPhase;
use crate::map::data::{OceanState, RoofState};
use crate::map::navigation::NavigationMap;
use crate::map::terrain_gen::{TerrainConfig, TerrainGenerator};
use crate::map::{
    ForestType, GenerationMode, HexCoord, LandscapeFeature, MapData, TerrainType, TileData,
    WorldSeed, HEX_SIZE, MAX_HEIGHT,
};
use bevy::prelude::*;
use noise::{Fbm, NoiseFn, OpenSimplex};
use std::collections::{HashMap, VecDeque};

pub struct TerrainGenerationPlugin;

impl Plugin for TerrainGenerationPlugin {
    fn build(&self, _app: &mut App) {}
}

#[allow(clippy::cast_possible_truncation)]
pub fn spawn_map_internal(
    commands: &mut Commands,
    terrain_gen: &TerrainGenerator,
    terrain_config: &TerrainConfig,
    seed: &WorldSeed,
    map_data: &mut MapData,
    nav_map: &mut NavigationMap,
    faction_manager: &crate::game_state::FactionManager,
    phase: EditorPhase,
    generation_mode: GenerationMode,
    auto_fill: Option<EditorPhase>,
) {
    let reset = generation_mode == GenerationMode::Reset;
    map_data.tiles = build_base_tiles(
        map_data,
        terrain_gen,
        terrain_config,
        *seed,
        phase,
        reset,
        auto_fill,
    );
    map_data.width = terrain_config.map_width;
    map_data.height = terrain_config.map_height;

    let distance_field = distance_to_ocean(map_data);

    apply_landscape_generation(
        map_data,
        terrain_config,
        *seed,
        &distance_field,
        reset || auto_fill == Some(EditorPhase::Landscape),
    );

    if reset || !map_data.tiles.values().any(|t| t.faction_id == Some(1)) {
        super::factions::auto_spawn_player_territory(map_data, seed.value());
    }

    rebuild_navigation_grid(map_data, nav_map);

    commands.queue(crate::economy::mesh_gen::SpawnGlobalTerrainCommand {
        map_data: map_data.clone(),
        phase,
        faction_manager: faction_manager.clone(),
        config: (*terrain_config).clone(),
    });
}

fn build_base_tiles(
    previous: &MapData,
    generator: &TerrainGenerator,
    config: &TerrainConfig,
    seed: WorldSeed,
    phase: EditorPhase,
    reset: bool,
    auto_fill: Option<EditorPhase>,
) -> HashMap<HexCoord, TileData> {
    let temp_noise = Fbm::<OpenSimplex>::new(seed.value() + 1);
    let humid_noise = Fbm::<OpenSimplex>::new(seed.value() + 2);
    let half_w = (config.map_width / 2).cast_signed();
    let half_h = (config.map_height / 2).cast_signed();
    let mut tiles = HashMap::new();
    for q in -half_w..half_w {
        for r in -half_h..half_h {
            let coord = HexCoord::new(q, r);
            let world = coord.to_world(HEX_SIZE);
            let border = q <= -half_w + 1 || q >= half_w - 2 || r <= -half_h + 1 || r >= half_h - 2;
            let old = previous.get_tile(q, r);
            let ocean = border
                || if reset {
                    generator.get_shape_value(config, world.x, world.z) <= 0.0
                } else {
                    old.map_or_else(
                        || generator.get_shape_value(config, world.x, world.z) <= 0.0,
                        |tile| tile.ocean_state == OceanState::Ocean,
                    )
                };
            let (temperature, humidity, elevation) = if phase == EditorPhase::Shape {
                (0.5, 0.5, 0.1)
            } else {
                #[allow(clippy::cast_possible_truncation)]
                let temperature =
                    (temp_noise.get([f64::from(world.x) * 0.05, f64::from(world.z) * 0.05]) as f32
                        + 1.0)
                        * 0.5;
                #[allow(clippy::cast_possible_truncation)]
                let humidity = (humid_noise
                    .get([f64::from(world.x) * 0.05, f64::from(world.z) * 0.05])
                    as f32
                    + 1.0)
                    * 0.5;
                (
                    temperature,
                    humidity,
                    (generator.get_elevation(config, world.x, world.z) / MAX_HEIGHT)
                        .clamp(0.0, 1.0),
                )
            };
            let refresh = reset || auto_fill == Some(EditorPhase::Sediments);
            let terrain = if phase == EditorPhase::Shape || ocean {
                TerrainType::Grass
            } else if refresh {
                super::climate::get_terrain_from_climate(temperature, humidity, elevation)
            } else {
                old.map_or(TerrainType::Grass, |tile| tile.terrain)
            };
            let feature = if reset {
                LandscapeFeature::None
            } else {
                old.map_or(LandscapeFeature::None, |tile| tile.landscape_feature)
            };
            let (forest_type, forest_density) = if !ocean
                && feature == LandscapeFeature::None
                && refresh
                && humidity > 0.6
                && temperature > 0.3
            {
                let density = (humidity - 0.4).max(0.0) * 0.8;
                (
                    if temperature < 0.5 || elevation > 0.6 {
                        ForestType::Coniferous
                    } else {
                        ForestType::Deciduous
                    },
                    density,
                )
            } else {
                old.map_or((ForestType::None, 0.0), |tile| {
                    (tile.forest_type, tile.forest_density)
                })
            };
            tiles.insert(
                coord,
                TileData {
                    terrain,
                    forest_type,
                    forest_density,
                    elevation,
                    temperature,
                    humidity,
                    roof_state: RoofState::Open,
                    ocean_state: if ocean {
                        OceanState::Ocean
                    } else {
                        OceanState::Land
                    },
                    faction_id: if reset {
                        None
                    } else {
                        old.and_then(|tile| tile.faction_id)
                    },
                    landscape_feature: feature,
                },
            );
        }
    }
    tiles
}

fn distance_to_ocean(map_data: &MapData) -> HashMap<HexCoord, u32> {
    let mut distance_field = HashMap::new();
    let mut queue = VecDeque::new();
    for (coord, tile) in &map_data.tiles {
        if tile.ocean_state == OceanState::Ocean {
            distance_field.insert(*coord, 0);
            queue.push_back(*coord);
        }
    }
    while let Some(curr) = queue.pop_front() {
        let Some(&current_distance) = distance_field.get(&curr) else {
            continue;
        };
        for neighbor in curr.neighbors() {
            if map_data.tiles.contains_key(&neighbor) && !distance_field.contains_key(&neighbor) {
                distance_field.insert(neighbor, current_distance + 1);
                queue.push_back(neighbor);
            }
        }
    }
    distance_field
}

fn apply_landscape_generation(
    map_data: &mut MapData,
    terrain_config: &TerrainConfig,
    seed: WorldSeed,
    distance_field: &HashMap<HexCoord, u32>,
    should_generate: bool,
) {
    if !should_generate {
        return;
    }

    let plateau_noise = Fbm::<OpenSimplex>::new(seed.value() + 60);
    for (coord, tile) in &mut map_data.tiles {
        if tile.ocean_state == OceanState::Ocean || tile.faction_id.is_some() {
            continue;
        }
        let distance = *distance_field.get(coord).unwrap_or(&0);
        let world_pos = coord.to_world(HEX_SIZE);
        #[allow(clippy::cast_possible_truncation)]
        let noise =
            plateau_noise.get([f64::from(world_pos.x) * 0.1, f64::from(world_pos.z) * 0.1]) as f32;
        let plateau_value = (noise + 1.0) * 0.5;
        tile.landscape_feature = match (distance, plateau_value) {
            (distance, value) if distance > 5 && value > 0.6 => LandscapeFeature::Mountain,
            (distance, value) if distance > 4 && value > 0.45 => LandscapeFeature::Plateau,
            (distance, value) if distance > 3 && value < 0.35 => LandscapeFeature::Lake,
            _ => tile.landscape_feature,
        };
    }
    super::cliffs::generate_cliffs(map_data, distance_field, seed.value());
    crate::map::river_gen::apply_rivers(map_data, terrain_config, seed.value());
    if terrain_config.mud_banks == crate::map::terrain_gen::MudBankMode::Enabled {
        crate::map::river_gen::apply_mud_banks(map_data);
    }
}

fn rebuild_navigation_grid(map_data: &MapData, nav_map: &mut NavigationMap) {
    nav_map.grid.clear();
    for (coord, tile) in &map_data.tiles {
        let cost =
            if tile.ocean_state == OceanState::Ocean || map_data.is_too_steep(coord.q, coord.r) {
                crate::map::navigation::COST_BLOCKER
            } else {
                match tile.terrain {
                    TerrainType::Swamp => 50,
                    TerrainType::Stony => 80,
                    _ => crate::map::navigation::COST_BASE,
                }
            };
        nav_map.grid.insert(IVec2::new(coord.q, coord.r), cost);
    }
}
