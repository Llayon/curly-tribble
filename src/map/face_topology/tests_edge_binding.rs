use bevy::prelude::*;

#[allow(dead_code)]
pub struct EdgeBindingTestsPlugin;

impl Plugin for EdgeBindingTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::data::{
        CliffLowerSide, EdgeCoord, EdgeData, EdgeType, MapData, TileData, WorldSeed,
    };
    use crate::map::face_topology::edge_binding::{bind_cliff_edges, CliffBindingError};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::generation::cliffs::generate_cliffs;
    use crate::map::HexCoord;
    use std::collections::HashMap;

    fn create_two_tile_map() -> MapData {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());
        let edge = EdgeCoord::new(c1, c2);
        map.edges.insert(
            edge,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::A,
            },
        );
        map
    }

    #[test]
    fn canonical_edge_coord_a_b_semantics_held_across_profiles() {
        for profile in [
            HexDeformationProfile::Subtle,
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ] {
            let map = create_two_tile_map();
            let seed = WorldSeed::new(42);
            let topology = generate_hex_face_topology_with_profile(&map, seed, profile)
                .expect("Topology generation failed");
            let bound = bind_cliff_edges(&map, &topology).expect("Edge binding failed");

            assert_eq!(bound.edges.len(), 1);
            let b = &bound.edges[0];
            assert_eq!(
                b.logical_edge,
                EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0))
            );
            assert_eq!(b.lower_side, CliffLowerSide::A);

            let he_a = topology
                .half_edges
                .get(b.half_edge_a.index())
                .expect("he_a missing");
            let he_b = topology
                .half_edges
                .get(b.half_edge_b.index())
                .expect("he_b missing");

            assert_eq!(he_a.incident_face, b.face_a);
            assert_eq!(he_b.incident_face, b.face_b);
            assert_eq!(he_a.twin, Some(b.half_edge_b));
            assert_eq!(he_b.twin, Some(b.half_edge_a));
            assert_eq!(he_a.origin, he_b.destination);
            assert_eq!(he_a.destination, he_b.origin);
        }
    }

    #[test]
    fn fail_fast_typed_error_on_missing_tile_face_or_adjacency() {
        let map = create_two_tile_map();
        let seed = WorldSeed::new(42);
        let mut topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("Topology generation failed");

        let mut invalid_map = map.clone();
        let missing_edge = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(5, 5));
        invalid_map.edges.insert(
            missing_edge,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::B,
            },
        );

        let err =
            bind_cliff_edges(&invalid_map, &topology).expect_err("Should fail on missing tile");
        assert_eq!(err, CliffBindingError::MissingTileB(missing_edge));

        let c2 = HexCoord::new(1, 0);
        topology.hex_to_face.remove(&c2);
        let edge = EdgeCoord::new(HexCoord::new(0, 0), c2);
        let err2 = bind_cliff_edges(&map, &topology).expect_err("Should fail on missing face");
        assert_eq!(err2, CliffBindingError::MissingFaceB(edge));
    }

    #[test]
    fn tile_and_edge_insertion_order_determinism_normal_reverse_lcg() {
        let mut map_normal = MapData::default();
        let mut map_reverse = MapData::default();
        let mut map_lcg = MapData::default();

        let coords: Vec<_> = (0..5)
            .flat_map(|q| (0..5).map(move |r| HexCoord::new(q, r)))
            .collect();

        for &c in &coords {
            map_normal.tiles.insert(c, TileData::default());
        }
        for &c in coords.iter().rev() {
            map_reverse.tiles.insert(c, TileData::default());
        }
        let mut lcg_coords = coords.clone();
        lcg_coords.sort_by_key(|c| {
            c.q.unsigned_abs()
                .wrapping_mul(1664525)
                .wrapping_add(c.r.unsigned_abs().wrapping_mul(1013904223))
                % 4294967291
        });
        for &c in &lcg_coords {
            map_lcg.tiles.insert(c, TileData::default());
        }

        let mut edges = Vec::new();
        for &c in &coords {
            for n in c.neighbors() {
                if coords.contains(&n) {
                    edges.push(EdgeCoord::new(c, n));
                }
            }
        }
        edges.sort_by_key(|e| (e.a, e.b));
        edges.dedup();

        let edge_data = |e: EdgeCoord| -> EdgeData {
            let lower = match (e.a.q + e.b.r).rem_euclid(3) {
                0 => CliffLowerSide::Unresolved,
                1 => CliffLowerSide::A,
                _ => CliffLowerSide::B,
            };
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: lower,
            }
        };

        for &e in &edges {
            map_normal.edges.insert(e, edge_data(e));
        }
        for &e in edges.iter().rev() {
            map_reverse.edges.insert(e, edge_data(e));
        }
        let mut lcg_edges = edges.clone();
        lcg_edges.sort_by_key(|e| {
            e.a.q
                .unsigned_abs()
                .wrapping_mul(1103515245)
                .wrapping_add(e.b.r.unsigned_abs().wrapping_mul(12345))
                % 2147483647
        });
        for &e in &lcg_edges {
            map_lcg.edges.insert(e, edge_data(e));
        }

        let seed = WorldSeed::new(42);
        let top_normal = generate_hex_face_topology_with_profile(
            &map_normal,
            seed,
            HexDeformationProfile::Subtle,
        )
        .unwrap();
        let top_reverse = generate_hex_face_topology_with_profile(
            &map_reverse,
            seed,
            HexDeformationProfile::Subtle,
        )
        .unwrap();
        let top_lcg =
            generate_hex_face_topology_with_profile(&map_lcg, seed, HexDeformationProfile::Subtle)
                .unwrap();

        let bound_normal = bind_cliff_edges(&map_normal, &top_normal).unwrap();
        let bound_reverse = bind_cliff_edges(&map_reverse, &top_reverse).unwrap();
        let bound_lcg = bind_cliff_edges(&map_lcg, &top_lcg).unwrap();

        assert_eq!(bound_normal, bound_reverse);
        assert_eq!(bound_normal, bound_lcg);
    }

    #[test]
    fn generate_cliffs_insertion_order_determinism() {
        let mut map_a = MapData::default();
        let mut map_b = MapData::default();

        let coords: Vec<_> = (0..6)
            .flat_map(|q| (0..6).map(move |r| HexCoord::new(q, r)))
            .collect();

        for &c in &coords {
            let mut tile = TileData::default();
            if (c.q + c.r) % 3 == 0 {
                tile.landscape_feature = crate::map::LandscapeFeature::Mountain;
            }
            map_a.tiles.insert(c, tile.clone());
        }
        for &c in coords.iter().rev() {
            let mut tile = TileData::default();
            if (c.q + c.r) % 3 == 0 {
                tile.landscape_feature = crate::map::LandscapeFeature::Mountain;
            }
            map_b.tiles.insert(c, tile);
        }

        let distance_field: HashMap<HexCoord, u32> = coords
            .iter()
            .map(|&c| (c, (c.q.abs() + c.r.abs()) as u32))
            .collect();

        generate_cliffs(&mut map_a, &distance_field, 42);
        generate_cliffs(&mut map_b, &distance_field, 42);

        assert_eq!(map_a.edges, map_b.edges);
    }

    #[test]
    fn fast_144_case_cliff_binding_matrix() {
        let mut cases = 0;
        for (shape, map) in q::all_shapes() {
            for seed_val in q::FAST_SEEDS {
                for profile in q::all_profiles() {
                    cases += 1;
                    let seed = WorldSeed::new(seed_val);
                    let mut test_map = map.clone();

                    let unique_coords: Vec<_> = test_map.tiles.keys().copied().collect();
                    for &coord in &unique_coords {
                        for n in coord.neighbors() {
                            if test_map.tiles.contains_key(&n) {
                                let edge = EdgeCoord::new(coord, n);
                                test_map.edges.entry(edge).or_insert(EdgeData {
                                    edge_type: EdgeType::Cliff,
                                    cliff_lower_side: if (coord.q + coord.r) % 2 == 0 {
                                        CliffLowerSide::A
                                    } else {
                                        CliffLowerSide::B
                                    },
                                });
                            }
                        }
                    }

                    let topology =
                        generate_hex_face_topology_with_profile(&test_map, seed, profile)
                            .expect("Topology generation failed");

                    let bound = bind_cliff_edges(&test_map, &topology)
                        .expect("144-matrix edge binding failed");

                    if shape == "1x1" {
                        assert_eq!(
                            bound.edges.len(),
                            0,
                            "1x1 shape must have 0 bound cliff edges"
                        );
                    }

                    for b in &bound.edges {
                        let he_a = topology
                            .half_edges
                            .get(b.half_edge_a.index())
                            .expect("he_a missing");
                        let he_b = topology
                            .half_edges
                            .get(b.half_edge_b.index())
                            .expect("he_b missing");

                        assert_eq!(he_a.incident_face, b.face_a);
                        assert_eq!(he_b.incident_face, b.face_b);
                        assert_eq!(he_a.twin, Some(b.half_edge_b));
                        assert_eq!(he_b.twin, Some(b.half_edge_a));
                    }
                }
            }
        }

        assert_eq!(cases, 144);
    }
}
