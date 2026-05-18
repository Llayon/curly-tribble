use crate::economy::mesh_gen::MeshGenPlugin;
use crate::events::{GameLogMessage, LogSeverity};
use crate::sets::{GameSet, StartupSet};
use bevy::prelude::*;
use std::collections::HashMap;
use construction::ConstructionPlugin;
use navigation::NavigationPlugin;
use noise::{Fbm, NoiseFn, OpenSimplex};
use rand::prelude::*;
use resources::ResourcesPlugin;
use river_gen::RiverGenPlugin;
use terrain_gen::{TerrainConfig, TerrainGenerator};
pub use zoning::{MapData, TerrainType, Tile, WorldSeed, MAX_HEIGHT};
pub mod hex_math;
pub use hex_math::HexCoord;

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
pub struct GenerateMapEvent {
    pub force_reset: bool,
}

#[derive(Message)]
pub struct RebuildMeshEvent;

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
            .add_message::<RebuildMeshEvent>()
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
                    ev.write(GenerateMapEvent { force_reset: true });
                })
                .in_set(StartupSet::SpawnEntities),
            )
            .add_systems(
                Update,
                (
                    handle_regeneration.in_set(GameSet::Logic),
                    handle_rebuild_mesh.in_set(GameSet::Logic),
                    handle_shape_tools.in_set(GameSet::Logic),
                    monitor_inspector_triggers
                        .run_if(resource_changed::<TerrainConfig>)
                        .in_set(GameSet::Logic),
                    rebuild_map_on_phase_change
                        .run_if(state_changed::<crate::game_state::EditorPhase>)
                        .in_set(GameSet::Logic),
                ),
            );
    }
}

fn rebuild_map_on_phase_change(mut ev_gen: MessageWriter<GenerateMapEvent>) {
    debug!("STATE_CHANGE: EditorPhase changed. Triggering map rebuild (preserving edits).");
    ev_gen.write(GenerateMapEvent { force_reset: false });
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
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    for ev in ev_gen.read() {
        debug!("MAP_GEN: Received GenerateMapEvent (ForceReset: {}). Starting cleanup...", ev.force_reset);

        // 1. Глубокая очистка мира
        for entity in &q_map_entities {
            commands.entity(entity).despawn();
        }
        debug!("MAP_GEN: Map entities cleaned.");

        nav_map.grid.clear();

        // 2. Обновление ресурсов
        *seed = WorldSeed::new(config.seed);
        *terrain_gen = TerrainGenerator::new(config.seed);
        
        // Синхронизируем размеры в MapData
        map_data.width = config.map_width;
        map_data.height = config.map_height;

        // 3. Полный цикл спавна
        spawn_map_internal(
            &mut commands,
            &terrain_gen,
            &config,
            &seed,
            &mut map_data,
            &mut nav_map,
            *phase.get(),
            ev.force_reset,
        );

        map_data.run_validation();

        log_writer.write(GameLogMessage {
            message: format!("World regenerated: {}x{}, seed {}", config.map_width, config.map_height, config.seed),
            severity: LogSeverity::Info,
        });
    }
}

fn handle_rebuild_mesh(
    mut commands: Commands,
    mut ev_rebuild: MessageReader<RebuildMeshEvent>,
    q_map_entities: Query<Entity, With<MapEntity>>,
    map_data: Res<MapData>,
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    for _ in ev_rebuild.read() {
        // Очистка только мешей (MapEntity)
        for entity in &q_map_entities {
            commands.entity(entity).despawn();
        }

        // Спавн только мешей на основе существующих MapData
        commands.queue(crate::economy::mesh_gen::SpawnGlobalTerrainCommand {
            map_data: map_data.clone(),
            phase: *phase.get(),
        });
    }
}

fn handle_shape_tools(
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut map_data: ResMut<MapData>,
    current_tool: Res<crate::game_state::CurrentTool>,
    phase: Res<State<crate::game_state::EditorPhase>>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    if *phase.get() != crate::game_state::EditorPhase::Shape
        || current_tool.shape == crate::game_state::ShapeTool::None
    {
        return;
    }

    if mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right) {
        let Ok((camera, camera_transform)) = q_camera.single() else {
            return;
        };
        let Ok(window) = q_window.single() else {
            return;
        };

        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) {
                // Плейн на высоте 0 для кликов
                let distance = ray.origin.y / -ray.direction.y;
                if distance > 0.0 {
                    let world_pos = ray.origin + ray.direction * distance;

                    let coord = crate::map::hex_math::HexCoord::from_world(world_pos, zoning::HEX_SIZE);
                    if let Some(tile) = map_data.get_tile_mut(coord.q, coord.r) {
                        let is_ocean = mouse.pressed(MouseButton::Left);
                        if tile.is_ocean != is_ocean {
                            tile.is_ocean = is_ocean;
                            map_data.run_validation();
                            ev_rebuild.write(RebuildMeshEvent);
                        }
                    }
                }
            }
        }
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
        debug!("INSPECTOR: Triggering regeneration event (Force Reset).");
        ev_gen.write(GenerateMapEvent { force_reset: true });
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
    phase: crate::game_state::EditorPhase,
    force_reset: bool,
) {
    let temp_noise = Fbm::<OpenSimplex>::new(seed.value() + 1);
    let humid_noise = Fbm::<OpenSimplex>::new(seed.value() + 2);

    let width = terrain_config.map_width;
    let height = terrain_config.map_height;
    let half_w = (width / 2).cast_signed();
    let half_h = (height / 2).cast_signed();

    map_data.width = width;
    map_data.height = height;
    
    let mut new_tiles = HashMap::new();

    for q in -half_w..half_w {
        for r in -half_h..half_h {
            let coord = HexCoord::new(q, r);
            let world_pos = coord.to_world(zoning::HEX_SIZE);
            
            // 1. Сохраняем состояние океана, если это НЕ форсированный сброс
            let is_border = q <= -half_w + 1 || q >= half_w - 2 || r <= -half_h + 1 || r >= half_h - 2;
            
            let mut is_ocean = if force_reset {
                let shape_val = terrain_gen.get_shape_value(terrain_config, world_pos.x, world_pos.z);
                is_border || shape_val <= 0.0
            } else {
                map_data.get_tile(q, r).map_or_else(
                    || {
                        let shape_val = terrain_gen.get_shape_value(terrain_config, world_pos.x, world_pos.z);
                        is_border || shape_val <= 0.0
                    },
                    |t| t.is_ocean
                )
            };

            if is_border { is_ocean = true; }

            // 2. Расчет высоты и климата
            let (terrain, temp_val, humid_val, normalized_elevation) = if phase == crate::game_state::EditorPhase::Shape {
                // На фазе Shape мы НЕ считаем сложную 3D-высоту, только форму
                (TerrainType::Grass, 0.5, 0.5, 0.1)
            } else {
                let elevation = terrain_gen.get_elevation(terrain_config, world_pos.x, world_pos.z);
                let norm_elev = (elevation / MAX_HEIGHT).clamp(0.0, 1.0);
                
                let temp =
                    ((temp_noise.get([f64::from(world_pos.x) * 0.05, f64::from(world_pos.z) * 0.05]) as f32) + 1.0) * 0.5;
                let humid =
                    ((humid_noise.get([f64::from(world_pos.x) * 0.05, f64::from(world_pos.z) * 0.05]) as f32) + 1.0) * 0.5;
                
                let t = if is_ocean {
                    TerrainType::Water
                } else {
                    let climate_terrain = get_terrain_from_climate(temp, humid, norm_elev);
                    if climate_terrain == TerrainType::Water {
                        TerrainType::Sand
                    } else {
                        climate_terrain
                    }
                };
                (t, temp, humid, norm_elev)
            };

            let tile_data = crate::map::zoning::TileData {
                terrain,
                elevation: normalized_elevation,
                temperature: temp_val,
                humidity: humid_val,
                roofed: false,
                is_ocean,
            };
            new_tiles.insert(coord, tile_data);
        }
    }
    map_data.tiles = new_tiles;

    for q in -half_w..half_w {
        for r in -half_h..half_h {
            let tile_data = map_data.get_tile(q, r).copied().unwrap_or_default();
            let terrain = tile_data.terrain;

            let mut cost = crate::map::navigation::COST_BASE;
            if tile_data.is_ocean {
                cost = crate::map::navigation::COST_BLOCKER;
            } else if map_data.is_too_steep(q, r) {
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
            nav_map.grid.insert(IVec2::new(q, r), cost);
        }
    }

    commands.queue(crate::economy::mesh_gen::SpawnGlobalTerrainCommand {
        map_data: map_data.clone(),
        phase,
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
