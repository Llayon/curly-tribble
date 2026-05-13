use crate::economy::mesh_gen::MeshGenPlugin;
use crate::events::{GameLogMessage, LogSeverity};
use crate::sets::{GameSet, StartupSet};
use bevy::prelude::*;
use construction::ConstructionPlugin;
use navigation::NavigationPlugin;
use noise::{Fbm, NoiseFn, Perlin};
use rand::prelude::*;
use resources::ResourcesPlugin;
use river_gen::RiverGenPlugin;
use terrain_gen::{TerrainConfig, TerrainGenerator};
pub use zoning::{MapData, TerrainType, Tile, WorldSeed, MAX_HEIGHT};

pub mod atmosphere;
pub mod construction;
pub mod navigation;
pub mod resources;
pub mod river_gen;
pub mod terrain_gen;
pub mod visibility;
pub mod zoning;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct MapEntity;

#[derive(Message)]
pub struct GenerateMapEvent;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let config = TerrainConfig::default();
        app.insert_resource(TerrainGenerator::new(config.seed))
            .insert_resource(config)
            .register_type::<TerrainConfig>()
            .register_type::<MapEntity>()
            .add_plugins(bevy_inspector_egui::quick::ResourceInspectorPlugin::<
                TerrainConfig,
            >::default())
            .add_message::<GenerateMapEvent>()
            .add_plugins((
                zoning::ZoningPlugin,
                ResourcesPlugin,
                ConstructionPlugin,
                NavigationPlugin,
                visibility::VisibilityPlugin,
                MeshGenPlugin,
                RiverGenPlugin,
                terrain_gen::TerrainGenPlugin,
            ))
            .add_systems(
                Startup,
                (|mut ev: MessageWriter<GenerateMapEvent>| {
                    ev.write(GenerateMapEvent);
                })
                .in_set(StartupSet::SpawnEntities),
            )
            .add_systems(
                Update,
                (
                    handle_regeneration,
                    monitor_inspector_triggers.run_if(resource_changed::<TerrainConfig>),
                )
                    .in_set(GameSet::Logic),
            );
    }
}

#[derive(Bundle)]
pub struct MapTileBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub tile: Tile,
}

fn handle_regeneration(
    mut commands: Commands,
    mut ev_gen: MessageReader<GenerateMapEvent>,
    q_map_entities: Query<Entity, With<MapEntity>>,
    config: Res<TerrainConfig>,
    mut seed: ResMut<WorldSeed>,
    mut terrain_gen: ResMut<TerrainGenerator>,
    mut map_data: ResMut<MapData>,
    mut nav_map: ResMut<crate::map::navigation::NavigationMap>,
    mut log_writer: MessageWriter<GameLogMessage>,
) {
    for _ in ev_gen.read() {
        debug!("MAP_GEN: Received GenerateMapEvent. Starting cleanup...");

        // 1. Очистка старого мира
        let mut count = 0;
        for entity in &q_map_entities {
            commands.entity(entity).despawn(); // Т.к. иерархий нет, обычного despawn достаточно
            count += 1;
        }
        debug!("MAP_GEN: Despawned {} map entities.", count);

        nav_map.grid.clear();
        debug!("MAP_GEN: Navigation grid cleared.");

        // 2. Обновление ресурсов на основе конфига
        *seed = WorldSeed::new(config.seed);
        *terrain_gen = TerrainGenerator::new(config.seed);
        debug!(
            "MAP_GEN: Generator re-initialized with seed {}.",
            config.seed
        );

        // 3. Спавн нового мира
        spawn_map_internal(
            &mut commands,
            &terrain_gen,
            &config,
            &seed,
            &mut map_data,
            &mut nav_map,
        );

        log_writer.write(GameLogMessage {
            message: format!("World regenerated with seed {}", config.seed),
            severity: LogSeverity::Info,
        });
    }
}

/// Система, которая следит за "кнопками-галками" в инспекторе `TerrainConfig`
fn monitor_inspector_triggers(
    mut config: ResMut<TerrainConfig>,
    mut ev_gen: MessageWriter<GenerateMapEvent>,
) {
    let mut triggered = false;

    if config.randomize_seed {
        config.seed = rand::thread_rng().gen_range(0..999_999);
        config.randomize_seed = false;
        debug!("INSPECTOR: Seed randomized to {}", config.seed);
        triggered = true;
    }

    if config.regenerate_world {
        config.regenerate_world = false;
        triggered = true;
    }

    if triggered {
        debug!("INSPECTOR: Triggering regeneration event.");
        ev_gen.write(GenerateMapEvent);
    }
}

#[allow(clippy::cast_possible_truncation)] // Noise output f64 to f32 is intentional for terrain climate
fn spawn_map_internal(
    commands: &mut Commands,
    terrain_gen: &TerrainGenerator,
    terrain_config: &TerrainConfig,
    seed: &WorldSeed,
    map_data: &mut MapData,
    nav_map: &mut crate::map::navigation::NavigationMap,
) {
    let temp_noise = Fbm::<Perlin>::new(seed.value() + 1);
    let humid_noise = Fbm::<Perlin>::new(seed.value() + 2);

    let width = terrain_config.map_width;
    let height = terrain_config.map_height;
    let half_w = (width / 2).cast_signed();
    let half_h = (height / 2).cast_signed();

    map_data.width = width;
    map_data.height = height;
    map_data.tiles = vec![crate::map::zoning::TileData::default(); (width * height) as usize];

    for x in -half_w..half_w {
        for z in -half_h..half_h {
            let elevation = terrain_gen.get_elevation(terrain_config, x as f32, z as f32);
            let normalized_elevation = (elevation / MAX_HEIGHT).clamp(0.0, 1.0);

            let temp_val =
                ((temp_noise.get([f64::from(x) * 0.05, f64::from(z) * 0.05]) as f32) + 1.0) * 0.5;
            let humid_val =
                ((humid_noise.get([f64::from(x) * 0.05, f64::from(z) * 0.05]) as f32) + 1.0) * 0.5;

            let terrain = get_terrain_from_climate(temp_val, humid_val, normalized_elevation);

            if let Some(tile_data) = map_data.get_tile_mut(x, z) {
                tile_data.terrain = terrain;
                tile_data.elevation = normalized_elevation;
                tile_data.temperature = temp_val;
                tile_data.humidity = humid_val;
                tile_data.roofed = false;
            }
        }
    }

    // Apply Rivers and Mud Banks
    river_gen::apply_rivers(map_data, terrain_config, seed.value());
    river_gen::apply_mud_banks(map_data);

    /*
    let mut rng = StdRng::seed_from_u64(u64::from(seed.value()) + 100);
    for x in -half_w..half_w {
        for z in -half_h..half_h {
            if let Some(tile_data) = map_data.get_tile(x, z) {
                // ПРИМЕЧАНИЕ: Пещеры временно отключены для исправления визуальных багов геометрии.
                if tile_data.terrain == TerrainType::Stone
                    && rng.gen_bool(0.05)
                    && tile_data.elevation > 0.6
                {
                    apply_cave_stamp(map_data, x, z);
                }
            }
        }
    }
    */

    for x in -half_w..half_w {
        for z in -half_h..half_h {
            let tile_data = map_data.get_tile(x, z).copied().unwrap_or_default();
            let terrain = tile_data.terrain;

            // Логическая сущность тайла (без меша) для кликов и ИИ
            commands.spawn(zoning::LogicTileBundle {
                transform: Transform::from_xyz(x as f32, 0.0, z as f32),
                tile: Tile { terrain },
                name: Name::new(format!("Tile {x},{z}")),
                marker: MapEntity,
            });

            let mut cost = crate::map::navigation::COST_BASE;
            if map_data.is_too_steep(x, z) {
                cost = crate::map::navigation::COST_BLOCKER;
            } else {
                match terrain {
                    TerrainType::Water => {
                        cost = crate::map::navigation::COST_BLOCKER;
                    }
                    TerrainType::Mud => {
                        cost = 50;
                    }
                    TerrainType::Stone => {
                        cost = 80;
                    }
                    TerrainType::Grass | TerrainType::Sand | TerrainType::CaveFloor => {}
                }
            }
            nav_map.grid.insert(IVec2::new(x, z), cost);
        }
    }

    // Создаем глобальный ландшафт, воду и крыши одной командой
    commands.queue(crate::economy::mesh_gen::SpawnGlobalTerrainCommand {
        map_data: map_data.clone(),
    });
}

#[allow(dead_code)]
fn apply_cave_stamp(map: &mut MapData, x: i32, z: i32) {
    for dx in -1..=1 {
        for dz in -1..=1 {
            if let Some(tile) = map.get_tile_mut(x + dx, z + dz) {
                tile.terrain = TerrainType::CaveFloor;
                tile.roofed = true;
            }
        }
    }
}

fn get_terrain_from_climate(temp: f32, humid: f32, elev: f32) -> TerrainType {
    if elev < 0.2 {
        return TerrainType::Water;
    }
    if elev < 0.25 {
        return TerrainType::Sand;
    }
    if elev > 0.8 {
        return TerrainType::Stone;
    }

    if humid > 0.7 {
        if temp < 0.3 {
            TerrainType::Mud
        } else {
            TerrainType::Grass
        }
    } else if humid < 0.3 {
        if temp > 0.7 {
            TerrainType::Sand
        } else {
            TerrainType::Grass
        }
    } else {
        TerrainType::Grass
    }
}
