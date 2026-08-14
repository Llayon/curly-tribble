// src/map/terrain_bake/tests_runtime.rs
//! Runtime lifecycle tests for `regenerate_surface_terrain_bake`.
//!
//! Tests drive the REAL M5 runtime through honest config transitions
//! (valid → invalid → valid) instead of stamp injection, because the
//! full topology pipeline emits its own M5 outcome on the first frame.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{OceanState, TileData, WorldSeed};
    use crate::map::surface_height::runtime::{
        HeightSolveGenerationOutcome, HeightSolveGenerationState,
    };
    use crate::map::surface_height::types::HeightSolverConfig;
    use crate::map::terrain_bake::runtime::{
        TerrainBakeGenerationOutcome, TerrainBakeGenerationState,
    };
    use crate::map::terrain_bake::types::SurfaceTerrainBake;
    use crate::map::{HexCoord, MapData, RebuildMeshEvent};
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct TerrainBakeRuntimeTestsPlugin;

    impl Plugin for TerrainBakeRuntimeTestsPlugin {
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
        ));
        app.add_message::<crate::map::GenerateMapEvent>();
        app.add_message::<RebuildMeshEvent>();
        app.insert_resource(WorldSeed::new(42));
        app.init_resource::<MapData>();
        app.init_resource::<RebuildEventCounter>();
        app.add_systems(
            Update,
            collect_rebuild_events
                .after(crate::map::terrain_bake::runtime::regenerate_surface_terrain_bake),
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

    /// Resources start empty and bake state uninitialized before any frame.
    #[test]
    fn bake_starts_default_and_uninitialized() {
        let app = setup_app();

        assert!(app
            .world()
            .resource::<SurfaceTerrainBake>()
            .vertices
            .is_empty());
        assert_eq!(
            app.world()
                .resource::<TerrainBakeGenerationState>()
                .last_outcome,
            TerrainBakeGenerationOutcome::Uninitialized
        );
    }

    /// M5 Success (real solve on a land hex) → bake Success with data → rebuild event.
    #[test]
    fn m5_success_builds_bake_and_writes_rebuild_event() {
        let mut app = setup_app();
        insert_land_tile(&mut app, 0, 0, 0.5);
        app.update();

        let m5_state = app.world().resource::<HeightSolveGenerationState>();
        assert_eq!(m5_state.last_outcome, HeightSolveGenerationOutcome::Success);
        assert_eq!(m5_state.generation_count, 1);

        let bake_state = app.world().resource::<TerrainBakeGenerationState>();
        assert_eq!(
            bake_state.last_outcome,
            TerrainBakeGenerationOutcome::Success
        );
        assert_eq!(bake_state.generation_count, 1);
        assert_eq!(bake_state.failure_count, 0);

        let bake = app.world().resource::<SurfaceTerrainBake>();
        assert!(
            !bake.vertices.is_empty(),
            "bake must contain ground vertices"
        );
        assert!(!bake.faces.is_empty(), "bake must contain ground faces");
        assert_eq!(bake.stats.ground_vertex_count, bake.vertices.len());

        let events = app.world().resource::<RebuildEventCounter>();
        assert_eq!(
            events.0, 1,
            "M5 Success must write exactly one RebuildMeshEvent"
        );
    }

    /// M5 Failure (invalid solver config) → bake cleared, no rebuild event.
    #[test]
    fn m5_failure_clears_bake_no_event() {
        let mut app = setup_app();
        app.world_mut()
            .resource_mut::<HeightSolverConfig>()
            .max_iterations = 0; // validate_config → ZeroIterations
        insert_land_tile(&mut app, 0, 0, 0.5);
        app.update();

        let m5_state = app.world().resource::<HeightSolveGenerationState>();
        assert_eq!(m5_state.last_outcome, HeightSolveGenerationOutcome::Failure);

        let bake_state = app.world().resource::<TerrainBakeGenerationState>();
        assert_eq!(
            bake_state.last_outcome,
            TerrainBakeGenerationOutcome::Failure
        );
        assert_eq!(bake_state.failure_count, 1);
        assert_eq!(bake_state.generation_count, 0);

        let bake = app.world().resource::<SurfaceTerrainBake>();
        assert!(bake.vertices.is_empty(), "failure: bake must be cleared");

        let events = app.world().resource::<RebuildEventCounter>();
        assert_eq!(events.0, 0, "M5 Failure must NOT write RebuildMeshEvent");
    }

    /// Same M5 stamp (no config/tile change) → no retry, no counters moving.
    #[test]
    fn same_m5_stamp_no_retry() {
        let mut app = setup_app();
        app.world_mut()
            .resource_mut::<HeightSolverConfig>()
            .max_iterations = 0;
        insert_land_tile(&mut app, 0, 0, 0.5);
        app.update();
        app.update();

        let bake_state = app.world().resource::<TerrainBakeGenerationState>();
        assert_eq!(
            bake_state.last_outcome,
            TerrainBakeGenerationOutcome::Failure
        );
        assert_eq!(bake_state.failure_count, 1, "same stamp: no retry");
    }

    /// Success(gen=1) → Failure(gen=1) is processed because the full stamp differs.
    #[test]
    fn success_then_failure_same_gen_is_retried() {
        let mut app = setup_app();
        insert_land_tile(&mut app, 0, 0, 0.5);
        app.update();

        assert_eq!(
            app.world()
                .resource::<TerrainBakeGenerationState>()
                .last_outcome,
            TerrainBakeGenerationOutcome::Success
        );

        app.world_mut()
            .resource_mut::<HeightSolverConfig>()
            .max_iterations = 0;
        app.update();

        let bake_state = app.world().resource::<TerrainBakeGenerationState>();
        assert_eq!(
            bake_state.last_outcome,
            TerrainBakeGenerationOutcome::Failure
        );
        assert_eq!(
            bake_state.failure_count, 1,
            "Failure(gen=1) must be processed"
        );
        assert_eq!(
            bake_state.generation_count, 1,
            "previous success count kept"
        );
        assert!(
            app.world()
                .resource::<SurfaceTerrainBake>()
                .vertices
                .is_empty(),
            "bake must be cleared after failure"
        );
    }

    /// Failure → Success wakes the bake: counters and geometry recover, event fires.
    #[test]
    fn failure_then_success_wakes_bake() {
        let mut app = setup_app();
        app.world_mut()
            .resource_mut::<HeightSolverConfig>()
            .max_iterations = 0;
        insert_land_tile(&mut app, 0, 0, 0.5);
        app.update();

        assert_eq!(
            app.world()
                .resource::<TerrainBakeGenerationState>()
                .last_outcome,
            TerrainBakeGenerationOutcome::Failure
        );

        app.world_mut()
            .resource_mut::<HeightSolverConfig>()
            .max_iterations = 32;
        app.update();

        let bake_state = app.world().resource::<TerrainBakeGenerationState>();
        assert_eq!(
            bake_state.last_outcome,
            TerrainBakeGenerationOutcome::Success
        );
        assert_eq!(bake_state.generation_count, 1);
        assert!(
            !app.world()
                .resource::<SurfaceTerrainBake>()
                .vertices
                .is_empty(),
            "bake must be rebuilt after M5 recovers"
        );

        let events = app.world().resource::<RebuildEventCounter>();
        assert_eq!(events.0, 1, "exactly one rebuild event on recovery");
    }
}
