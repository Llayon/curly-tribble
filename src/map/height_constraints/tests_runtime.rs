// src/map/height_constraints/tests_runtime.rs
//! Integration tests for height constraint compilation state and runtime fingerprint lifecycle.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightConstraintRuntimeTestsPlugin;

impl Plugin for HeightConstraintRuntimeTestsPlugin {
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
    fn landscape_semantic_edit_triggers_recompile() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::map::face_topology::FaceTopologyPlugin,
            crate::map::surface_topology::SurfaceTopologyPlugin,
            crate::map::height_constraints::HeightConstraintsPlugin,
        ));

        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        map.tiles.insert(c1, TileData::default());

        app.add_message::<crate::map::GenerateMapEvent>()
            .add_message::<crate::map::RebuildMeshEvent>()
            .insert_resource(map)
            .insert_resource(WorldSeed::new(42));

        app.update();
        let gen1 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .generation_count;
        assert_eq!(
            app.world().resource::<HeightConstraintSet>().regions.len(),
            0
        );

        // Mutate landscape feature
        let mut map = app.world_mut().resource_mut::<MapData>();
        if let Some(tile) = map.tiles.get_mut(&c1) {
            tile.landscape_feature = LandscapeFeature::Mountain;
        }

        app.update();
        let gen2 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .generation_count;
        assert_eq!(gen2, gen1 + 1);
        assert_eq!(
            app.world().resource::<HeightConstraintSet>().regions.len(),
            1
        );
    }

    #[test]
    fn elevation_only_edit_does_not_trigger_constraint_recompile() {
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
                elevation: 0.0,
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

        // Mutate elevation only
        let mut map = app.world_mut().resource_mut::<MapData>();
        if let Some(tile) = map.tiles.get_mut(&c1) {
            tile.elevation = 10.0;
        }

        app.update();
        let gen2 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .generation_count;
        assert_eq!(gen2, gen1);
    }

    #[test]
    fn semantic_removal_clears_constraints_and_increments_generation() {
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
        map.tiles.insert(
            c1,
            TileData {
                landscape_feature: LandscapeFeature::Mountain,
                ..Default::default()
            },
        );
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
        assert_eq!(
            app.world().resource::<HeightConstraintSet>().regions.len(),
            1
        );
        assert_eq!(
            app.world().resource::<HeightConstraintSet>().cliffs.len(),
            1
        );
        let gen1 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .generation_count;

        // Remove mountain and cliff
        let mut map = app.world_mut().resource_mut::<MapData>();
        if let Some(tile) = map.tiles.get_mut(&c1) {
            tile.landscape_feature = LandscapeFeature::None;
        }
        if let Some(edge_data) = map.edges.get_mut(&edge) {
            edge_data.edge_type = EdgeType::Flat;
        }

        app.update();
        let gen2 = app
            .world()
            .resource::<HeightConstraintCompilationState>()
            .generation_count;
        assert_eq!(gen2, gen1 + 1);

        let constraints = app.world().resource::<HeightConstraintSet>();
        assert!(constraints.regions.is_empty());
        assert!(constraints.cliffs.is_empty());
    }
}
