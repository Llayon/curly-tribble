// src/map/face_topology/tests_runtime.rs
//! Integration tests for HexFaceTopology runtime generation and lifecycle.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct RuntimeTestsPlugin;

impl Plugin for RuntimeTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::face_topology::debug::{HexFaceDebugCache, HexFaceDebugSettings};
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::runtime::{
        regenerate_hex_face_topology, HexFaceTopologyGenerationState,
    };
    use crate::map::face_topology::types::HexFaceTopology;
    use crate::map::face_topology::validate_complete_topology;
    use crate::map::{GenerateMapEvent, MapData, RebuildMeshEvent, WorldSeed};
    use bevy::prelude::*;

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
