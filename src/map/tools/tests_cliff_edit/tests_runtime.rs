// src/map/tools/tests_cliff_edit/tests_runtime.rs
//! Runtime integration tests for warped cliff editing.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct CliffEditTestsRuntimePlugin;

impl Plugin for CliffEditTestsRuntimePlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::game_state::{CurrentTool, EditorPhase, GameState};
    use crate::map::data::{
        CliffLowerSide, EdgeCoord, EdgeData, EdgeType, MapData, TileData, WorldSeed,
    };
    use crate::map::face_topology::edge_binding::BoundCliffEdges;
    use crate::map::face_topology::types::HexFaceTopology;
    use crate::map::tools::cliff_edit::{CliffStrokeState, HoveredCliffEdge};
    use crate::map::tools::landscape_edge_picker::{
        rebuild_landscape_edge_pick_index, LandscapeEdgePickIndex,
    };
    use crate::map::HexCoord;
    use bevy::prelude::*;

    fn create_two_tile_map() -> MapData {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());
        map
    }

    #[test]
    fn runtime_integration_cliff_edit_does_not_regenerate_topology() {
        let map = create_two_tile_map();
        let seed = WorldSeed::new(42);

        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_resource(map)
            .insert_resource(seed)
            .init_resource::<crate::map::terrain_gen::TerrainConfig>()
            .init_resource::<HexFaceTopology>()
            .init_resource::<crate::map::face_topology::debug::HexFaceDebugSettings>()
            .init_resource::<crate::map::face_topology::debug::HexFaceDebugCache>()
            .init_resource::<crate::map::face_topology::runtime::HexFaceTopologyGenerationState>()
            .init_resource::<LandscapeEdgePickIndex>()
            .init_resource::<BoundCliffEdges>()
            .init_resource::<CliffStrokeState>()
            .init_resource::<HoveredCliffEdge>()
            .init_resource::<CurrentTool>()
            .insert_state(EditorPhase::Landscape)
            .insert_state(GameState::Playing)
            .add_message::<crate::map::GenerateMapEvent>()
            .add_message::<crate::map::RebuildMeshEvent>()
            .add_systems(
                Update,
                (
                    crate::map::face_topology::runtime::regenerate_hex_face_topology,
                    crate::map::face_topology::runtime::rebuild_bound_cliff_edges,
                    rebuild_landscape_edge_pick_index,
                ),
            );

        app.update();

        let count_before = app
            .world()
            .resource::<crate::map::face_topology::runtime::HexFaceTopologyGenerationState>()
            .generation_count;

        let edge = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        if let Some(mut map_data) = app.world_mut().get_resource_mut::<MapData>() {
            map_data.edges.insert(
                edge,
                EdgeData {
                    edge_type: EdgeType::Cliff,
                    cliff_lower_side: CliffLowerSide::A,
                },
            );
        }

        app.update();

        let count_after = app
            .world()
            .resource::<crate::map::face_topology::runtime::HexFaceTopologyGenerationState>()
            .generation_count;

        assert_eq!(count_before, count_after);

        let bound = app.world().resource::<BoundCliffEdges>();
        assert_eq!(bound.edges.len(), 1);
        assert_eq!(bound.edges[0].logical_edge, edge);
        assert_eq!(bound.edges[0].lower_side, CliffLowerSide::A);
    }
}
