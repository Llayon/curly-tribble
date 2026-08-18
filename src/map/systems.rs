use crate::events::{GameLogMessage, LogSeverity};
use crate::game_state::{EditorPhase, FactionManager};
use crate::map::generation::{
    auto_spawn_bio_deposits, auto_spawn_npcs, auto_spawn_treasures, spawn_map_internal,
};
use crate::map::surface_gameplay::runtime::{
    SurfaceGameplayGenerationOutcome, SurfaceGameplayGenerationState,
};
use crate::map::surface_gameplay::types::SurfaceGameplayMap;
use crate::map::terrain_bake::runtime::{TerrainBakeGenerationOutcome, TerrainBakeGenerationState};
use crate::map::terrain_bake::types::SurfaceTerrainBake;
use crate::map::terrain_gen::{
    GenerationRequest, TerrainConfig, TerrainConfigFingerprint, TerrainGenerator,
};
use crate::map::{
    FactionMarker, GenerateMapEvent, GenerationMode, MapData, MapEntity, RebuildMeshEvent,
    WorldSeed,
};
use bevy::prelude::*;
use rand::Rng;

pub struct MapSystemsPlugin;

impl Plugin for MapSystemsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn handle_regeneration(
    mut commands: Commands,
    mut ev_gen: MessageReader<GenerateMapEvent>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
    q_map_entities: Query<Entity, With<MapEntity>>,
    q_faction_markers: Query<Entity, With<FactionMarker>>,
    q_mines: Query<Entity, With<super::mines::MineDeposit>>,
    q_treasures: Query<Entity, With<crate::map::treasures::TreasureDeposit>>,
    config: Res<TerrainConfig>,
    mut seed: ResMut<WorldSeed>,
    mut terrain_gen: ResMut<TerrainGenerator>,
    mut map_data: ResMut<MapData>,
    mut log_writer: MessageWriter<GameLogMessage>,
    faction_manager: Res<FactionManager>,
    phase: Res<State<EditorPhase>>,
) {
    for ev in ev_gen.read() {
        let reset = ev.mode == GenerationMode::Reset;
        debug!(
            "MAP_GEN: Received GenerateMapEvent ({:?}, AutoFill: {:?}). Starting cleanup...",
            ev.mode, ev.auto_fill_phase
        );

        if reset {
            for entity in &q_map_entities {
                commands.entity(entity).despawn();
            }
            for entity in &q_faction_markers {
                commands.entity(entity).despawn();
            }
            *seed = WorldSeed::new(config.seed);
            *terrain_gen = TerrainGenerator::new(config.seed);
            map_data.width = config.map_width;
            map_data.height = config.map_height;
        }

        spawn_map_internal(
            &terrain_gen,
            &config,
            &seed,
            &mut map_data,
            *phase.get(),
            ev.mode,
            ev.auto_fill_phase,
        );

        if ev.auto_fill_phase == Some(EditorPhase::Factions) {
            super::generation::spawn_factions(&mut commands, &map_data, &faction_manager);
        }

        if ev.auto_fill_phase == Some(EditorPhase::NPCs) {
            auto_spawn_npcs(&mut commands, &map_data, &faction_manager, seed.value());
        }

        if ev.auto_fill_phase == Some(EditorPhase::Plants) {
            auto_spawn_bio_deposits(&mut commands, &map_data, seed.value());
        }

        if ev.auto_fill_phase == Some(EditorPhase::Treasures) {
            for entity in &q_treasures {
                commands.entity(entity).despawn();
            }
            auto_spawn_treasures(&mut commands, &map_data, seed.value());
        }

        if ev.auto_fill_phase == Some(EditorPhase::Mines) {
            for entity in &q_mines {
                commands.entity(entity).despawn();
            }
            super::mines::auto_spawn_mines(&mut commands, &map_data, seed.value());
        }

        crate::map::validation::run_map_validation(&mut map_data, *phase.get());

        ev_rebuild.write(RebuildMeshEvent);

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
    map_data: Res<MapData>,
    face_topology: Res<crate::map::face_topology::types::HexFaceTopology>,
    bake_state: Res<TerrainBakeGenerationState>,
    gameplay_state: Res<SurfaceGameplayGenerationState>,
    faction_manager: Res<FactionManager>,
    config: Res<TerrainConfig>,
    phase: Res<State<EditorPhase>>,
    bake: Res<SurfaceTerrainBake>,
    gameplay: Res<SurfaceGameplayMap>,
) {
    if ev_rebuild.read().count() == 0 {
        return;
    }

    // Fail-closed: the ONLY authoritative sources of ground geometry and
    // buildability are a successfully generated SurfaceTerrainBake and a
    // successfully generated SurfaceGameplayMap. A missing/empty/failed
    // bake or gameplay layer must never fall back to legacy topology — old
    // terrain stays in place.
    if bake_state.last_outcome != TerrainBakeGenerationOutcome::Success
        || gameplay_state.last_outcome != SurfaceGameplayGenerationOutcome::Success
    {
        return;
    }

    if !map_data.tiles.is_empty() && face_topology.faces.is_empty() {
        bevy::log::tracing::event!(
            bevy::log::tracing::Level::ERROR,
            "Cannot rebuild mesh: map is non-empty but face topology generation failed"
        );
        return;
    }

    match crate::map::terrain_bake::derive_terrain_topology_from_bake(&bake) {
        Ok(derived_topology) => {
            commands.queue(crate::economy::mesh_gen::SpawnGlobalTerrainCommand {
                topology: derived_topology,
                face_topology: (*face_topology).clone(),
                map_data: map_data.clone(),
                phase: *phase.get(),
                faction_manager: faction_manager.clone(),
                config: (*config).clone(),
                bake: (*bake).clone(),
                gameplay: (*gameplay).clone(),
            });
        }
        Err(err) => {
            bevy::log::tracing::event!(
                bevy::log::tracing::Level::ERROR,
                error = ?err,
                "Failed to derive terrain topology from SurfaceTerrainBake"
            );
        }
    }
}

pub fn monitor_inspector_triggers(
    mut config: ResMut<TerrainConfig>,
    mut ev_gen: MessageWriter<GenerateMapEvent>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
    mut fingerprint: ResMut<TerrainConfigFingerprint>,
) {
    let current = config.mesh_fingerprint();
    if fingerprint.last == current {
        return;
    }
    fingerprint.last = current;

    let request = config.generation_request;
    if request == GenerationRequest::None {
        ev_rebuild.write(RebuildMeshEvent);
        return;
    }

    let bypass = config.bypass_change_detection();
    bypass.generation_request = GenerationRequest::None;

    if request == GenerationRequest::RandomizeSeed {
        bypass.seed = rand::thread_rng().gen_range(0..999_999);
    }

    fingerprint.last = config.mesh_fingerprint();

    ev_gen.write(GenerateMapEvent {
        mode: GenerationMode::Reset,
        auto_fill_phase: None,
    });
}

pub fn handle_faction_auto_relocation(
    faction_manager: Res<FactionManager>,
    mut map_data: ResMut<MapData>,
    terrain_gen: Res<TerrainGenerator>,
    config: Res<TerrainConfig>,
    seed: Res<WorldSeed>,
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
            &terrain_gen,
            &config,
            &seed,
            &mut map_data,
            *phase.get(),
            GenerationMode::Preserve,
            None,
        );
        crate::map::validation::run_map_validation(&mut map_data, *phase.get());
        ev_rebuild.write(RebuildMeshEvent);
    }
}
