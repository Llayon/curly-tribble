use crate::economy::mesh_gen::MeshGenPlugin;
use crate::events::{GameLogMessage, LogSeverity};
use crate::sets::{GameSet, StartupSet};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
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
                    handle_faction_tools.in_set(GameSet::Logic),
                    handle_landscape_tools.in_set(GameSet::Logic),
                    handle_sediment_tools.in_set(GameSet::Logic),
                    handle_faction_auto_relocation.in_set(GameSet::Logic),
                    validate_faction_placements.in_set(GameSet::Logic),
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

fn rebuild_map_on_phase_change(
    mut ev_gen: MessageWriter<GenerateMapEvent>,
    phase: Res<State<crate::game_state::EditorPhase>>,
    map_data: Res<MapData>,
) {
    debug!("MAP_GEN: Phase changed to {:?}. Refreshing mesh...", *phase.get());
    
    // Авто-генерация при переходе ВПЕРЕД
    let current_phase = *phase.get();
    let _needs_auto_gen = match current_phase {
        crate::game_state::EditorPhase::Landscape => {
            !map_data.tiles.values().any(|t| t.landscape_feature != zoning::LandscapeFeature::None)
        }
        crate::game_state::EditorPhase::Sediments => {
            !map_data.tiles.values().any(|t| t.terrain != zoning::TerrainType::Grass || t.forest_type != zoning::ForestType::None)
        }
        _ => false,
    };

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
    q_faction_markers: Query<Entity, With<zoning::FactionMarker>>,
    config: Res<TerrainConfig>,
    mut seed: ResMut<WorldSeed>,
    mut terrain_gen: ResMut<TerrainGenerator>,
    mut map_data: ResMut<MapData>,
    mut nav_map: ResMut<crate::map::navigation::NavigationMap>,
    mut log_writer: MessageWriter<GameLogMessage>,
    faction_manager: Res<crate::game_state::FactionManager>,
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    for ev in ev_gen.read() {
        debug!("MAP_GEN: Received GenerateMapEvent (ForceReset: {}). Starting cleanup...", ev.force_reset);

        // 1. Глубокая очистка мира
        for entity in &q_map_entities {
            commands.entity(entity).despawn();
        }
        
        // Если это полный сброс, удаляем и фракции
        if ev.force_reset {
            for entity in &q_faction_markers {
                commands.entity(entity).despawn();
            }
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
            &faction_manager,
            *phase.get(),
            ev.force_reset,
        );

        if ev.force_reset {
            spawn_factions(&mut commands, &map_data, &faction_manager);
        }

        map_data.run_validation();

        log_writer.write(GameLogMessage {
            message: format!("World regenerated: {}x{}, seed {}", config.map_width, config.map_height, config.seed),
            severity: LogSeverity::Info,
        });
    }
}

fn spawn_factions(
    commands: &mut Commands,
    map_data: &MapData,
    faction_manager: &crate::game_state::FactionManager,
) {
    use crate::map::zoning::{FactionMarker, FactionMarkerBundle};

    let mut faction_territories: HashMap<u32, Vec<HexCoord>> = HashMap::new();
    for (coord, tile) in &map_data.tiles {
        if let Some(f_id) = tile.faction_id {
            faction_territories.entry(f_id).or_default().push(*coord);
        }
    }

    for (f_id, coords) in faction_territories {
        if coords.is_empty() { continue; }

        let faction = faction_manager.factions.iter().find(|f| f.id == f_id);
        let f_type = faction.map(|f| f.faction_type).unwrap_or(crate::game_state::FactionType::Neutral);
        let f_name = faction.map(|f| f.name.clone()).unwrap_or_else(|| format!("Faction {}", f_id));

        // Ищем гекс, максимально близкий к центру масс территории, который при этом наиболее плоский
        let mut sum_q = 0;
        let mut sum_r = 0;
        for c in &coords {
            sum_q += c.q;
            sum_r += c.r;
        }
        let avg_q = sum_q / coords.len() as i32;
        let avg_r = sum_r / coords.len() as i32;
        let center = HexCoord::new(avg_q, avg_r);

        // Находим гекс территории, который:
        // 1. Близко к центру (в радиусе 3 гексов)
        // 2. Является самым плоским (минимальный перепад высот с соседями)
        let candidates: Vec<_> = coords.iter()
            .filter(|c| c.distance(center) <= 3)
            .collect();

        let best_coord = if candidates.is_empty() {
            // Если в радиусе 3 ничего нет, берем просто ближайший к центру
            *coords.iter().min_by_key(|c| c.distance(center)).unwrap()
        } else {
            **candidates.iter().min_by_key(|c| {
                // Оценка "плоскости": сумма разностей высот с соседями
                let mut flatness_score = 0.0;
                let h_center = map_data.get_tile(c.q, c.r).map_or(0.0, |t| t.elevation);
                for n in c.neighbors() {
                    if let Some(nt) = map_data.get_tile(n.q, n.r) {
                        flatness_score += (nt.elevation - h_center).abs();
                    }
                }
                (flatness_score * 1000.0) as i32 // Превращаем в i32 для min_by_key
            }).unwrap()
        };

        commands.spawn(FactionMarkerBundle {
            marker: FactionMarker {
                faction_type: f_type,
                hex_coord: best_coord,
            },
            name: Name::new(f_name),
            transform: Transform::from_translation(best_coord.to_world(zoning::HEX_SIZE)),
            visibility: Visibility::Visible,
            inherited_visibility: InheritedVisibility::default(),
        });    }
}
fn validate_faction_placements(
    map_data: Res<MapData>,
    mut q_factions: Query<(&mut zoning::FactionMarker, &mut Transform)>,
) {
    if map_data.is_changed() {
        for (mut marker, mut transform) in &mut q_factions {
            let coord = marker.hex_coord;
            let is_invalid = map_data.get_tile(coord.q, coord.r).map_or(true, |t| t.is_ocean);

            if is_invalid {
                // Реактивная релокация: BFS поиск ближайшей суши
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
                    
                    if visited.len() > 400 { break; }

                    for neighbor in curr.neighbors() {
                        if !visited.contains(&neighbor) {
                            visited.insert(neighbor);
                            queue.push_back(neighbor);
                        }
                    }
                }

                if let Some(new_coord) = found_coord {
                    marker.hex_coord = new_coord;
                    transform.translation = new_coord.to_world(zoning::HEX_SIZE);
                    debug!("FACTION: Relocated faction to {:?}", new_coord);
                }
            } else {
                transform.translation = coord.to_world(zoning::HEX_SIZE);
            }
        }
    }
}

fn handle_rebuild_mesh(
    mut commands: Commands,
    mut ev_rebuild: MessageReader<RebuildMeshEvent>,
    q_map_entities: Query<Entity, With<MapEntity>>,
    map_data: Res<MapData>,
    faction_manager: Res<crate::game_state::FactionManager>,
    config: Res<TerrainConfig>,
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    for _ in ev_rebuild.read() {
        for entity in &q_map_entities {
            commands.entity(entity).despawn();
        }

        commands.queue(crate::economy::mesh_gen::SpawnGlobalTerrainCommand {
            map_data: map_data.clone(),
            phase: *phase.get(),
            faction_manager: faction_manager.clone(),
            config: (*config).clone(),
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
                let distance = ray.origin.y / -ray.direction.y;
                if distance > 0.0 {
                    let world_pos = ray.origin + ray.direction * distance;

                    let coord = crate::map::hex_math::HexCoord::from_world(world_pos, zoning::HEX_SIZE);
                    if let Some(tile) = map_data.get_tile_mut(coord.q, coord.r) {
                        let is_ocean = mouse.pressed(MouseButton::Left);
                        if tile.is_ocean != is_ocean {
                            tile.is_ocean = is_ocean;
                            if is_ocean {
                                tile.faction_id = None;
                            }
                            map_data.run_validation();
                            ev_rebuild.write(RebuildMeshEvent);
                        }
                    }
                }
            }
        }
    }
}

fn handle_sediment_tools(
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut map_data: ResMut<MapData>,
    current_tool: Res<crate::game_state::CurrentTool>,
    phase: Res<State<crate::game_state::EditorPhase>>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    if *phase.get() != crate::game_state::EditorPhase::Sediments {
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
                let distance = ray.origin.y / -ray.direction.y;
                if distance > 0.0 {
                    let world_pos = ray.origin + ray.direction * distance;
                    let coord = crate::map::hex_math::HexCoord::from_world(world_pos, zoning::HEX_SIZE);
                    
                    if let Some(tile) = map_data.get_tile_mut(coord.q, coord.r) {
                        let mut changed = false;

                        // 1. Седименты
                        if current_tool.active_sediment_tool {
                            let target_sediment = if mouse.pressed(MouseButton::Left) {
                                current_tool.sediment
                            } else {
                                zoning::TerrainType::Grass // RM removes to grass
                            };

                            if tile.terrain != target_sediment {
                                tile.terrain = target_sediment;
                                changed = true;
                            }
                        }

                        // 2. Леса
                        if current_tool.active_forest_tool {
                            let (target_type, target_density) = if mouse.pressed(MouseButton::Left) {
                                (current_tool.forest_type, current_tool.forest_density)
                            } else {
                                (zoning::ForestType::None, 0.0)
                            };

                            if tile.forest_type != target_type || (tile.forest_density - target_density).abs() > 0.01 {
                                tile.forest_type = target_type;
                                tile.forest_density = target_density;
                                changed = true;
                            }
                        }

                        if changed {
                            ev_rebuild.write(RebuildMeshEvent);
                        }
                    }
                }
            }
        }
    }
}

fn handle_landscape_tools(
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut map_data: ResMut<MapData>,
    current_tool: Res<crate::game_state::CurrentTool>,
    phase: Res<State<crate::game_state::EditorPhase>>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    if *phase.get() != crate::game_state::EditorPhase::Landscape
        || current_tool.landscape == crate::game_state::LandscapeTool::None
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
                let distance = ray.origin.y / -ray.direction.y;
                if distance > 0.0 {
                    let world_pos = ray.origin + ray.direction * distance;
                    let coord = crate::map::hex_math::HexCoord::from_world(world_pos, zoning::HEX_SIZE);
                    
                    match current_tool.landscape {
                        crate::game_state::LandscapeTool::Mountain | 
                        crate::game_state::LandscapeTool::Lake | 
                        crate::game_state::LandscapeTool::River |
                        crate::game_state::LandscapeTool::Plateau => {
                            if let Some(tile) = map_data.get_tile_mut(coord.q, coord.r) {
                                let new_feature = if mouse.pressed(MouseButton::Left) {
                                    match current_tool.landscape {
                                        crate::game_state::LandscapeTool::Mountain => zoning::LandscapeFeature::Mountain,
                                        crate::game_state::LandscapeTool::Lake => zoning::LandscapeFeature::Lake,
                                        crate::game_state::LandscapeTool::River => zoning::LandscapeFeature::River,
                                        crate::game_state::LandscapeTool::Plateau => zoning::LandscapeFeature::Plateau,
                                        _ => zoning::LandscapeFeature::None,
                                    }
                                } else {
                                    zoning::LandscapeFeature::None
                                };

                                if tile.landscape_feature != new_feature {
                                    tile.landscape_feature = new_feature;
                                    ev_rebuild.write(RebuildMeshEvent);
                                }
                            }
                        }
                        crate::game_state::LandscapeTool::Cliff => {
                            let mut best_edge = None;
                            let mut min_dist = f32::MAX;

                            for neighbor in coord.neighbors() {
                                let edge = zoning::EdgeCoord::new(coord, neighbor);
                                let center_a = coord.to_world(zoning::HEX_SIZE);
                                let center_b = neighbor.to_world(zoning::HEX_SIZE);
                                let edge_midpoint = (center_a + center_b) * 0.5;
                                
                                let dist = world_pos.distance(edge_midpoint);
                                if dist < min_dist && dist < zoning::HEX_SIZE * 0.6 {
                                    min_dist = dist;
                                    best_edge = Some(edge);
                                }
                            }

                            if let Some(edge) = best_edge {
                                if mouse.just_pressed(MouseButton::Left) {
                                    let mut data = map_data.edges.get(&edge).copied().unwrap_or_default();
                                    if !data.is_cliff {
                                        data.is_cliff = true;
                                        data.direction = true;
                                    } else {
                                        data.direction = !data.direction;
                                    }
                                    map_data.edges.insert(edge, data);
                                    ev_rebuild.write(RebuildMeshEvent);
                                } else if mouse.pressed(MouseButton::Right) {
                                    if map_data.edges.remove(&edge).is_some() {
                                        ev_rebuild.write(RebuildMeshEvent);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn handle_faction_tools(
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut map_data: ResMut<MapData>,
    current_tool: Res<crate::game_state::CurrentTool>,
    faction_manager: Res<crate::game_state::FactionManager>,
    phase: Res<State<crate::game_state::EditorPhase>>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    if *phase.get() != crate::game_state::EditorPhase::Factions
        || current_tool.faction != crate::game_state::FactionTool::Brush
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
                let distance = ray.origin.y / -ray.direction.y;
                if distance > 0.0 {
                    let world_pos = ray.origin + ray.direction * distance;

                    let coord = crate::map::hex_math::HexCoord::from_world(world_pos, zoning::HEX_SIZE);
                    
                    let target_faction = if mouse.pressed(MouseButton::Left) {
                        faction_manager.selected_faction
                    } else {
                        None
                    };

                    if let Some(tile) = map_data.get_tile(coord.q, coord.r) {
                        // 1. Проверка на сушу (Land only)
                        if tile.is_ocean { return; }

                        // 2. ЗАЩИТА: Нельзя рисовать поверх существующих фракций
                        if let Some(existing_fid) = tile.faction_id {
                            if let Some(target_fid) = target_faction {
                                if existing_fid != target_fid { return; }
                            }
                        }

                        // 3. Проверка Правила 1 Гекса (The 1-Hex Gap)
                        if let Some(f_id) = target_faction {
                            for neighbor_coord in coord.neighbors() {
                                if let Some(neighbor) = map_data.get_tile(neighbor_coord.q, neighbor_coord.r) {
                                    if let Some(n_f_id) = neighbor.faction_id {
                                        if n_f_id != f_id {
                                            // Блокируем покраску, если сосед принадлежит другой фракции
                                            return;
                                        }
                                    }
                                }
                            }
                        }

                        // Если всё ок, красим
                        if let Some(tile_mut) = map_data.get_tile_mut(coord.q, coord.r) {
                            // Защита Фракции 1 (Player Start)
                            if tile_mut.faction_id == Some(1) {
                                return;
                            }

                            if tile_mut.faction_id != target_faction {
                                tile_mut.faction_id = target_faction;
                                ev_rebuild.write(RebuildMeshEvent);
                            }
                        }
                    }
                }
            }
        }
    }
}

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
    faction_manager: &crate::game_state::FactionManager,
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

            let faction_id = if force_reset {
                None
            } else {
                map_data.get_tile(q, r).and_then(|t| t.faction_id)
            };

            let (terrain, temp_val, humid_val, normalized_elevation, feature) = if phase == crate::game_state::EditorPhase::Shape {
                (TerrainType::Grass, 0.5, 0.5, 0.1, zoning::LandscapeFeature::None)
            } else {
                let elevation = terrain_gen.get_elevation(terrain_config, world_pos.x, world_pos.z);
                let norm_elev = (elevation / MAX_HEIGHT).clamp(0.0, 1.0);
                
                let temp =
                    ((temp_noise.get([f64::from(world_pos.x) * 0.05, f64::from(world_pos.z) * 0.05]) as f32) + 1.0) * 0.5;
                let humid =
                    ((humid_noise.get([f64::from(world_pos.x) * 0.05, f64::from(world_pos.z) * 0.05]) as f32) + 1.0) * 0.5;
                
                let t = if is_ocean {
                    TerrainType::Grass
                } else {
                    get_terrain_from_climate(temp, humid, norm_elev)
                };

                let feat = if force_reset {
                    zoning::LandscapeFeature::None
                } else {
                    map_data.get_tile(q, r).map_or(zoning::LandscapeFeature::None, |t| t.landscape_feature)
                };

                (t, temp, humid, norm_elev, feat)
            };

            // Авто-генерация лесов для Фазы 4
            let (f_type, f_density) = if !is_ocean && feature == zoning::LandscapeFeature::None {
                if humid_val > 0.6 && temp_val > 0.3 {
                    let density = (humid_val - 0.4).max(0.0) * 0.8;
                    if temp_val < 0.5 || normalized_elevation > 0.6 {
                        (zoning::ForestType::Coniferous, density)
                    } else {
                        (zoning::ForestType::Deciduous, density)
                    }
                } else {
                    (zoning::ForestType::None, 0.0)
                }
            } else {
                (zoning::ForestType::None, 0.0)
            };

            let tile_data = crate::map::zoning::TileData {
                terrain,
                forest_type: f_type,
                forest_density: f_density,
                elevation: normalized_elevation,
                temperature: temp_val,
                humidity: humid_val,
                roofed: false,
                is_ocean,
                faction_id,
                landscape_feature: feature,
            };
            new_tiles.insert(coord, tile_data);
        }
    }
    map_data.tiles = new_tiles;

    // 1. Расчет Distance Field (расстояние до океана)
    let mut distance_field: HashMap<HexCoord, u32> = HashMap::new();
    let mut queue = VecDeque::new();
    for (coord, tile) in &map_data.tiles {
        if tile.is_ocean {
            distance_field.insert(*coord, 0);
            queue.push_back(*coord);
        }
    }

    while let Some(curr) = queue.pop_front() {
        let curr_dist = *distance_field.get(&curr).unwrap();
        for n in curr.neighbors() {
            if map_data.tiles.contains_key(&n) && !distance_field.contains_key(&n) {
                distance_field.insert(n, curr_dist + 1);
                queue.push_back(n);
            }
        }
    }

    // 2. Процедурная расстановка Landscape Features (Mountains, Lakes, Plateaus)
    let plateau_noise = Fbm::<OpenSimplex>::new(seed.value() + 60);

    for (coord, tile) in map_data.tiles.iter_mut() {
        if tile.is_ocean { continue; }
        
        // Маска фракций: Внутри территорий фракций - только плоская земля
        if tile.faction_id.is_some() {
            tile.landscape_feature = zoning::LandscapeFeature::None;
            continue;
        }

        let dist = *distance_field.get(coord).unwrap_or(&0);
        let world_pos = coord.to_world(zoning::HEX_SIZE);
        let p_noise = ((plateau_noise.get([f64::from(world_pos.x) * 0.1, f64::from(world_pos.z) * 0.1]) as f32) + 1.0) * 0.5;

        if force_reset {
            // Горы: Глубоко внутри суши, высокий шум
            if dist > 8 && p_noise > 0.7 {
                tile.landscape_feature = zoning::LandscapeFeature::Mountain;
            } 
            // Плато: Средняя удаленность, средний шум
            else if dist > 4 && p_noise > 0.5 {
                tile.landscape_feature = zoning::LandscapeFeature::Plateau;
            }
            // Озера: Недалеко от берега или в низинах (по шуму), если нет других фич
            else if dist > 3 && p_noise < 0.15 {
                tile.landscape_feature = zoning::LandscapeFeature::Lake;
            }
        }
    }

    // 3. Авто-генерация клиффов
    if force_reset {
        map_data.edges.clear();
        let mut new_cliffs = Vec::new();
        
        {
            let coords: Vec<_> = map_data.tiles.keys().copied().collect();
            for coord in coords {
                let tile_a = map_data.get_tile(coord.q, coord.r).unwrap();
                let feat_a = tile_a.landscape_feature;
                
                for n in coord.neighbors() {
                    if let Some(tile_b) = map_data.get_tile(n.q, n.r) {
                        let feat_b = tile_b.landscape_feature;
                        
                        let mut is_cliff = false;
                        let mut direction = true;

                        // А) Границы Гор и Плато
                        if (feat_a != feat_b) && 
                           (feat_a == zoning::LandscapeFeature::Mountain || feat_a == zoning::LandscapeFeature::Plateau ||
                            feat_b == zoning::LandscapeFeature::Mountain || feat_b == zoning::LandscapeFeature::Plateau) {
                            is_cliff = true;
                            direction = feat_a == zoning::LandscapeFeature::Mountain || feat_a == zoning::LandscapeFeature::Plateau;
                        }
                        // Б) Природные разломы на равнине
                        else if !tile_a.is_ocean && !tile_b.is_ocean && tile_a.faction_id.is_none() && tile_b.faction_id.is_none() {
                            let d_a = *distance_field.get(&coord).unwrap_or(&0) as i32;
                            let d_b = *distance_field.get(&n).unwrap_or(&0) as i32;
                            
                            // Каждые 12 уровней расстояния создаем "террасу" (гораздо реже)
                            if d_a != d_b && (d_a % 12 == 0 || d_b % 12 == 0) {
                                // Шанс появления клиффа с более плавным шумом (0.05 вместо 0.2)
                                let fault_noise = plateau_noise.get([f64::from(coord.q) * 0.05, f64::from(coord.r) * 0.05]);
                                if fault_noise > 0.4 { // Более строгий порог для разреженности
                                    is_cliff = true;
                                    direction = d_a > d_b;
                                }
                            }
                        }

                        if is_cliff {
                            let edge = zoning::EdgeCoord::new(coord, n);
                            new_cliffs.push((edge, zoning::EdgeData {
                                is_cliff: true,
                                direction,
                            }));
                        }
                    }
                }
            }
        }

        for (edge, data) in new_cliffs {
            map_data.edges.insert(edge, data);
        }
    }

    // Auto-spawn Player Start (Faction 1)
    let has_player_start = map_data.tiles.values().any(|t| t.faction_id == Some(1));
    if force_reset || !has_player_start {
        auto_spawn_player_territory(map_data, seed.value());
    }

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
                    TerrainType::Swamp => {
                        cost = 50;
                    }
                    TerrainType::Stony => {
                        cost = 80;
                    }
                    _ => {}
                }
            }
            nav_map.grid.insert(IVec2::new(q, r), cost);
        }
    }

    commands.queue(crate::economy::mesh_gen::SpawnGlobalTerrainCommand {
        map_data: map_data.clone(),
        phase,
        faction_manager: faction_manager.clone(),
        config: (*terrain_config).clone(),
    });
}

#[allow(dead_code)]
fn apply_cave_stamp(map: &mut MapData, x: i32, z: i32) {
    for dx in -1..=1 {
        for dz in -1..=1 {
            if let Some(tile) = map.get_tile_mut(x + dx, z + dz) {
                tile.terrain = TerrainType::Stony;
                tile.roofed = true;
            }
        }
    }
}

fn handle_faction_auto_relocation(
    mut commands: Commands,
    faction_manager: Res<crate::game_state::FactionManager>,
    mut map_data: ResMut<MapData>,
    terrain_gen: Res<TerrainGenerator>,
    config: Res<TerrainConfig>,
    seed: Res<WorldSeed>,
    mut nav_map: ResMut<crate::map::navigation::NavigationMap>,
    phase: Res<State<crate::game_state::EditorPhase>>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    if !map_data.is_changed() { return; }

    let mut changed = false;
    
    // Собираем фракции, которым нужна релокация (размер меньше порога)
    let mut to_relocate = Vec::new();
    for faction in &faction_manager.factions {
        let count = map_data.tiles.values().filter(|t| t.faction_id == Some(faction.id)).count();
        let min_required = if faction.id == 1 { 15 } else { 20 };
        
        if count < min_required {
            to_relocate.push(faction.id);
        }
    }

    for f_id in to_relocate {
        // 1. Очищаем старые остатки территории
        for tile in map_data.tiles.values_mut() {
            if tile.faction_id == Some(f_id) {
                tile.faction_id = None;
            }
        }

        // 2. Спавним на новом месте
        if f_id == 1 {
            auto_spawn_player_territory(&mut map_data, seed.value());
            debug!("RESCUE: Relocated Player Start (Faction 1) - insufficient space.");
        } else {
            auto_spawn_npc_territory(&mut map_data, f_id, seed.value() + f_id);
            debug!("RESCUE: Relocated Faction {} - insufficient space.", f_id);
        }
        changed = true;
    }

    if changed {
        // Динамическая география: Пересчитываем ландшафт с учетом новых позиций фракций
        spawn_map_internal(
            &mut commands,
            &terrain_gen,
            &config,
            &seed,
            &mut map_data,
            &mut nav_map,
            &faction_manager,
            *phase.get(),
            false, // Не сбрасываем ручные правки ландшафта (если будут), но обновляем процедурку
        );
        map_data.run_validation();
        ev_rebuild.write(RebuildMeshEvent);
    }
}

fn auto_spawn_npc_territory(map_data: &mut MapData, faction_id: u32, seed: u32) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(seed));
    
    // Ищем любой свободный Land гекс, который не занят другой фракцией
    let free_land: Vec<_> = map_data.tiles.iter()
        .filter(|(_, t)| !t.is_ocean && t.faction_id.is_none())
        .map(|(c, _)| *c)
        .collect();

    if let Some(start_coord) = free_land.choose(&mut rng) {
        let mut queue = VecDeque::new();
        queue.push_back(*start_coord);
        
        let mut territory = HashSet::new();
        territory.insert(*start_coord);
        
        let target_size = rng.gen_range(20..=30); // NPC фракции теперь большие
        
        while let Some(curr) = queue.pop_front() {
            if territory.len() >= target_size { break; }
            
            let mut neighbors: Vec<_> = curr.neighbors().into_iter().collect();
            neighbors.shuffle(&mut rng);
            
            for n in neighbors {
                if territory.len() >= target_size { break; }
                if let Some(tile) = map_data.tiles.get(&n) {
                    // Проверка на сушу и отсутствие других фракций + правило 1 гекса
                    let mut can_paint = !tile.is_ocean && tile.faction_id.is_none();
                    if can_paint {
                        for nn in n.neighbors() {
                            if let Some(nt) = map_data.tiles.get(&nn) {
                                if let Some(fid) = nt.faction_id {
                                    if fid != faction_id { can_paint = false; break; }
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

fn auto_spawn_player_territory(map_data: &mut MapData, seed: u32) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(seed));
    
    // Ищем подходящий берег (Land гекс, у которого есть сосед Ocean)
    let mut coastal_tiles = Vec::new();
    for (coord, tile) in &map_data.tiles {
        if !tile.is_ocean {
            for neighbor_coord in coord.neighbors() {
                if let Some(neighbor) = map_data.tiles.get(&neighbor_coord) {
                    if neighbor.is_ocean {
                        coastal_tiles.push(*coord);
                        break;
                    }
                }
            }
        }
    }

    if let Some(start_coord) = coastal_tiles.choose(&mut rng) {
        // Создаем пятно (10-15 гексов) через BFS
        let mut queue = VecDeque::new();
        queue.push_back(*start_coord);
        
        let mut territory = HashSet::new();
        territory.insert(*start_coord);
        
        let target_size = rng.gen_range(15..=25); // Игрок тоже побольше
        
        while let Some(curr) = queue.pop_front() {
            if territory.len() >= target_size { break; }
            
            let mut neighbors: Vec<_> = curr.neighbors().into_iter().collect();
            neighbors.shuffle(&mut rng);
            
            for n in neighbors {
                if territory.len() >= target_size { break; }
                if let Some(tile) = map_data.tiles.get(&n) {
                    if !tile.is_ocean && !territory.contains(&n) {
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

fn get_terrain_from_climate(temp: f32, humid: f32, elev: f32) -> TerrainType {
    if elev > 0.8 {
        return TerrainType::Stony;
    }

    if humid > 0.7 {
        if temp < 0.3 {
            TerrainType::Swamp
        } else {
            TerrainType::Mossy
        }
    } else if humid < 0.3 {
        if temp > 0.7 {
            TerrainType::Dusty
        } else {
            TerrainType::Steppe
        }
    } else if temp < 0.3 {
        TerrainType::Stony
    } else {
        TerrainType::Grass
    }
}
