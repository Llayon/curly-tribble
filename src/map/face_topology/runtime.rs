//! Authoritative runtime population of the data-only face topology resource.
use crate::map::face_topology::acceptance::warn_on_acceptance_misses;
use crate::map::face_topology::cache::HexFaceDebugCache;
use crate::map::face_topology::edge_binding::{self, BoundCliffEdges};
use crate::map::face_topology::fingerprint::topology_fingerprints;
use crate::map::face_topology::generate_hex_face_topology_with_profile;
use crate::map::face_topology::profiles::HexDeformationProfile;
use crate::map::face_topology::types::HexFaceTopology;
use crate::map::{GenerateMapEvent, MapData, RebuildMeshEvent, WorldSeed};
use crate::sets::GameSet;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalMapInputs {
    pub width: u32,
    pub height: u32,
    pub seed: u32,
    pub tiles: Vec<crate::map::HexCoord>,
    pub profile: HexDeformationProfile,
}

impl LogicalMapInputs {
    #[must_use]
    pub fn from_map(
        map_data: &MapData,
        world_seed: WorldSeed,
        profile: HexDeformationProfile,
    ) -> Self {
        let mut tiles: Vec<_> = map_data.tiles.keys().copied().collect();
        tiles.sort_by_key(|coord| (coord.q, coord.r));
        Self {
            width: map_data.width,
            height: map_data.height,
            seed: world_seed.value(),
            tiles,
            profile,
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct HexFaceTopologyGenerationState {
    pub last_inputs: Option<LogicalMapInputs>,
    pub last_successful_inputs: Option<LogicalMapInputs>,
    pub generation_count: u64,
    pub failure_count: u64,
    pub generation_events_consumed: u64,
    pub rebuild_events_consumed: u64,
}

pub struct FaceTopologyRuntimePlugin;

impl Plugin for FaceTopologyRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HexFaceTopologyGenerationState>()
            .init_resource::<edge_binding::BoundCliffEdges>()
            .add_systems(
                Update,
                (
                    regenerate_hex_face_topology,
                    rebuild_bound_cliff_edges.after(regenerate_hex_face_topology),
                )
                    .chain()
                    .in_set(GameSet::Visuals),
            );
    }
}

/// Regenerates the authoritative resource only when logical inputs differ.
pub fn regenerate_hex_face_topology(
    mut topology: ResMut<HexFaceTopology>,
    mut debug_cache: ResMut<HexFaceDebugCache>,
    mut generation_state: ResMut<HexFaceTopologyGenerationState>,
    map_data: Res<MapData>,
    world_seed: Res<WorldSeed>,
    terrain_config: Option<Res<crate::map::terrain_gen::TerrainConfig>>,
    mut generation_events: MessageReader<GenerateMapEvent>,
    mut rebuild_events: MessageReader<RebuildMeshEvent>,
) {
    let generation_events_consumed = generation_events.read().count();
    let rebuild_events_consumed = rebuild_events.read().count();
    generation_state.generation_events_consumed += generation_events_consumed as u64;
    generation_state.rebuild_events_consumed += rebuild_events_consumed as u64;
    let event_requested = generation_events_consumed > 0 || rebuild_events_consumed > 0;
    let profile = terrain_config
        .as_ref()
        .map_or(HexDeformationProfile::Subtle, |c| c.deformation_profile);
    let input_may_have_changed = map_data.is_changed()
        || world_seed.is_changed()
        || terrain_config
            .as_ref()
            .is_some_and(bevy::prelude::DetectChanges::is_changed)
        || generation_state.last_inputs.is_none();
    if !event_requested && !input_may_have_changed {
        return;
    }

    let inputs = LogicalMapInputs::from_map(&map_data, *world_seed, profile);
    if generation_state.last_inputs.as_ref() == Some(&inputs) {
        return;
    }
    generation_state.last_inputs = Some(inputs.clone());

    match generate_hex_face_topology_with_profile(&map_data, *world_seed, profile) {
        Ok(new_topology) => {
            let mut new_cache = HexFaceDebugCache::default();
            new_cache.rebuild(&new_topology, &map_data);
            if !new_cache.is_consistent(&new_topology) {
                generation_state.failure_count += 1;
                if generation_state.last_successful_inputs.as_ref() != Some(&inputs) {
                    *topology = HexFaceTopology::default();
                    debug_cache.clear();
                }
                bevy::log::tracing::event!(
                    bevy::log::tracing::Level::ERROR,
                    seed = world_seed.value(),
                    tiles = map_data.tiles.len(),
                    "HexFaceTopology debug cache consistency failed"
                );
                return;
            }
            let unique_edge_count = new_cache.edges.len();
            bevy::log::tracing::event!(
                bevy::log::tracing::Level::INFO,
                seed = world_seed.value(),
                tiles = map_data.tiles.len(),
                faces = new_topology.faces.len(),
                vertices = new_topology.vertices.len(),
                half_edges = new_topology.half_edges.len(),
                unique_debug_edges = unique_edge_count,
                unique_regular_edges = new_cache.regular_edges.len(),
                profile = profile.name(),
                stats = ?new_topology.stats,
                "HexFaceTopology regenerated"
            );
            *debug_cache = new_cache;
            topology.clone_from(&new_topology);
            let fingerprint = topology_fingerprints(&map_data, *world_seed, &new_topology);
            warn_on_acceptance_misses(&topology, profile, fingerprint.geometry);
            generation_state.last_successful_inputs = Some(inputs);
            generation_state.generation_count += 1;
        }
        Err(_error) => {
            generation_state.failure_count += 1;
            if generation_state.last_successful_inputs.as_ref() != Some(&inputs) {
                *topology = HexFaceTopology::default();
                debug_cache.clear();
            }
            bevy::log::tracing::event!(
                bevy::log::tracing::Level::ERROR,
                seed = world_seed.value(),
                tiles = map_data.tiles.len(),
                width = map_data.width,
                height = map_data.height,
                "HexFaceTopology generation failed"
            );
        }
    }
}

/// Derives `BoundCliffEdges` runtime state from `MapData` and `HexFaceTopology`.
pub fn rebuild_bound_cliff_edges(
    map_data: Res<MapData>,
    face_topology: Res<HexFaceTopology>,
    mut bound_cliff_edges: ResMut<BoundCliffEdges>,
) {
    if !map_data.is_changed() && !face_topology.is_changed() && !bound_cliff_edges.is_added() {
        return;
    }

    match edge_binding::bind_cliff_edges(&map_data, &face_topology) {
        Ok(bound) => {
            *bound_cliff_edges = bound;
        }
        Err(err) => {
            bevy::log::tracing::event!(
                bevy::log::tracing::Level::ERROR,
                error = ?err,
                "Failed to bind cliff edges to topology"
            );
            *bound_cliff_edges = BoundCliffEdges::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::face_topology::debug::HexFaceDebugSettings;
    use crate::map::face_topology::validate_complete_topology;

    fn map_with_tiles(count: i32) -> MapData {
        let mut map = MapData::default();
        for q in 0..count {
            map.tiles.insert(
                crate::map::HexCoord::new(q, 0),
                crate::map::TileData::default(),
            );
        }
        map.width = count as u32;
        map.height = 1;
        map
    }

    fn test_app(map: MapData, seed: u32) -> App {
        let mut app = App::new();
        app.insert_resource(map)
            .insert_resource(WorldSeed::new(seed))
            .init_resource::<crate::map::terrain_gen::TerrainConfig>()
            .init_resource::<HexFaceTopology>()
            .init_resource::<HexFaceDebugSettings>()
            .init_resource::<HexFaceDebugCache>()
            .init_resource::<HexFaceTopologyGenerationState>()
            .add_message::<GenerateMapEvent>()
            .add_message::<RebuildMeshEvent>()
            .add_systems(Update, regenerate_hex_face_topology);
        app
    }

    fn count(app: &App) -> u64 {
        app.world()
            .resource::<HexFaceTopologyGenerationState>()
            .generation_count
    }

    fn fail_count(app: &App) -> u64 {
        app.world()
            .resource::<HexFaceTopologyGenerationState>()
            .failure_count
    }
    #[test]
    fn valid_map_populates_and_validates_stored_topology() {
        let map = map_with_tiles(2);
        let mut app = test_app(map, 42);
        app.update();
        let topology = app.world().resource::<HexFaceTopology>();
        validate_complete_topology(topology, app.world().resource::<MapData>())
            .expect("stored topology must validate");
        assert_eq!(topology.faces.len(), 2);
    }

    #[test]
    fn same_inputs_do_not_regenerate_and_seed_change_does() {
        let mut app = test_app(map_with_tiles(2), 42);
        app.update();
        let first = app.world().resource::<HexFaceTopology>().clone();
        assert_eq!(count(&app), 1);
        app.update();
        assert_eq!(count(&app), 1);
        app.world_mut().insert_resource(WorldSeed::new(99));
        app.update();
        let second = app.world().resource::<HexFaceTopology>();
        assert_ne!(first.vertices, second.vertices);
        assert_eq!(count(&app), 2);
    }

    #[test]
    fn content_change_does_not_regenerate_but_membership_change_does() {
        let mut app = test_app(map_with_tiles(2), 42);
        app.update();
        app.world_mut()
            .resource_mut::<MapData>()
            .tiles
            .get_mut(&crate::map::HexCoord::new(0, 0))
            .map(|tile| tile.faction_id = Some(7));
        app.update();
        assert_eq!(count(&app), 1);
        app.world_mut().resource_mut::<MapData>().tiles.insert(
            crate::map::HexCoord::new(2, 0),
            crate::map::TileData::default(),
        );
        app.update();
        assert_eq!(count(&app), 2);
    }

    #[test]
    fn failed_generation_clears_once_and_does_not_store_partial_data() {
        let mut app = test_app(MapData::default(), 42);
        app.update();
        assert!(app.world().resource::<HexFaceTopology>().faces.is_empty());
        assert_eq!(fail_count(&app), 1);
        app.update();
        assert_eq!(fail_count(&app), 1);
    }

    #[test]
    fn event_burst_drains_both_readers_and_regenerates_once() {
        let mut app = test_app(map_with_tiles(2), 42);
        for _ in 0..3 {
            app.world_mut().write_message(GenerateMapEvent {
                mode: crate::map::GenerationMode::Preserve,
                auto_fill_phase: None,
            });
        }
        for _ in 0..4 {
            app.world_mut().write_message(RebuildMeshEvent);
        }

        app.update();
        let state = app.world().resource::<HexFaceTopologyGenerationState>();
        assert_eq!(state.generation_events_consumed, 3);
        assert_eq!(state.rebuild_events_consumed, 4);
        assert_eq!(state.generation_count, 1);

        app.update();
        let state = app.world().resource::<HexFaceTopologyGenerationState>();
        assert_eq!(state.generation_events_consumed, 3);
        assert_eq!(state.rebuild_events_consumed, 4);
        assert_eq!(state.generation_count, 1);
    }

    #[test]
    fn profile_change_regenerates_once_without_changing_map_data() {
        let mut app = test_app(map_with_tiles(2), 42);
        app.update();
        app.world_mut()
            .resource_mut::<crate::map::terrain_gen::TerrainConfig>()
            .deformation_profile = HexDeformationProfile::Organic;
        app.update();
        let state = app.world().resource::<HexFaceTopologyGenerationState>();
        assert_eq!(state.generation_count, 2);
        assert_eq!(
            app.world().resource::<HexFaceTopology>().stats.profile,
            HexDeformationProfile::Organic
        );
    }
}
