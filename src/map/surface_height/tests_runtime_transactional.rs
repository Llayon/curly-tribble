// src/map/surface_height/tests_runtime_transactional.rs
//! Transactional publication and lifecycle tests for the surface height runtime.
//! Tests the triple-atomic contract: guide + targets + layer published/cleared together.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{
        CliffLowerSide, EdgeCoord, EdgeData, EdgeType, MapData, OceanState, TileData,
    };
    use crate::map::height_graph::runtime::{
        HeightGraphGenerationOutcome, HeightGraphGenerationState,
    };
    use crate::map::height_graph::types::HeightConstraintGraph;
    use crate::map::surface_height::guide::LegacyHeightGuide;
    use crate::map::surface_height::runtime::{
        HeightSolveGenerationOutcome, HeightSolveGenerationState,
    };
    use crate::map::surface_height::targets::HeightTargetField;
    use crate::map::surface_height::types::{HeightSolverConfig, SurfaceHeightLayer};
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceHeightRuntimeTransactionalTestsPlugin;

    impl Plugin for SurfaceHeightRuntimeTransactionalTestsPlugin {
        fn build(&self, _app: &mut App) {}
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
        ));
        app.add_message::<crate::map::GenerateMapEvent>();
        app.add_message::<crate::map::RebuildMeshEvent>();
        app.insert_resource(crate::map::data::WorldSeed::new(42));
        app.init_resource::<MapData>();
        app
    }

    fn setup_single_land_tile(app: &mut App, elevation: f32) {
        let mut map_data = app.world_mut().resource_mut::<MapData>();
        map_data.tiles.insert(
            HexCoord::new(0, 0),
            TileData {
                elevation,
                ocean_state: OceanState::Land,
                ..Default::default()
            },
        );
    }

    /// After success: guide, targets, and layer are all populated simultaneously.
    #[test]
    fn transactional_triple_publication_on_success() {
        let mut app = setup_app();
        setup_single_land_tile(&mut app, 0.40);
        app.update();

        let state = app.world().resource::<HeightSolveGenerationState>();
        assert_eq!(state.last_outcome, HeightSolveGenerationOutcome::Success);
        let graph_nodes = app.world().resource::<HeightConstraintGraph>().nodes.len();
        let guide_len = app.world().resource::<LegacyHeightGuide>().samples.len();
        let targets_len = app.world().resource::<HeightTargetField>().samples.len();
        let layer_len = app.world().resource::<SurfaceHeightLayer>().heights.len();

        assert_eq!(guide_len, graph_nodes, "guide must match node count");
        assert_eq!(targets_len, graph_nodes, "targets must match node count");
        assert_eq!(layer_len, graph_nodes, "layer must match node count");
    }

    /// When M4.1 reports Failure, all three derived resources are cleared atomically.
    #[test]
    fn m4_1_failure_clears_all_three_derived_resources() {
        let mut app = setup_app();
        setup_single_land_tile(&mut app, 0.40);
        app.update();

        // Inject M4.1 Failure directly
        {
            let mut graph_state = app.world_mut().resource_mut::<HeightGraphGenerationState>();
            graph_state.last_outcome = HeightGraphGenerationOutcome::Failure;
        }
        app.world_mut()
            .resource_mut::<HeightConstraintGraph>()
            .bypass_change_detection();
        app.update();

        assert!(
            app.world()
                .resource::<LegacyHeightGuide>()
                .samples
                .is_empty(),
            "guide must be cleared on M4.1 failure"
        );
        assert!(
            app.world()
                .resource::<HeightTargetField>()
                .samples
                .is_empty(),
            "targets must be cleared on M4.1 failure"
        );
        assert!(
            app.world()
                .resource::<SurfaceHeightLayer>()
                .heights
                .is_empty(),
            "layer must be cleared on M4.1 failure"
        );
    }

    /// After one failure, a second update without input change does NOT retry.
    #[test]
    fn failure_does_not_retry() {
        let mut app = setup_app();
        // Invalid config triggers solve failure
        {
            let mut config = app.world_mut().resource_mut::<HeightSolverConfig>();
            config.guide_weight = 0.0;
        }
        setup_single_land_tile(&mut app, 0.30);
        app.update();
        let fc1 = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .failure_count;

        app.update();
        let fc2 = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .failure_count;

        assert_eq!(
            fc1, fc2,
            "no retry: failure_count must not increase on same inputs"
        );
    }

    /// M4.1 Failure followed by Success with new generation_count wakes M5.
    #[test]
    fn m4_1_failure_then_success_wakes_m5() {
        let mut app = setup_app();
        setup_single_land_tile(&mut app, 0.40);
        app.update();
        let gen_before = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;

        // Inject Failure
        {
            let mut gs = app.world_mut().resource_mut::<HeightGraphGenerationState>();
            gs.last_outcome = HeightGraphGenerationOutcome::Failure;
        }
        app.world_mut()
            .resource_mut::<HeightConstraintGraph>()
            .bypass_change_detection();
        app.update();

        // Restore Success with incremented generation_count
        {
            let mut gs = app.world_mut().resource_mut::<HeightGraphGenerationState>();
            gs.last_outcome = HeightGraphGenerationOutcome::Success;
            gs.generation_count += 1;
        }
        app.world_mut()
            .resource_mut::<HeightConstraintGraph>()
            .bypass_change_detection();
        app.update();

        let state = app.world().resource::<HeightSolveGenerationState>();
        assert_eq!(state.last_outcome, HeightSolveGenerationOutcome::Success);
        assert!(state.generation_count > gen_before);
    }

    /// Changing a tile to Ocean triggers M5 rerun.
    #[test]
    fn ocean_edit_triggers_m5_rerun() {
        let mut app = setup_app();
        setup_single_land_tile(&mut app, 0.40);
        app.update();
        let gen_1 = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;

        {
            let mut map_data = app.world_mut().resource_mut::<MapData>();
            if let Some(tile) = map_data.tiles.get_mut(&HexCoord::new(0, 0)) {
                tile.ocean_state = OceanState::Ocean;
            }
        }
        app.update();
        let gen_2 = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;

        assert!(gen_2 > gen_1, "ocean state change must trigger M5 rerun");
    }

    /// Editing HeightSolverConfig triggers M5 rerun.
    #[test]
    fn config_edit_triggers_m5_rerun() {
        let mut app = setup_app();
        setup_single_land_tile(&mut app, 0.40);
        app.update();
        let gen_1 = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;

        {
            let mut config = app.world_mut().resource_mut::<HeightSolverConfig>();
            config.relaxation = 0.70;
        }
        app.update();
        let gen_2 = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;

        assert!(gen_2 > gen_1, "config change must trigger M5 rerun");
    }

    /// Changing cliff lower_side on an edge triggers M5 rerun.
    #[test]
    fn cliff_lower_side_change_triggers_m5_rerun() {
        let mut app = setup_app();
        let edge_coord = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        {
            let mut map_data = app.world_mut().resource_mut::<MapData>();
            map_data.tiles.insert(
                HexCoord::new(0, 0),
                TileData {
                    elevation: 0.20,
                    ocean_state: OceanState::Land,
                    ..Default::default()
                },
            );
            map_data.tiles.insert(
                HexCoord::new(1, 0),
                TileData {
                    elevation: 0.60,
                    ocean_state: OceanState::Land,
                    ..Default::default()
                },
            );
            map_data.edges.insert(
                edge_coord,
                EdgeData {
                    edge_type: EdgeType::Cliff,
                    cliff_lower_side: CliffLowerSide::Unresolved,
                },
            );
        }
        app.update();
        let gen_1 = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;

        {
            let mut map_data = app.world_mut().resource_mut::<MapData>();
            if let Some(edge) = map_data.edges.get_mut(&edge_coord) {
                edge.cliff_lower_side = CliffLowerSide::A;
            }
        }
        app.update();
        let gen_2 = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;

        assert!(
            gen_2 > gen_1,
            "cliff lower_side change must trigger M5 rerun"
        );
    }
}
