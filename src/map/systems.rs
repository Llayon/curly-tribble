use crate::events::{GameLogMessage, LogSeverity};
use crate::game_state::{EditorPhase, FactionManager};
use crate::map::generation::{auto_spawn_npcs, spawn_map_internal};
use crate::map::navigation::NavigationMap;
use crate::map::terrain_gen::{TerrainConfig, TerrainGenerator};
use crate::map::{MapData, FactionMarker, WorldSeed, MapEntity, GenerateMapEvent, RebuildMeshEvent};
use bevy::prelude::*;
use rand::Rng;

pub fn handle_regeneration(
    mut commands: Commands,
    mut ev_gen: MessageReader<GenerateMapEvent>,
    q_map_entities: Query<Entity, With<MapEntity>>,
    q_faction_markers: Query<Entity, With<FactionMarker>>,
    config: Res<TerrainConfig>,
    mut seed: ResMut<WorldSeed>,
    mut terrain_gen: ResMut<TerrainGenerator>,
    mut map_data: ResMut<MapData>,
    mut nav_map: ResMut<NavigationMap>,
    mut log_writer: MessageWriter<GameLogMessage>,
    faction_manager: Res<FactionManager>,
    phase: Res<State<EditorPhase>>,
) {
    for ev in ev_gen.read() {
        debug!("MAP_GEN: Received GenerateMapEvent (ForceReset: {}, AutoFill: {:?}). Starting cleanup...", ev.force_reset, ev.auto_fill_phase);

        if ev.force_reset {
            for entity in &q_map_entities {
                commands.entity(entity).despawn();
            }
            for entity in &q_faction_markers {
                commands.entity(entity).despawn();
            }
            nav_map.grid.clear();
            *seed = WorldSeed::new(config.seed);
            *terrain_gen = TerrainGenerator::new(config.seed);
            map_data.width = config.map_width;
            map_data.height = config.map_height;
        }

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
            ev.auto_fill_phase,
        );

        if ev.force_reset || ev.auto_fill_phase == Some(EditorPhase::Factions) {
            super::generation::spawn_factions(&mut commands, &map_data, &faction_manager);
        }

        if ev.force_reset || ev.auto_fill_phase == Some(EditorPhase::NPCs) {
            auto_spawn_npcs(&mut commands, &map_data, &faction_manager, seed.value());
        }

        crate::map::validation::run_map_validation(&mut map_data);

        log_writer.write(GameLogMessage {
            message: format!(
                "World regenerated: {}x{}, seed {}",
                config.map_width, config.map_height, config.seed
            ),
            severity: LogSeverity::Info,
        });
    }
}

pub fn handle_rebuild_mesh(
    mut commands: Commands,
    mut ev_rebuild: MessageReader<RebuildMeshEvent>,
    q_map_entities: Query<Entity, With<MapEntity>>,
    map_data: Res<MapData>,
    faction_manager: Res<FactionManager>,
    config: Res<TerrainConfig>,
    phase: Res<State<EditorPhase>>,
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

pub fn monitor_inspector_triggers(
    mut config: ResMut<TerrainConfig>,
    mut ev_gen: MessageWriter<GenerateMapEvent>,
) {
    let mut triggered = false;
    if config.randomize_seed {
        config.seed = rand::thread_rng().gen_range(0..999_999);
        config.randomize_seed = false;
        triggered = true;
    }
    if config.regenerate_world {
        config.regenerate_world = false;
        triggered = true;
    }
    if triggered {
        ev_gen.write(GenerateMapEvent {
            force_reset: true,
            auto_fill_phase: None,
        });
    }
}

pub fn handle_faction_auto_relocation(
    mut commands: Commands,
    faction_manager: Res<FactionManager>,
    mut map_data: ResMut<MapData>,
    terrain_gen: Res<TerrainGenerator>,
    config: Res<TerrainConfig>,
    seed: Res<WorldSeed>,
    mut nav_map: ResMut<NavigationMap>,
    phase: Res<State<EditorPhase>>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    if !map_data.is_changed() {
        return;
    }
    let mut changed = false;
    let mut to_relocate = Vec::new();
    for faction in &faction_manager.factions {
        let count = map_data
            .tiles
            .values()
            .filter(|t| t.faction_id == Some(faction.id))
            .count();
        let min_required = if faction.id == 1 { 15 } else { 20 };
        if count < min_required {
            to_relocate.push(faction.id);
        }
    }

    for f_id in to_relocate {
        for tile in map_data.tiles.values_mut() {
            if tile.faction_id == Some(f_id) {
                tile.faction_id = None;
            }
        }
        if f_id == 1 {
            super::generation::auto_spawn_player_territory(&mut map_data, seed.value());
        } else {
            super::generation::auto_spawn_npc_territory(&mut map_data, f_id, seed.value() + f_id);
        }
        changed = true;
    }

    if changed {
        spawn_map_internal(
            &mut commands,
            &terrain_gen,
            &config,
            &seed,
            &mut map_data,
            &mut nav_map,
            &faction_manager,
            *phase.get(),
            false,
            None,
        );
        crate::map::validation::run_map_validation(&mut map_data);
        ev_rebuild.write(RebuildMeshEvent);
    }
}
