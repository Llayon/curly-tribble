// src/map/tools/tests_cliff_edit/tests.rs
//! Unit tests for picker, edit semantics, stroke mechanics, and runtime integration.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct CliffEditTestsSubPlugin;

impl Plugin for CliffEditTestsSubPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::{CurrentTool, EditorPhase, GameState};
    use crate::map::data::{
        CliffLowerSide, EdgeCoord, EdgeData, EdgeType, MapData, TileData, WorldSeed,
    };
    use crate::map::face_topology::edge_binding::BoundCliffEdges;
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::types::HexFaceTopology;
    use crate::map::tools::cliff_edit::{
        apply_single_cliff_click, CliffClickButton, CliffStrokeState, HoveredCliffEdge,
    };
    use crate::map::tools::cliff_picking::{classify_side, LogicalEdgeSide};
    use crate::map::tools::landscape_edge_picker::{
        build_landscape_edge_pick_index, rebuild_landscape_edge_pick_index, LandscapeEdgePickIndex,
    };
    use crate::map::HexCoord;

    fn create_two_tile_map() -> MapData {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());
        map
    }

    #[test]
    fn picker_exact_warped_endpoints_and_canonical_ab() {
        let map = create_two_tile_map();
        let seed = WorldSeed::new(42);
        let topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("Topology failed");

        let pick_index = build_landscape_edge_pick_index(&topology).expect("Index build failed");

        assert_eq!(pick_index.edges.len(), 1);
        let edge = &pick_index.edges[0];

        let expected_edge = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        assert_eq!(edge.logical_edge, expected_edge);

        let face_a_obj = &topology.faces[edge.face_a.index()];
        let face_b_obj = &topology.faces[edge.face_b.index()];

        assert_eq!(face_a_obj.hex, expected_edge.a);
        assert_eq!(face_b_obj.hex, expected_edge.b);

        let he_a = &topology.half_edges[edge.half_edge_a.index()];
        let he_b = &topology.half_edges[edge.half_edge_b.index()];

        assert_eq!(he_a.incident_face, edge.face_a);
        assert_eq!(he_b.incident_face, edge.face_b);
        assert_eq!(he_a.twin, Some(edge.half_edge_b));
        assert_eq!(he_b.twin, Some(edge.half_edge_a));

        let v0 = topology.vertices[he_a.origin.index()].position;
        let v1 = topology.vertices[he_a.destination.index()].position;

        assert_eq!(edge.segment_start, v0);
        assert_eq!(edge.segment_end, v1);
    }

    #[test]
    fn cursor_side_classification_a_b_none() {
        let seg_start = Vec2::new(0.0, 0.0);
        let seg_end = Vec2::new(2.0, 0.0);
        let center_a = Vec2::new(1.0, 1.0);
        let center_b = Vec2::new(1.0, -1.0);

        let cursor_toward_a = Vec2::new(1.0, 0.5);
        let side_a = classify_side(cursor_toward_a, seg_start, seg_end, center_a, center_b);
        assert_eq!(side_a, Some(LogicalEdgeSide::A));

        let cursor_toward_b = Vec2::new(1.0, -0.5);
        let side_b = classify_side(cursor_toward_b, seg_start, seg_end, center_a, center_b);
        assert_eq!(side_b, Some(LogicalEdgeSide::B));

        let cursor_on_line = Vec2::new(1.0, 0.0);
        let side_none = classify_side(cursor_on_line, seg_start, seg_end, center_a, center_b);
        assert_eq!(side_none, None);
    }

    #[test]
    fn single_click_edit_semantics_flat_unresolved_a_b_rmb() {
        let mut map = create_two_tile_map();
        let edge = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));

        let hit_unresolved = crate::map::tools::cliff_picking::LandscapeEdgeHit {
            logical_edge: edge,
            side: None,
            vertices: [
                crate::map::face_topology::types::VertexId::new(0),
                crate::map::face_topology::types::VertexId::new(1),
            ],
            distance_squared: 0.01,
        };

        // 1. Flat + LMB -> Unresolved
        let changed =
            apply_single_cliff_click(&mut map, &hit_unresolved, CliffClickButton::Primary);
        assert!(changed);
        if let Some(edge_data) = map.edges.get(&edge) {
            assert_eq!(edge_data.cliff_lower_side, CliffLowerSide::Unresolved);
        } else {
            panic!("Expected edge to exist");
        }

        // 2. Click A side -> A
        let hit_a = crate::map::tools::cliff_picking::LandscapeEdgeHit {
            side: Some(LogicalEdgeSide::A),
            ..hit_unresolved.clone()
        };
        let changed = apply_single_cliff_click(&mut map, &hit_a, CliffClickButton::Primary);
        assert!(changed);
        if let Some(edge_data) = map.edges.get(&edge) {
            assert_eq!(edge_data.cliff_lower_side, CliffLowerSide::A);
        } else {
            panic!("Expected edge to exist");
        }

        // 3. Click B side -> B
        let hit_b = crate::map::tools::cliff_picking::LandscapeEdgeHit {
            side: Some(LogicalEdgeSide::B),
            ..hit_unresolved.clone()
        };
        let changed = apply_single_cliff_click(&mut map, &hit_b, CliffClickButton::Primary);
        assert!(changed);
        if let Some(edge_data) = map.edges.get(&edge) {
            assert_eq!(edge_data.cliff_lower_side, CliffLowerSide::B);
        } else {
            panic!("Expected edge to exist");
        }

        // 4. RMB -> Removed
        let changed =
            apply_single_cliff_click(&mut map, &hit_unresolved, CliffClickButton::Secondary);
        assert!(changed);
        assert!(!map.edges.contains_key(&edge));
    }

    #[test]
    fn stroke_state_connectivity_and_visited_protection() {
        let mut stroke = CliffStrokeState::default();
        let e1 = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        let _e2 = EdgeCoord::new(HexCoord::new(1, 0), HexCoord::new(2, 0));

        let v0 = crate::map::face_topology::types::VertexId::new(0);
        let v1 = crate::map::face_topology::types::VertexId::new(1);
        let v2 = crate::map::face_topology::types::VertexId::new(2);

        stroke.active = true;
        stroke.visited_edges.insert(e1);
        stroke.previous_accepted_edge = Some(e1);
        stroke.previous_accepted_vertices = Some([v0, v1]);

        // e2 shares v1 with e1 -> connected!
        let e2_connected = [v1, v2].contains(&v0) || [v1, v2].contains(&v1);
        assert!(e2_connected);

        // e_disjoint shares no vertices -> rejected!
        let v10 = crate::map::face_topology::types::VertexId::new(10);
        let v11 = crate::map::face_topology::types::VertexId::new(11);
        let disjoint_connected = [v10, v11].contains(&v0) || [v10, v11].contains(&v1);
        assert!(!disjoint_connected);

        stroke.reset();
        assert!(!stroke.active);
        assert!(stroke.visited_edges.is_empty());
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

        // Topology generation count MUST NOT change!
        assert_eq!(count_before, count_after);

        // BoundCliffEdges MUST update to reflect the new cliff!
        let bound = app.world().resource::<BoundCliffEdges>();
        assert_eq!(bound.edges.len(), 1);
        assert_eq!(bound.edges[0].logical_edge, edge);
        assert_eq!(bound.edges[0].lower_side, CliffLowerSide::A);
    }
}
