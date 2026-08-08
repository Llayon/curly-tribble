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
        }
    }
}
