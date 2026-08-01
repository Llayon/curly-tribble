/// Stress tests, golden vectors, and cluster tests for hex face topology.
#[cfg(test)]
mod stress_tests {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::corner_key::{canonical_corner_key, seed_for_corner};
    use crate::map::face_topology::generator::generate_hex_face_topology;
    use crate::map::face_topology::types::{HalfEdgeId, SharedCornerKey};
    use crate::map::face_topology::validate_complete_topology;
    use crate::map::HexCoord;
    use crate::map::WorldSeed;

    fn map_40x40() -> MapData {
        let mut map = MapData::default();
        for r in 0..40 {
            let r_offset = r >> 1;
            for q in -r_offset..(40 - r_offset) {
                map.tiles.insert(HexCoord::new(q, r), TileData::default());
            }
        }
        map
    }

    fn map_40x40_reverse_insertion() -> MapData {
        let mut coords = Vec::new();
        for r in 0..40 {
            let r_offset = r >> 1;
            for q in -r_offset..(40 - r_offset) {
                coords.push(HexCoord::new(q, r));
            }
        }
        coords.sort_by_key(|coord| (coord.q, coord.r));
        let mut map = MapData::default();
        for coord in coords.into_iter().rev() {
            map.tiles.insert(coord, TileData::default());
        }
        map
    }

    fn isolated_hex() -> MapData {
        let mut m = MapData::default();
        m.tiles.insert(HexCoord::new(0, 0), TileData::default());
        m
    }

    fn two_neighbors() -> MapData {
        let mut m = MapData::default();
        m.tiles.insert(HexCoord::new(0, 0), TileData::default());
        m.tiles.insert(HexCoord::new(1, 0), TileData::default());
        m
    }

    fn seven_hex_cluster() -> MapData {
        let mut m = MapData::default();
        let center = HexCoord::new(0, 0);
        m.tiles.insert(center, TileData::default());
        for n in center.neighbors() {
            m.tiles.insert(n, TileData::default());
        }
        m
    }

    fn sparse_l_shape() -> MapData {
        let mut m = MapData::default();
        for coord in [
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(2, 0),
            HexCoord::new(0, 1),
            HexCoord::new(0, 2),
        ] {
            m.tiles.insert(coord, TileData::default());
        }
        m
    }

    fn diagonal_strip() -> MapData {
        let mut m = MapData::default();
        for coord in [
            HexCoord::new(0, 0),
            HexCoord::new(1, 1),
            HexCoord::new(2, 2),
            HexCoord::new(3, 3),
        ] {
            m.tiles.insert(coord, TileData::default());
        }
        m
    }

    #[test]
    fn test_seed_42_is_deterministic() {
        let map = map_40x40();
        let seed = WorldSeed::new(42);
        let t1 = generate_hex_face_topology(&map, seed).expect("gen 1");
        let t2 = generate_hex_face_topology(&map, seed).expect("gen 2");
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_hashmap_iteration_order_does_not_change_topology() {
        let ordered = map_40x40();
        let reverse = map_40x40_reverse_insertion();
        let t1 = generate_hex_face_topology(&ordered, WorldSeed::new(42)).expect("ordered");
        let t2 = generate_hex_face_topology(&reverse, WorldSeed::new(42)).expect("reverse");
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_different_seed_produces_different_positions() {
        let map = map_40x40();
        let t42 = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("42");
        let t99 = generate_hex_face_topology(&map, WorldSeed::new(99)).expect("99");
        let diff = t42
            .vertices
            .iter()
            .zip(t99.vertices.iter())
            .any(|(a, b)| a.position != b.position);
        assert!(diff, "Different seeds must produce different vertices");
    }

    #[test]
    fn test_isolated_hex_topology() {
        let map = isolated_hex();
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("isolated");
        assert_eq!(topo.faces.len(), 1);
        assert_eq!(topo.stats.border_edge_count, 6);
        assert_eq!(topo.stats.paired_edge_count, 0);
        validate_complete_topology(&topo, &map).expect("validation");
    }

    #[test]
    fn test_two_neighbor_topology() {
        let map = two_neighbors();
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("two");
        assert_eq!(topo.faces.len(), 2);
        assert_eq!(topo.stats.paired_edge_count, 1);
        assert_eq!(topo.stats.border_edge_count, 10);
        validate_complete_topology(&topo, &map).expect("validation");
    }

    #[test]
    fn test_seven_hex_cluster_topology() {
        let map = seven_hex_cluster();
        let topo = generate_hex_face_topology(&map, WorldSeed::new(42)).expect("7-hex");
        assert_eq!(topo.faces.len(), 7);
        validate_complete_topology(&topo, &map).expect("validation");
        let center = HexCoord::new(0, 0);
        let cf = &topo.faces[topo.hex_to_face[&center].index()];
        for i in 0..6 {
            let e = HalfEdgeId::new(cf.boundary.index() + i);
            assert!(topo.half_edges[e.index()].twin.is_some(), "Center edge {i}");
        }
    }

    #[test]
    fn test_stable_corner_seed_golden_vectors() {
        let key_origin = SharedCornerKey::new(
            HexCoord::new(-1, 0),
            HexCoord::new(0, -1),
            HexCoord::new(0, 0),
        );
        let key_far = SharedCornerKey::new(
            HexCoord::new(10, 10),
            HexCoord::new(10, 11),
            HexCoord::new(11, 10),
        );

        let h1 = seed_for_corner(42, key_origin);
        let h2 = seed_for_corner(42, key_far);
        let h3 = seed_for_corner(99, key_origin);

        assert_eq!(h1, seed_for_corner(42, key_origin));
        assert_eq!(h2, seed_for_corner(42, key_far));
        assert_ne!(h1, h3);
        assert_ne!(h1, h2);

        // Pinned golden values (fixed by the stable mixer algorithm)
        assert_eq!(h1, 691_015_723_763_045_943, "Golden vector 1");
        assert_eq!(h2, 12_812_762_374_463_184_579, "Golden vector 2");
        assert_eq!(h3, 3_724_955_678_604_950_531, "Golden vector 3");
    }

    #[test]
    fn test_seed_for_corner_is_order_independent() {
        let key = canonical_corner_key(HexCoord::new(0, 0), 0);
        let n0 = HexCoord::new(0, 0).neighbors()[0];
        let n5 = HexCoord::new(0, 0).neighbors()[5];
        let mut found_n0 = false;
        for i in 0..6 {
            if canonical_corner_key(n0, i) == key {
                assert_eq!(
                    seed_for_corner(42, canonical_corner_key(n0, i)),
                    seed_for_corner(42, key)
                );
                found_n0 = true;
            }
        }
        assert!(found_n0, "n0 should share corner");
        let mut found_n5 = false;
        for i in 0..6 {
            if canonical_corner_key(n5, i) == key {
                assert_eq!(
                    seed_for_corner(42, canonical_corner_key(n5, i)),
                    seed_for_corner(42, key)
                );
                found_n5 = true;
            }
        }
        assert!(found_n5, "n5 should share corner");
    }

    #[test]
    fn test_multi_seed_stress_256_seeds() {
        let maps: Vec<(&str, MapData)> = vec![
            ("40x40", map_40x40()),
            ("isolated", isolated_hex()),
            ("two_neighbors", two_neighbors()),
            ("seven_hex", seven_hex_cluster()),
            ("l_shape", sparse_l_shape()),
            ("diagonal", diagonal_strip()),
        ];
        let mut reduced_displacements = 0;
        let mut regular_fallbacks = 0;

        for seed_val in 0..256u32 {
            let seed = WorldSeed::new(seed_val);
            for (name, map) in &maps {
                let topo = generate_hex_face_topology(map, seed)
                    .unwrap_or_else(|e| panic!("seed={seed_val} map={name}: {e:?}"));
                validate_complete_topology(&topo, map)
                    .unwrap_or_else(|e| panic!("seed={seed_val} map={name}: {e:?}"));
                reduced_displacements += topo.stats.reduced_displacement_fallbacks;
                regular_fallbacks += topo.stats.regular_position_fallbacks;
            }
        }
        assert_eq!(
            reduced_displacements, 0,
            "unexpected displacement reduction"
        );
        assert_eq!(regular_fallbacks, 0, "unexpected regular position fallback");
    }
}
