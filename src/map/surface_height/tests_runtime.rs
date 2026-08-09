// src/map/surface_height/tests_runtime.rs
//! Direct unit tests for surface height layer runtime lifecycle transitions and trigger gates.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{LandscapeFeature, MapData, OceanState, TileData};
    use crate::map::height_constraints::HeightConstraintSet;
    use crate::map::height_graph::types::HeightConstraintGraph;
    use crate::map::surface_height::guide::LegacyHeightGuide;
    use crate::map::surface_height::runtime::HeightSolveGenerationState;
    use crate::map::surface_height::types::{HeightSolverConfig, SurfaceHeightLayer};
    use crate::map::surface_topology::types::SurfaceTopology;
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceHeightRuntimeTestsPlugin;

    impl Plugin for SurfaceHeightRuntimeTestsPlugin {
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

    #[test]
    fn elevation_only_edit_regenerates_m5_without_topology_change() {
        let mut app = setup_app();

        {
            let mut map_data = app.world_mut().resource_mut::<MapData>();
            map_data.tiles.insert(
                HexCoord::new(0, 0),
                TileData {
                    elevation: 0.30,
                    ocean_state: OceanState::Land,
                    ..Default::default()
                },
            );
        }

        app.update();
        let initial_gen = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;
        assert_eq!(initial_gen, 1);

        // Edit elevation only
        {
            let mut map_data = app.world_mut().resource_mut::<MapData>();
            if let Some(tile) = map_data.tiles.get_mut(&HexCoord::new(0, 0)) {
                tile.elevation = 0.80;
            }
        }

        app.update();
        let updated_gen = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;
        assert_eq!(updated_gen, 2);
    }

    #[test]
    fn non_height_map_data_edit_does_not_regenerate_m5() {
        let mut app = setup_app();

        {
            let mut map_data = app.world_mut().resource_mut::<MapData>();
            map_data.tiles.insert(
                HexCoord::new(0, 0),
                TileData {
                    elevation: 0.30,
                    humidity: 0.20,
                    ocean_state: OceanState::Land,
                    ..Default::default()
                },
            );
        }

        app.update();
        let initial_gen = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;
        assert_eq!(initial_gen, 1);

        // Edit humidity only (non-height field)
        {
            let mut map_data = app.world_mut().resource_mut::<MapData>();
            if let Some(tile) = map_data.tiles.get_mut(&HexCoord::new(0, 0)) {
                tile.humidity = 0.90;
            }
        }

        app.update();
        let updated_gen = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;
        assert_eq!(updated_gen, 1); // Generation unchanged!
    }

    #[test]
    fn landscape_edit_propagates_full_m4_m4_1_m5_chain() {
        let mut app = setup_app();

        {
            let mut map_data = app.world_mut().resource_mut::<MapData>();
            map_data.tiles.insert(
                HexCoord::new(0, 0),
                TileData {
                    elevation: 0.30,
                    landscape_feature: LandscapeFeature::None,
                    ocean_state: OceanState::Land,
                    ..Default::default()
                },
            );
        }

        app.update();
        let initial_gen = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;
        assert_eq!(initial_gen, 1);

        // Edit landscape feature: None -> Mountain
        {
            let mut map_data = app.world_mut().resource_mut::<MapData>();
            if let Some(tile) = map_data.tiles.get_mut(&HexCoord::new(0, 0)) {
                tile.landscape_feature = LandscapeFeature::Mountain;
            }
        }

        app.update();
        let updated_gen = app
            .world()
            .resource::<HeightSolveGenerationState>()
            .generation_count;
        assert_eq!(updated_gen, 2);
    }
}
