// src/map/height_constraints/tests_runtime_lifecycle.rs
//! Lifecycle integration tests for missing surface retry prevention and cliff edit stability.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightConstraintLifecycleTestsPlugin;

impl Plugin for HeightConstraintLifecycleTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::EdgeCoord;
    use crate::map::data::{
        CliffLowerSide, EdgeData, EdgeType, LandscapeFeature, MapData, TileData, WorldSeed,
    };
    use crate::map::height_constraints::runtime::HeightConstraintCompilationState;
    use crate::map::height_constraints::types::HeightConstraintSet;
    use crate::map::HexCoord;
    use bevy::app::App;
    use bevy::MinimalPlugins;

    #[test]
    fn failed_surface_topology_increments_failure_and_does_not_retry() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::map::height_constraints::HeightConstraintsPlugin,
        ));

        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        map.tiles.insert(
            c1,
            TileData {
                landscape_feature: LandscapeFeature::Mountain,
                ..Default::default()
            },
        );

        app.insert_resource(map)
            .insert_resource(crate::map::surface_topology::types::SurfaceTopology::default());

        app.update();
        let fail1 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .failure_count;
        assert_eq!(fail1, 1);
        assert!(app
            .world()
            .resource::<HeightConstraintSet>()
            .regions
            .is_empty());

        // Update frame without input changes -> no retry
        app.update();
        let fail2 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .failure_count;
        assert_eq!(fail2, 1);
    }

    #[test]
    fn cliff_lower_side_edit_increments_generation_without_changing_topology() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::map::face_topology::FaceTopologyPlugin,
            crate::map::surface_topology::SurfaceTopologyPlugin,
            crate::map::height_constraints::HeightConstraintsPlugin,
        ));

        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());

        let edge = EdgeCoord::new(c1, c2);
        map.edges.insert(
            edge,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::Unresolved,
            },
        );

        app.add_message::<crate::map::GenerateMapEvent>()
            .add_message::<crate::map::RebuildMeshEvent>()
            .insert_resource(map)
            .insert_resource(WorldSeed::new(42));

        app.update();
        let face_gen1 = app
            .world()
            .resource::<crate::map::face_topology::runtime::HexFaceTopologyGenerationState>()
            .generation_count;
        let gen1 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .generation_count;

        // Edit cliff lower side Unresolved -> A
        let mut map = app.world_mut().resource_mut::<MapData>();
        if let Some(edge_data) = map.edges.get_mut(&edge) {
            edge_data.cliff_lower_side = CliffLowerSide::A;
        }

        app.update();
        let face_gen2 = app
            .world()
            .resource::<crate::map::face_topology::runtime::HexFaceTopologyGenerationState>()
            .generation_count;
        let gen2 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .generation_count;

        assert_eq!(face_gen2, face_gen1); // Topology unchanged
        assert_eq!(gen2, gen1 + 1); // Constraint recompiled
        assert_eq!(
            app.world().resource::<HeightConstraintSet>().cliffs[0].lower_side,
            CliffLowerSide::A
        );
    }

    #[test]
    fn surface_topology_change_increments_generation_with_identical_inputs() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::map::face_topology::FaceTopologyPlugin,
            crate::map::surface_topology::SurfaceTopologyPlugin,
            crate::map::height_constraints::HeightConstraintsPlugin,
        ));

        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        map.tiles.insert(
            c1,
            TileData {
                landscape_feature: LandscapeFeature::Mountain,
                ..Default::default()
            },
        );

        app.add_message::<crate::map::GenerateMapEvent>()
            .add_message::<crate::map::RebuildMeshEvent>()
            .insert_resource(map)
            .insert_resource(WorldSeed::new(42));

        app.update();
        let gen1 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .generation_count;

        // Mutate WorldSeed (triggers face & surface topology regeneration without changing MapData features)
        let mut seed = app.world_mut().resource_mut::<WorldSeed>();
        *seed = WorldSeed::new(99);

        app.update();
        let gen2 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .generation_count;
        assert_eq!(gen2, gen1 + 1);
    }

    #[test]
    fn height_constraint_compilation_outcome_transitions_test() {
        use crate::map::height_constraints::runtime::HeightConstraintCompilationOutcome;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::map::face_topology::FaceTopologyPlugin,
            crate::map::surface_topology::SurfaceTopologyPlugin,
            crate::map::height_constraints::HeightConstraintsPlugin,
        ));

        let state = app.world().resource::<HeightConstraintCompilationState>();
        assert_eq!(
            state.last_outcome,
            HeightConstraintCompilationOutcome::Uninitialized
        );

        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        map.tiles.insert(
            c1,
            TileData {
                landscape_feature: LandscapeFeature::Mountain,
                ..Default::default()
            },
        );

        app.add_message::<crate::map::GenerateMapEvent>()
            .add_message::<crate::map::RebuildMeshEvent>()
            .insert_resource(map)
            .insert_resource(WorldSeed::new(42));

        app.update();
        let outcome1 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .last_outcome;
        assert_eq!(outcome1, HeightConstraintCompilationOutcome::Success);
    }
}
