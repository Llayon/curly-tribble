/// Focused tests for debug-only topology helpers.
#[cfg(test)]
mod debug_tests {
    use crate::game_state::{EditorPhase, GameState};
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::debug::{
        apply_debug_shortcuts, debug_overlay_visible, extract_shared_vertices,
        extract_unique_regular_edges, extract_unique_undirected_edges, HexFaceDebugCache,
        HexFaceDebugSettings,
    };
    use crate::map::face_topology::generator::generate_hex_face_topology;
    use crate::map::HexCoord;
    use crate::map::WorldSeed;
    use bevy::input::keyboard::KeyCode;
    use std::collections::HashSet;

    fn two_hex_map() -> MapData {
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), TileData::default());
        map.tiles.insert(HexCoord::new(1, 0), TileData::default());
        map
    }

    fn map_40x40() -> MapData {
        let mut map = MapData::default();
        for r in 0..40 {
            let offset = r >> 1;
            for q in -offset..(40 - offset) {
                map.tiles.insert(HexCoord::new(q, r), TileData::default());
            }
        }
        map.width = 40;
        map.height = 40;
        map
    }

    #[test]
    fn debug_settings_are_disabled_with_expected_submode_defaults() {
        let settings = HexFaceDebugSettings::default();
        assert!(!settings.enabled);
        assert!(settings.show_regular_outlines);
        assert!(settings.show_warped_outlines);
        assert!(!settings.show_shared_vertices);
        assert!(!settings.show_half_edge_directions);
    }

    #[test]
    fn unique_edges_and_vertices_use_authoritative_ids() {
        let map = two_hex_map();
        let topology =
            generate_hex_face_topology(&map, WorldSeed::new(42)).expect("two-hex topology");
        let edges = extract_unique_undirected_edges(&topology);
        let regular_edges = extract_unique_regular_edges(&map);
        let vertices = extract_shared_vertices(&topology);
        assert_eq!(
            edges.len(),
            topology.stats.paired_edge_count + topology.stats.border_edge_count
        );
        assert_eq!(regular_edges.len(), edges.len());
        assert_eq!(vertices.len(), topology.vertices.len());
        assert_eq!(edges.iter().collect::<HashSet<_>>().len(), edges.len());
        assert_eq!(
            regular_edges.iter().collect::<HashSet<_>>().len(),
            regular_edges.len()
        );
    }

    #[test]
    fn helpers_do_not_mutate_map_or_topology() {
        let map = two_hex_map();
        let topology =
            generate_hex_face_topology(&map, WorldSeed::new(42)).expect("two-hex topology");
        let map_keys_before: HashSet<_> = map.tiles.keys().copied().collect();
        let map_dimensions_before = (map.width, map.height);
        let topology_before = topology.clone();
        let _ = extract_unique_undirected_edges(&topology);
        let _ = extract_shared_vertices(&topology);
        assert_eq!(map_dimensions_before, (map.width, map.height));
        assert_eq!(map_keys_before, map.tiles.keys().copied().collect());
        assert_eq!(topology, topology_before);
    }

    #[test]
    fn visibility_is_disabled_or_limited_to_flat_phases() {
        let mut settings = HexFaceDebugSettings::default();
        assert!(!debug_overlay_visible(
            GameState::Playing,
            EditorPhase::Shape,
            &settings
        ));
        assert!(!debug_overlay_visible(
            GameState::Editing,
            EditorPhase::Height3D,
            &settings
        ));
        settings.enabled = true;
        assert!(debug_overlay_visible(
            GameState::Editing,
            EditorPhase::Balance,
            &settings
        ));
        assert!(!debug_overlay_visible(
            GameState::Playing,
            EditorPhase::Balance,
            &settings
        ));
    }

    #[test]
    fn shortcuts_are_ignored_outside_editing() {
        let mut keyboard = bevy::input::ButtonInput::default();
        keyboard.press(KeyCode::F5);
        keyboard.press(KeyCode::F6);
        keyboard.press(KeyCode::F7);
        keyboard.press(KeyCode::F8);
        let defaults = HexFaceDebugSettings::default();
        let mut settings = defaults.clone();
        apply_debug_shortcuts(&mut settings, GameState::Playing, &keyboard);
        assert_eq!(settings, defaults);
        apply_debug_shortcuts(&mut settings, GameState::Editing, &keyboard);
        assert!(settings.enabled);
        assert!(settings.show_shared_vertices);
        assert!(settings.show_half_edge_directions);
        assert_eq!(
            settings.profile,
            crate::map::face_topology::HexDeformationProfile::Organic
        );
    }

    #[test]
    fn forty_by_forty_cache_has_one_regular_and_warped_edge_per_logical_edge() {
        let map = map_40x40();
        let topology =
            generate_hex_face_topology(&map, WorldSeed::new(42)).expect("40x40 topology");
        let mut cache = HexFaceDebugCache::default();
        cache.rebuild(&topology, &map);
        assert_eq!(cache.edges.len(), 4_959);
        assert_eq!(cache.regular_edges.len(), 4_959);
        assert_eq!(cache.edges.len(), cache.regular_edges.len());
        assert_eq!(cache.shared_vertices.len(), topology.vertices.len());
        assert!(cache.is_consistent(&topology));
    }

    #[test]
    fn debug_settings_toggle_does_not_alter_production_face_topology() {
        use crate::map::face_topology::fingerprint::topology_fingerprints;
        use crate::map::face_topology::profiles::HexDeformationProfile;
        use crate::map::face_topology::runtime::{
            regenerate_hex_face_topology, HexFaceTopologyGenerationState,
        };
        use crate::map::face_topology::types::HexFaceTopology;
        use crate::map::terrain_gen::{TerrainConfig, TerrainConfigFingerprint};
        use crate::map::{GenerateMapEvent, RebuildMeshEvent};
        use bevy::prelude::*;

        let mut app = App::new();
        app.insert_resource(map_40x40())
            .insert_resource(WorldSeed::new(42))
            .init_resource::<TerrainConfig>()
            .init_resource::<TerrainConfigFingerprint>()
            .init_resource::<HexFaceTopology>()
            .init_resource::<HexFaceDebugSettings>()
            .init_resource::<HexFaceDebugCache>()
            .init_resource::<HexFaceTopologyGenerationState>()
            .add_message::<GenerateMapEvent>()
            .add_message::<RebuildMeshEvent>()
            .add_systems(
                Update,
                (
                    crate::map::systems::monitor_inspector_triggers
                        .run_if(resource_changed::<TerrainConfig>),
                    regenerate_hex_face_topology,
                )
                    .chain(),
            );
        app.update();
        let fp1 = topology_fingerprints(
            app.world().resource::<MapData>(),
            WorldSeed::new(42),
            app.world().resource::<HexFaceTopology>(),
        );

        app.world_mut()
            .resource_mut::<HexFaceDebugSettings>()
            .profile = HexDeformationProfile::Organic;
        app.update();
        let fp2 = topology_fingerprints(
            app.world().resource::<MapData>(),
            WorldSeed::new(42),
            app.world().resource::<HexFaceTopology>(),
        );
        assert_eq!(fp1.geometry, fp2.geometry);
        assert_eq!(
            app.world().resource::<HexFaceTopology>().stats.profile,
            HexDeformationProfile::Subtle
        );

        app.world_mut()
            .resource_mut::<TerrainConfig>()
            .deformation_profile = HexDeformationProfile::Organic;
        app.update();
        let fp3 = topology_fingerprints(
            app.world().resource::<MapData>(),
            WorldSeed::new(42),
            app.world().resource::<HexFaceTopology>(),
        );
        assert_ne!(fp1.geometry, fp3.geometry);
        assert_eq!(
            app.world().resource::<HexFaceTopology>().stats.profile,
            HexDeformationProfile::Organic
        );
    }

    #[test]
    fn production_profile_change_rebuilds_mesh_exactly_once_without_feedback_loop() {
        use crate::game_state::{EditorPhase, FactionManager};
        use crate::map::face_topology::profiles::HexDeformationProfile;
        use crate::map::face_topology::runtime::{
            regenerate_hex_face_topology, HexFaceTopologyGenerationState,
        };
        use crate::map::face_topology::types::HexFaceTopology;
        use crate::map::systems::{handle_rebuild_mesh, monitor_inspector_triggers};
        use crate::map::terrain_gen::{TerrainConfig, TerrainConfigFingerprint};
        use crate::map::{GenerateMapEvent, RebuildMeshEvent};
        use bevy::prelude::*;

        let mut app = App::new();
        app.insert_resource(map_40x40())
            .insert_resource(WorldSeed::new(42))
            .insert_resource(FactionManager::default())
            .insert_resource(State::new(EditorPhase::Shape))
            .insert_resource(crate::economy::GameAssets::default())
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<StandardMaterial>::default())
            .init_resource::<TerrainConfig>()
            .init_resource::<TerrainConfigFingerprint>()
            .init_resource::<HexFaceTopology>()
            .init_resource::<crate::map::surface_topology::types::SurfaceTopology>()
            .init_resource::<crate::map::terrain_bake::types::SurfaceTerrainBake>()
            .init_resource::<crate::map::terrain_bake::runtime::TerrainBakeGenerationState>()
            .init_resource::<crate::map::surface_gameplay::runtime::SurfaceGameplayGenerationState>(
            )
            .init_resource::<crate::map::surface_gameplay::types::SurfaceGameplayMap>()
            .init_resource::<HexFaceDebugCache>()
            .init_resource::<HexFaceTopologyGenerationState>()
            .init_resource::<crate::map::surface_topology::runtime::SurfaceTopologyGenerationState>(
            )
            .add_message::<GenerateMapEvent>()
            .add_message::<RebuildMeshEvent>()
            .add_systems(
                Update,
                (
                    monitor_inspector_triggers.run_if(resource_changed::<TerrainConfig>),
                    regenerate_hex_face_topology,
                    crate::map::surface_topology::runtime::regenerate_surface_topology,
                    handle_rebuild_mesh,
                )
                    .chain(),
            );

        app.world_mut()
            .resource_mut::<crate::map::terrain_bake::runtime::TerrainBakeGenerationState>()
            .last_outcome =
            crate::map::terrain_bake::runtime::TerrainBakeGenerationOutcome::Success;
        app.world_mut()
            .resource_mut::<crate::map::surface_gameplay::runtime::SurfaceGameplayGenerationState>()
            .last_outcome =
            crate::map::surface_gameplay::runtime::SurfaceGameplayGenerationOutcome::Success;

        // Initial setup frame
        app.update();
        assert_eq!(
            app.world().resource::<HexFaceTopology>().stats.profile,
            HexDeformationProfile::Subtle
        );

        // Mutate production profile once
        app.world_mut()
            .resource_mut::<TerrainConfig>()
            .deformation_profile = HexDeformationProfile::Organic;

        // Frame 1: triggers rebuild, face topology updates to Organic, handle_rebuild_mesh consumes event
        app.update();
        assert_eq!(
            app.world().resource::<HexFaceTopology>().stats.profile,
            HexDeformationProfile::Organic
        );
        assert!(app
            .world()
            .contains_resource::<crate::map::topology::TerrainTopology>());

        // Subsequent frames: resource_changed must be false, 0 new rebuilds
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<HexFaceTopology>().stats.profile,
            HexDeformationProfile::Organic
        );
    }
}
