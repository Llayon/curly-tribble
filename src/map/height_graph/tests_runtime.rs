// src/map/height_graph/tests_runtime.rs
//! Runtime lifecycle integration tests for HeightConstraintGraph.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightGraphRuntimeTestsPlugin;

impl Plugin for HeightGraphRuntimeTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{LandscapeFeature, MapData, TileData, WorldSeed};
    use crate::map::height_graph::runtime::HeightGraphGenerationState;
    use crate::map::height_graph::types::HeightConstraintGraph;
    use crate::map::HexCoord;
    use bevy::app::App;
    use bevy::MinimalPlugins;

    #[test]
    fn height_graph_regenerates_on_map_data_change() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::map::face_topology::FaceTopologyPlugin,
            crate::map::surface_topology::SurfaceTopologyPlugin,
            crate::map::height_constraints::HeightConstraintsPlugin,
            crate::map::height_graph::HeightGraphPlugin,
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
            .resource::<HeightGraphGenerationState>()
            .generation_count;
        let node_count1 = app.world().resource::<HeightConstraintGraph>().nodes.len();
        assert_eq!(gen1, 1);
        assert!(node_count1 > 0);
    }
}
