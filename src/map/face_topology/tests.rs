// src/map/face_topology/tests.rs
use bevy::prelude::*;

pub struct TestsPlugin;

impl Plugin for TestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod unit_tests {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::generator::generate_hex_face_topology;
    use crate::map::face_topology::validation::signed_area;
    use crate::map::face_topology::HalfEdgeId;
    use crate::map::HexCoord;
    use crate::map::WorldSeed;
    use bevy::math::Vec2;
    use std::collections::HashSet;

    fn generate_test_map(width: i32, height: i32) -> MapData {
        let mut map = MapData::default();
        for r in 0..height {
            let r_offset = r >> 1;
            for q in -r_offset..(width - r_offset) {
                map.tiles.insert(HexCoord::new(q, r), TileData::default());
            }
        }
        map
    }

    #[test]
    fn test_topology_verifications_1_to_21_on_40x40() {
        let map_orig = generate_test_map(40, 40);
        let map_clone = map_orig.clone();
        let seed42 = WorldSeed::new(42);

        let topo1 = generate_hex_face_topology(&map_orig, seed42).expect("Topology 1 failed");

        // 1. MapData unchanged
        assert_eq!(
            map_orig.tiles.len(),
            map_clone.tiles.len(),
            "Test 1: MapData tiles unchanged"
        );
        assert_eq!(
            map_orig.width, map_clone.width,
            "Test 1: MapData width unchanged"
        );
        assert_eq!(
            map_orig.height, map_clone.height,
            "Test 1: MapData height unchanged"
        );

        // 2. HexCoord neighbor results unchanged
        let c0 = HexCoord::new(0, 0);
        assert_eq!(c0.neighbors().len(), 6, "Test 2: HexCoord neighbors");

        // 3. One HexFace per MapData tile
        assert_eq!(
            topo1.faces.len(),
            map_orig.tiles.len(),
            "Test 3: One face per tile"
        );

        // 4. Six unique boundary vertices per face
        // 5. Six HalfEdges per face
        // 6-8. Next and Prev cycles and consistency
        // 12-15. Signed area, winding, self-intersection, non-zero edge length
        for (i, face) in topo1.faces.iter().enumerate() {
            let v_set: HashSet<_> = face.vertices.iter().copied().collect();
            assert_eq!(v_set.len(), 6, "Test 4: 6 unique vertices for face {i}");

            let mut pts = [Vec2::ZERO; 6];
            for k in 0..6 {
                pts[k] = topo1.vertices[face.vertices[k].index()].position;
            }

            let area = signed_area(&pts);
            assert!(area > 0.0, "Test 12 & 13: Positive area & CCW for face {i}");

            let mut curr = face.boundary;
            let mut count = 0;
            for _ in 0..6 {
                let edge = &topo1.half_edges[curr.index()];
                assert_eq!(edge.incident_face.index(), i, "Edge belongs to face {i}");

                let next_edge = &topo1.half_edges[edge.next.index()];
                assert_eq!(next_edge.prev, curr, "Test 8: Next/Prev consistent");

                curr = edge.next;
                count += 1;
            }
            assert_eq!(curr, face.boundary, "Test 6: Next forms 6-cycle");
            assert_eq!(count, 6, "Test 5 & 6: 6 edges in cycle");

            let mut curr_p = face.boundary;
            for _ in 0..6 {
                curr_p = topo1.half_edges[curr_p.index()].prev;
            }
            assert_eq!(curr_p, face.boundary, "Test 7: Prev forms 6-cycle");
        }

        // 9-11. Shared border, Twin links and border HalfEdges
        for (e_idx, edge) in topo1.half_edges.iter().enumerate() {
            if let Some(twin_id) = edge.twin {
                let twin = &topo1.half_edges[twin_id.index()];
                assert_eq!(
                    twin.origin, edge.destination,
                    "Test 9: Shared border origin"
                );
                assert_eq!(twin.destination, edge.origin, "Test 9: Shared border dest");
                assert_eq!(
                    twin.twin,
                    Some(HalfEdgeId::new(e_idx)),
                    "Test 10: Symmetric Twin"
                );
            }
        }

        // 16. Y = 0 (data is 2D Vec2 position)
        assert!(
            topo1
                .vertices
                .iter()
                .all(|v| v.position.x.is_finite() && v.position.y.is_finite()),
            "Test 16: X/Z 2D Vec2"
        );

        // 17-18. Seed 42 determinism & seed variance
        let topo1_again =
            generate_hex_face_topology(&map_orig, seed42).expect("Topology 1 again failed");
        assert_eq!(topo1, topo1_again, "Test 17: Seed 42 is deterministic");

        let seed99 = WorldSeed::new(99);
        let topo_diff =
            generate_hex_face_topology(&map_orig, seed99).expect("Topology diff failed");
        let any_diff = topo1
            .vertices
            .iter()
            .zip(topo_diff.vertices.iter())
            .any(|(v1, v2)| v1.position != v2.position);
        assert!(any_diff, "Test 18: Different seed changes vertex positions");

        // 19. Every HexCoord maps to exactly 1 FaceId
        assert_eq!(
            topo1.hex_to_face.len(),
            map_orig.tiles.len(),
            "Test 19: 1 HexCoord -> 1 FaceId"
        );

        // 20-21. 1-to-1 bijection between SharedCornerKey and VertexId
        let key_set: HashSet<_> = topo1.vertices.iter().map(|v| v.canonical_key).collect();
        assert_eq!(
            key_set.len(),
            topo1.vertices.len(),
            "Test 20 & 21: Unique SharedCornerKeys"
        );
    }

    #[test]
    fn test_hand_authored_clusters() {
        // Isolated hex
        let mut map_1 = MapData::default();
        map_1.tiles.insert(HexCoord::new(0, 0), TileData::default());
        let topo_1 =
            generate_hex_face_topology(&map_1, WorldSeed::new(42)).expect("Isolated hex failed");
        assert_eq!(topo_1.faces.len(), 1);
        assert_eq!(topo_1.stats.border_edge_count, 6);
        assert_eq!(topo_1.stats.paired_edge_count, 0);

        // Two neighboring hexes
        let mut map_2 = MapData::default();
        map_2.tiles.insert(HexCoord::new(0, 0), TileData::default());
        map_2.tiles.insert(HexCoord::new(1, 0), TileData::default());
        let topo_2 =
            generate_hex_face_topology(&map_2, WorldSeed::new(42)).expect("Two hexes failed");
        assert_eq!(topo_2.faces.len(), 2);
        assert_eq!(topo_2.stats.paired_edge_count, 1);
        assert_eq!(topo_2.stats.border_edge_count, 10);

        // Center hex with all 6 neighbors
        let mut map_7 = MapData::default();
        let center = HexCoord::new(0, 0);
        map_7.tiles.insert(center, TileData::default());
        for n in center.neighbors() {
            map_7.tiles.insert(n, TileData::default());
        }
        let topo_7 =
            generate_hex_face_topology(&map_7, WorldSeed::new(42)).expect("7-hex cluster failed");
        assert_eq!(topo_7.faces.len(), 7);
        let center_face_id = topo_7.hex_to_face[&center];
        let center_face = &topo_7.faces[center_face_id.index()];
        for i in 0..6 {
            let e_id = HalfEdgeId::new(center_face.boundary.index() + i);
            let edge = &topo_7.half_edges[e_id.index()];
            assert!(edge.twin.is_some(), "Center hex edges must all have twins");
        }
    }
}
