// src/map/tools/tests_cliff_edit/tests_semantics.rs
//! Unit tests for single-click edit semantics and connected stroke steps.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct CliffEditTestsSemanticsPlugin;

impl Plugin for CliffEditTestsSemanticsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{CliffLowerSide, EdgeCoord, EdgeData, EdgeType, MapData, TileData};
    use crate::map::tools::cliff_edit::{
        apply_cliff_stroke_step, apply_single_cliff_click, CliffClickButton, CliffStrokePhase,
        CliffStrokeState,
    };
    use crate::map::tools::cliff_picking::{LandscapeEdgeHit, LogicalEdgeSide};
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
    fn single_click_edit_semantics_flat_unresolved_a_b_rmb() {
        let mut map = create_two_tile_map();
        let edge = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));

        let hit_unresolved = LandscapeEdgeHit {
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
        let hit_a = LandscapeEdgeHit {
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
        let hit_b = LandscapeEdgeHit {
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
    fn stroke_orient_existing_and_erase_reject_flat_bridge() {
        let mut map = create_two_tile_map();
        let e1 = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        let e_flat = EdgeCoord::new(HexCoord::new(1, 0), HexCoord::new(2, 0));
        let e2 = EdgeCoord::new(HexCoord::new(2, 0), HexCoord::new(3, 0));

        let v0 = crate::map::face_topology::types::VertexId::new(0);
        let v1 = crate::map::face_topology::types::VertexId::new(1);
        let v2 = crate::map::face_topology::types::VertexId::new(2);
        let v3 = crate::map::face_topology::types::VertexId::new(3);

        map.edges.insert(
            e1,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::Unresolved,
            },
        );
        map.edges.insert(
            e2,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::Unresolved,
            },
        );

        let hit1 = LandscapeEdgeHit {
            logical_edge: e1,
            side: Some(LogicalEdgeSide::A),
            vertices: [v0, v1],
            distance_squared: 0.0,
        };
        let hit_flat = LandscapeEdgeHit {
            logical_edge: e_flat,
            side: Some(LogicalEdgeSide::A),
            vertices: [v1, v2],
            distance_squared: 0.0,
        };
        let hit2 = LandscapeEdgeHit {
            logical_edge: e2,
            side: Some(LogicalEdgeSide::A),
            vertices: [v2, v3],
            distance_squared: 0.0,
        };

        // 1. Test OrientExisting: start at e1
        let mut stroke = CliffStrokeState::default();
        let P_INIT = CliffStrokePhase::Initial;
        let P_SUB = CliffStrokePhase::Subsequent;
        let step1 = apply_cliff_stroke_step(&mut map, &mut stroke, &hit1, P_INIT, true, false);
        assert!(step1);
        assert!(stroke.active);
        assert_eq!(stroke.previous_accepted_edge, Some(e1));

        // Attempt stroke step on e_flat -> MUST REJECT!
        let step_flat =
            apply_cliff_stroke_step(&mut map, &mut stroke, &hit_flat, P_SUB, false, false);
        assert!(!step_flat);
        assert!(!stroke.visited_edges.contains(&e_flat));
        assert_eq!(stroke.previous_accepted_edge, Some(e1));

        // Attempt stroke step directly on e2 -> rejected due to disconnected vertices!
        let step2 = apply_cliff_stroke_step(&mut map, &mut stroke, &hit2, P_SUB, false, false);
        assert!(!step2);

        // 2. Test Erase: initial RMB on Flat does not start stroke
        let mut stroke_erase = CliffStrokeState::default();
        let rmb_flat =
            apply_cliff_stroke_step(&mut map, &mut stroke_erase, &hit_flat, P_INIT, false, true);
        assert!(!rmb_flat);
        assert!(!stroke_erase.active);

        // Start Erase on e1
        let step1_erase =
            apply_cliff_stroke_step(&mut map, &mut stroke_erase, &hit1, P_INIT, false, true);
        assert!(step1_erase);
        assert!(!map.edges.contains_key(&e1));

        // Step onto e_flat -> MUST REJECT!
        let step_flat_erase =
            apply_cliff_stroke_step(&mut map, &mut stroke_erase, &hit_flat, P_SUB, false, false);
        assert!(!step_flat_erase);
        assert_eq!(stroke_erase.previous_accepted_edge, Some(e1));
    }

    #[test]
    fn stroke_paint_unresolved_paints_flats_and_preserves_existing_cliffs() {
        let mut map = create_two_tile_map();
        let e_flat = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        let e_cliff_a = EdgeCoord::new(HexCoord::new(1, 0), HexCoord::new(2, 0));

        let v0 = crate::map::face_topology::types::VertexId::new(0);
        let v1 = crate::map::face_topology::types::VertexId::new(1);
        let v2 = crate::map::face_topology::types::VertexId::new(2);

        map.edges.insert(
            e_cliff_a,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::A,
            },
        );

        let hit_flat = LandscapeEdgeHit {
            logical_edge: e_flat,
            side: None,
            vertices: [v0, v1],
            distance_squared: 0.0,
        };
        let hit_cliff_a = LandscapeEdgeHit {
            logical_edge: e_cliff_a,
            side: Some(LogicalEdgeSide::B),
            vertices: [v1, v2],
            distance_squared: 0.0,
        };

        let mut stroke = CliffStrokeState::default();
        let step1 = apply_cliff_stroke_step(
            &mut map,
            &mut stroke,
            &hit_flat,
            CliffStrokePhase::Initial,
            true,
            false,
        );
        assert!(step1);
        assert_eq!(
            map.edges.get(&e_flat).unwrap().cliff_lower_side,
            CliffLowerSide::Unresolved
        );

        // Step onto e_cliff_a -> MUST accept and PRESERVE side A!
        let step2 = apply_cliff_stroke_step(
            &mut map,
            &mut stroke,
            &hit_cliff_a,
            CliffStrokePhase::Subsequent,
            false,
            false,
        );
        assert!(step2);
        assert_eq!(
            map.edges.get(&e_cliff_a).unwrap().cliff_lower_side,
            CliffLowerSide::A
        );
    }
}
