// src/map/surface_gameplay/tests_runtime.rs
//! Runtime lifecycle tests for `regenerate_surface_gameplay`.
//!
//! Tests drive the REAL M6 runtime through honest config / terrain
//! transitions (valid → terrain change → config change) instead of
//! fingerprint injection.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{OceanState, TerrainType, TileData, WorldSeed};
    use crate::map::surface_gameplay::runtime::{
        SurfaceGameplayGenerationOutcome, SurfaceGameplayGenerationState,
    };
    use crate::map::surface_gameplay::types::{SurfaceGameplayMap, SurfaceMetricField};
    use crate::map::{HexCoord, MapData, RebuildMeshEvent};
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceGameplayRuntimeTestsPlugin;

    impl Plugin for SurfaceGameplayRuntimeTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    #[derive(Resource, Default)]
    struct RebuildEventCounter(u64);

    fn collect_rebuild_events(
        mut ev_rebuild: MessageReader<RebuildMeshEvent>,
        mut count: ResMut<RebuildEventCounter>,
    ) {
        count.0 += ev_rebuild.read().count() as u64;
    }

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::map::face_topology::FaceTopologyPlugin,
            crate::map::surface_topology::SurfaceTopologyPlugin,
            crate::map::height_constraints::HeightConstraintsPlugin,
            crate::map::height_graph::HeightGraphPlugin,
            crate::map::surface_height::SurfaceHeightPlugin,
            crate::map::terrain_bake::TerrainBakePlugin,
            crate::map::surface_gameplay::SurfaceGameplayPlugin,
        ));
        app.add_message::<crate::map::GenerateMapEvent>();
        app.add_message::<RebuildMeshEvent>();
        app.insert_resource(WorldSeed::new(42));
        app.init_resource::<MapData>();
        app.init_resource::<RebuildEventCounter>();
        app.add_systems(
            Update,
            collect_rebuild_events
                .after(crate::map::surface_gameplay::runtime::regenerate_surface_gameplay),
        );
        app
    }

    fn insert_land_tile(app: &mut App, q: i32, r: i32, elevation: f32) {
        app.world_mut()
            .get_resource_mut::<MapData>()
            .expect("MapData must exist in test world")
            .tiles
            .insert(
                HexCoord::new(q, r),
                TileData {
                    ocean_state: OceanState::Land,
                    elevation,
                    ..Default::default()
                },
            );
    }

    /// Resources start empty and state uninitialized before any frame.
    #[test]
    fn gameplay_starts_default_and_uninitialized() {
        let app = setup_app();

        assert!(app
            .world()
            .resource::<SurfaceMetricField>()
            .cells
            .is_empty());
        assert!(app
            .world()
            .resource::<SurfaceGameplayMap>()
            .cells
            .is_empty());
        assert_eq!(
            app.world()
                .resource::<SurfaceGameplayGenerationState>()
                .last_outcome,
            SurfaceGameplayGenerationOutcome::Uninitialized
        );
    }

    /// Bake success → gameplay Success with populated resources → rebuild event.
    #[test]
    fn bake_success_builds_gameplay_and_writes_rebuild_event() {
        let mut app = setup_app();
        insert_land_tile(&mut app, 0, 0, 0.5);
        app.update();

        let state = app.world().resource::<SurfaceGameplayGenerationState>();
        assert_eq!(
            state.last_outcome,
            SurfaceGameplayGenerationOutcome::Success
        );
        assert_eq!(state.generation_count, 1);
        assert_eq!(state.failure_count, 0);

        let field = app.world().resource::<SurfaceMetricField>();
        assert!(
            !field.cells.is_empty(),
            "metrics must be populated on success"
        );

        let gameplay = app.world().resource::<SurfaceGameplayMap>();
        assert!(!gameplay.cells.is_empty(), "gameplay must be populated");

        let events = app.world().resource::<RebuildEventCounter>();
        assert_eq!(
            events.0, 2,
            "one bake cycle must write 2 RebuildMeshEvents (bake + gameplay)"
        );
    }

    /// Bake failure (invalid solver config) → gameplay cleared, no event.
    #[test]
    fn bake_failure_clears_gameplay_no_event() {
        let mut app = setup_app();
        app.world_mut()
            .resource_mut::<crate::map::surface_height::types::HeightSolverConfig>()
            .max_iterations = 0; // validate_config → ZeroIterations
        insert_land_tile(&mut app, 0, 0, 0.5);
        app.update();

        let state = app.world().resource::<SurfaceGameplayGenerationState>();
        assert_eq!(
            state.last_outcome,
            SurfaceGameplayGenerationOutcome::Failure
        );
        assert_eq!(state.failure_count, 1);
        assert_eq!(state.generation_count, 0);

        let field = app.world().resource::<SurfaceMetricField>();
        assert!(field.cells.is_empty(), "failure: metrics must be cleared");

        let gameplay = app.world().resource::<SurfaceGameplayMap>();
        assert!(
            gameplay.cells.is_empty(),
            "failure: gameplay must be cleared"
        );

        let events = app.world().resource::<RebuildEventCounter>();
        assert_eq!(events.0, 0, "bake failure must NOT write RebuildMeshEvent");
    }

    /// Same fingerprint (no changes) → no retry, no counters moving.
    #[test]
    fn same_fingerprint_no_retry() {
        let mut app = setup_app();
        insert_land_tile(&mut app, 0, 0, 0.5);
        app.update();
        app.update();

        let state = app.world().resource::<SurfaceGameplayGenerationState>();
        assert_eq!(
            state.last_outcome,
            SurfaceGameplayGenerationOutcome::Success
        );
        assert_eq!(state.generation_count, 1);

        let events = app.world().resource::<RebuildEventCounter>();
        assert_eq!(events.0, 2, "no changes must not emit rebuild events");
    }

    /// Terrain classification change without bake change → regenerated.
    #[test]
    fn terrain_change_regenerates_gameplay() {
        let mut app = setup_app();
        insert_land_tile(&mut app, 0, 0, 0.5);
        app.update();

        {
            let state = app.world().resource::<SurfaceGameplayGenerationState>();
            assert_eq!(state.generation_count, 1);
            let gameplay = app.world().resource::<SurfaceGameplayMap>();
            assert_eq!(gameplay.cells[&HexCoord::new(0, 0)].movement_cost, 20);
        }

        app.world_mut()
            .get_resource_mut::<MapData>()
            .expect("MapData must exist")
            .tiles
            .get_mut(&HexCoord::new(0, 0))
            .expect("tile must exist")
            .terrain = TerrainType::Swamp;
        app.update();

        let state = app.world().resource::<SurfaceGameplayGenerationState>();
        assert_eq!(state.generation_count, 2, "terrain change must regenerate");
        let gameplay = app.world().resource::<SurfaceGameplayMap>();
        assert_eq!(
            gameplay.cells[&HexCoord::new(0, 0)].movement_cost,
            50,
            "swamp cost must apply"
        );

        let events = app.world().resource::<RebuildEventCounter>();
        assert_eq!(events.0, 3, "terrain change must emit a rebuild event");
    }

    /// Config change → regenerated with the new policy.
    #[test]
    fn config_change_regenerates_gameplay() {
        let mut app = setup_app();
        insert_land_tile(&mut app, 0, 0, 0.5);
        app.update();

        app.world_mut()
            .resource_mut::<crate::map::surface_gameplay::config::SurfaceGameplayConfig>()
            .max_walk_step = 0.10;
        app.update();

        let state = app.world().resource::<SurfaceGameplayGenerationState>();
        assert_eq!(state.generation_count, 2, "config change must regenerate");

        let events = app.world().resource::<RebuildEventCounter>();
        assert_eq!(events.0, 3, "config change must emit a rebuild event");
    }
}
