//! Authoritative runtime population of the data-only face topology resource.
use crate::map::face_topology::debug::HexFaceDebugCache;
use crate::map::face_topology::generate_hex_face_topology;
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
}

impl LogicalMapInputs {
    #[must_use]
    pub fn from_map(map_data: &MapData, world_seed: WorldSeed) -> Self {
        let mut tiles: Vec<_> = map_data.tiles.keys().copied().collect();
        tiles.sort_by_key(|coord| (coord.q, coord.r));
        Self {
            width: map_data.width,
            height: map_data.height,
            seed: world_seed.value(),
            tiles,
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct HexFaceTopologyGenerationState {
    pub last_inputs: Option<LogicalMapInputs>,
    pub last_successful_inputs: Option<LogicalMapInputs>,
    pub generation_count: u64,
    pub failure_count: u64,
}

pub struct FaceTopologyRuntimePlugin;

impl Plugin for FaceTopologyRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HexFaceTopologyGenerationState>()
            .add_systems(
                Update,
                regenerate_hex_face_topology.in_set(GameSet::Visuals),
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
    mut generation_events: MessageReader<GenerateMapEvent>,
    mut rebuild_events: MessageReader<RebuildMeshEvent>,
) {
    let event_requested =
        generation_events.read().next().is_some() || rebuild_events.read().next().is_some();
    let input_may_have_changed =
        map_data.is_changed() || world_seed.is_changed() || generation_state.last_inputs.is_none();
    if !event_requested && !input_may_have_changed {
        return;
    }

    let inputs = LogicalMapInputs::from_map(&map_data, *world_seed);
    if generation_state.last_inputs.as_ref() == Some(&inputs) {
        return;
    }
    generation_state.last_inputs = Some(inputs.clone());

    match generate_hex_face_topology(&map_data, *world_seed) {
        Ok(new_topology) => {
            let unique_edge_count =
                crate::map::face_topology::debug::extract_unique_undirected_edges(&new_topology)
                    .len();
            bevy::log::tracing::event!(
                bevy::log::tracing::Level::INFO,
                seed = world_seed.value(),
                tiles = map_data.tiles.len(),
                width = map_data.width,
                height = map_data.height,
                faces = new_topology.faces.len(),
                vertices = new_topology.vertices.len(),
                half_edges = new_topology.half_edges.len(),
                paired_edges = new_topology.stats.paired_edge_count,
                border_edges = new_topology.stats.border_edge_count,
                unique_debug_edges = unique_edge_count,
                reduced_displacements = new_topology.stats.reduced_displacement_fallbacks,
                regular_fallbacks = new_topology.stats.regular_position_fallbacks,
                "HexFaceTopology regenerated"
            );
            debug_cache.rebuild(&new_topology);
            topology.clone_from(&new_topology);
            generation_state.last_successful_inputs = Some(inputs);
            generation_state.generation_count += 1;
        }
        Err(error) => {
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
                error = ?error,
                "HexFaceTopology generation failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            .init_resource::<HexFaceTopology>()
            .init_resource::<HexFaceDebugCache>()
            .init_resource::<HexFaceTopologyGenerationState>()
            .add_message::<GenerateMapEvent>()
            .add_message::<RebuildMeshEvent>()
            .add_systems(Update, regenerate_hex_face_topology);
        app
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
        assert_eq!(
            app.world()
                .resource::<HexFaceTopologyGenerationState>()
                .generation_count,
            1
        );
        app.update();
        assert_eq!(
            app.world()
                .resource::<HexFaceTopologyGenerationState>()
                .generation_count,
            1
        );
        app.world_mut().insert_resource(WorldSeed::new(99));
        app.update();
        let second = app.world().resource::<HexFaceTopology>();
        assert_ne!(first.vertices, second.vertices);
        assert_eq!(
            app.world()
                .resource::<HexFaceTopologyGenerationState>()
                .generation_count,
            2
        );
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
        assert_eq!(
            app.world()
                .resource::<HexFaceTopologyGenerationState>()
                .generation_count,
            1
        );
        app.world_mut().resource_mut::<MapData>().tiles.insert(
            crate::map::HexCoord::new(2, 0),
            crate::map::TileData::default(),
        );
        app.update();
        assert_eq!(
            app.world()
                .resource::<HexFaceTopologyGenerationState>()
                .generation_count,
            2
        );
    }

    #[test]
    fn failed_generation_clears_once_and_does_not_store_partial_data() {
        let mut app = test_app(MapData::default(), 42);
        app.update();
        assert!(app.world().resource::<HexFaceTopology>().faces.is_empty());
        assert_eq!(
            app.world()
                .resource::<HexFaceTopologyGenerationState>()
                .failure_count,
            1
        );
        app.update();
        assert_eq!(
            app.world()
                .resource::<HexFaceTopologyGenerationState>()
                .failure_count,
            1
        );
    }
}
