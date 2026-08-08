// src/map/tools/tests_cliff_edit/tests_picking.rs
//! Unit tests for warped cliff edge picking and side classification.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct CliffEditTestsPickingPlugin;

impl Plugin for CliffEditTestsPickingPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{EdgeCoord, MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::tools::cliff_picking::{classify_side, LogicalEdgeSide};
    use crate::map::tools::landscape_edge_picker::build_landscape_edge_pick_index;
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
}
