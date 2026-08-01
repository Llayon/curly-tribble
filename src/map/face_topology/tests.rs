/// Core unit tests for hex face topology.
#[cfg(test)]
mod unit_tests {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::corner_key::{canonical_corner_key, regular_corner_position};
    use crate::map::face_topology::generator::generate_hex_face_topology;
    use crate::map::face_topology::types::{FaceId, HalfEdgeId, SharedCornerKey};
    use crate::map::face_topology::validate_complete_topology;
    use crate::map::face_topology::validation::{
        min_edge_length, segments_intersect, signed_area, validate_face_geometry, MIN_EDGE_LENGTH,
    };
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
    fn test_map_data_unchanged_after_generation() {
        let map = generate_test_map(40, 40);
        let tile_count = map.tiles.len();
        let (width, height) = (map.width, map.height);
        let _topo =
            generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        assert_eq!(map.tiles.len(), tile_count);
        assert_eq!(map.width, width);
        assert_eq!(map.height, height);
    }

    #[test]
    fn test_one_face_per_tile() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        assert_eq!(topo.faces.len(), map.tiles.len());
    }

    #[test]
    fn test_hex_to_face_complete_bijection() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        assert_eq!(topo.hex_to_face.len(), map.tiles.len());
        for &coord in map.tiles.keys() {
            assert_eq!(topo.faces[topo.hex_to_face[&coord].index()].hex, coord);
        }
    }

    #[test]
    fn test_six_unique_vertices_per_face() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (i, face) in topo.faces.iter().enumerate() {
            let v_set: HashSet<_> = face.vertices.iter().copied().collect();
            assert_eq!(v_set.len(), 6, "Face {i} does not have 6 unique vertices");
        }
    }

    #[test]
    fn test_next_cycle_closes_in_six_steps() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (i, face) in topo.faces.iter().enumerate() {
            let mut curr = face.boundary;
            for _ in 0..6 {
                curr = topo.half_edges[curr.index()].next;
            }
            assert_eq!(curr, face.boundary, "Face {i} Next cycle not closed");
        }
    }

    #[test]
    fn test_prev_cycle_closes_in_six_steps() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (i, face) in topo.faces.iter().enumerate() {
            let mut curr = face.boundary;
            for _ in 0..6 {
                curr = topo.half_edges[curr.index()].prev;
            }
            assert_eq!(curr, face.boundary, "Face {i} Prev cycle not closed");
        }
    }

    #[test]
    fn test_next_prev_mutual_consistency() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (i, face) in topo.faces.iter().enumerate() {
            let mut curr = face.boundary;
            for _ in 0..6 {
                let edge = &topo.half_edges[curr.index()];
                let next_edge = &topo.half_edges[edge.next.index()];
                assert_eq!(next_edge.prev, curr, "Face {i} next/prev inconsistent");
                curr = edge.next;
            }
        }
    }

    #[test]
    fn test_edge_incident_face_correctness() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (f_idx, face) in topo.faces.iter().enumerate() {
            let mut curr = face.boundary;
            for _ in 0..6 {
                assert_eq!(topo.half_edges[curr.index()].incident_face.index(), f_idx);
                curr = topo.half_edges[curr.index()].next;
            }
        }
    }

    #[test]
    fn test_positive_area_and_ccw_winding() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (i, face) in topo.faces.iter().enumerate() {
            let mut pts = [Vec2::ZERO; 6];
            for k in 0..6 {
                pts[k] = topo.vertices[face.vertices[k].index()].position;
            }
            validate_face_geometry(&pts, FaceId::new(i)).expect("production geometry validation");
            assert!(signed_area(&pts) > 0.0, "Face {i} non-positive area");
        }
    }

    #[test]
    fn test_strict_convexity_every_face() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (i, face) in topo.faces.iter().enumerate() {
            let mut pts = [Vec2::ZERO; 6];
            for k in 0..6 {
                pts[k] = topo.vertices[face.vertices[k].index()].position;
            }
            for j in 0..6 {
                let v1 = pts[(j + 1) % 6] - pts[j];
                let v2 = pts[(j + 2) % 6] - pts[(j + 1) % 6];
                assert!(v1.x * v2.y - v1.y * v2.x > 0.0, "Face {i} corner {j}");
            }
        }
    }

    #[test]
    fn test_minimum_edge_threshold() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (i, face) in topo.faces.iter().enumerate() {
            let mut pts = [Vec2::ZERO; 6];
            for k in 0..6 {
                pts[k] = topo.vertices[face.vertices[k].index()].position;
            }
            assert!(
                min_edge_length(&pts) > MIN_EDGE_LENGTH,
                "Face {i} near-zero edge"
            );
        }
    }

    #[test]
    fn test_no_self_intersections() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (f_idx, face) in topo.faces.iter().enumerate() {
            let mut pts = [Vec2::ZERO; 6];
            for k in 0..6 {
                pts[k] = topo.vertices[face.vertices[k].index()].position;
            }
            for i in 0..6 {
                for j in (i + 2)..6 {
                    if i == 0 && j == 5 {
                        continue;
                    }
                    assert!(
                        !segments_intersect(pts[i], pts[(i + 1) % 6], pts[j], pts[(j + 1) % 6]),
                        "Face {f_idx} edges {i} and {j} intersect"
                    );
                }
            }
        }
    }

    #[test]
    fn test_production_geometry_validation_every_face() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (i, face) in topo.faces.iter().enumerate() {
            let mut pts = [Vec2::ZERO; 6];
            for k in 0..6 {
                pts[k] = topo.vertices[face.vertices[k].index()].position;
            }
            validate_face_geometry(&pts, crate::map::face_topology::FaceId::new(i))
                .unwrap_or_else(|e| panic!("Face {i} failed: {e:?}"));
        }
    }

    #[test]
    fn test_twin_symmetric_and_reversed() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (e_idx, edge) in topo.half_edges.iter().enumerate() {
            if let Some(twin_id) = edge.twin {
                let twin = &topo.half_edges[twin_id.index()];
                assert_eq!(twin.twin, Some(HalfEdgeId::new(e_idx)));
                assert_eq!(twin.origin, edge.destination);
                assert_eq!(twin.destination, edge.origin);
            }
        }
    }

    #[test]
    fn test_twin_faces_are_logical_neighbors() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        for (e_idx, edge) in topo.half_edges.iter().enumerate() {
            if let Some(twin_id) = edge.twin {
                let twin = &topo.half_edges[twin_id.index()];
                let hex_a = topo.faces[edge.incident_face.index()].hex;
                let hex_b = topo.faces[twin.incident_face.index()].hex;
                assert!(
                    hex_a.neighbors().contains(&hex_b),
                    "Edge {e_idx} twin mismatch"
                );
            }
        }
    }

    #[test]
    fn test_border_edges_have_no_twin() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        crate::map::face_topology::validation_twins::validate_border_edges(&topo, &map)
            .expect("Border edges should all be valid outer boundaries");
        assert_eq!(
            topo.half_edges
                .iter()
                .filter(|edge| edge.twin.is_none())
                .count(),
            topo.stats.border_edge_count
        );
    }

    #[test]
    fn test_edge_count_invariant() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        assert_eq!(
            topo.stats.paired_edge_count * 2 + topo.stats.border_edge_count,
            topo.half_edges.len()
        );
    }

    #[test]
    fn test_shared_corner_key_bijection() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        let key_set: HashSet<_> = topo.vertices.iter().map(|v| v.canonical_key).collect();
        assert_eq!(key_set.len(), topo.vertices.len());
    }

    #[test]
    fn test_complete_topology_validation_40x40() {
        let map = generate_test_map(40, 40);
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("generation failed");
        validate_complete_topology(&topo, &map).expect("validation failed");
    }

    #[test]
    fn test_regular_corner_position_returns_ok() {
        let coord = HexCoord::new(0, 0);
        for i in 0..6 {
            let key = canonical_corner_key(coord, i);
            let pos = regular_corner_position(key).expect("should be Ok");
            assert!(pos.x.is_finite() && pos.y.is_finite());
        }
    }

    #[test]
    fn test_regular_corner_position_rejects_unknown_key() {
        let key = SharedCornerKey::new(
            HexCoord::new(0, 0),
            HexCoord::new(100, 100),
            HexCoord::new(200, 200),
        );
        assert!(regular_corner_position(key).is_err());
    }
}
