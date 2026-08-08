use bevy::prelude::*;

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
    use crate::map::HexCoord;

    fn generate_test_shape(shape_id: usize) -> MapData {
        let mut map = MapData::default();
        let size = match shape_id {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 4,
            4 => 5,
            _ => 6,
        };
        for q in -size..=size {
            for r in -size..=size {
                let sum: i32 = q + r;
                if sum.abs() <= size {
                    map.tiles.insert(HexCoord::new(q, r), TileData::default());
                }
            }
        }
        map
    }

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

            let he_a = &topology.half_edges[b.half_edge_a.index()];
            let he_b = &topology.half_edges[b.half_edge_b.index()];

            assert_eq!(he_a.incident_face, b.face_a);
            assert_eq!(he_b.incident_face, b.face_b);
            assert_eq!(he_a.twin, Some(b.half_edge_b));
            assert_eq!(he_b.twin, Some(b.half_edge_a));
            assert_eq!(he_a.origin, he_b.destination);
            assert_eq!(he_a.destination, he_b.origin);
        }
    }

    #[test]
    fn fail_fast_typed_error_on_missing_tile_or_face() {
        let map = create_two_tile_map();
        let seed = WorldSeed::new(42);
        let topology =
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
    }

    #[test]
    fn insertion_order_determinism_holds() {
        let mut map_a = MapData::default();
        let mut map_b = MapData::default();
        let coords = [
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(0, 1),
        ];

        for &c in &coords {
            map_a.tiles.insert(c, TileData::default());
            map_b.tiles.insert(c, TileData::default());
        }

        let edges = [
            (EdgeCoord::new(coords[0], coords[1]), CliffLowerSide::A),
            (EdgeCoord::new(coords[0], coords[2]), CliffLowerSide::B),
            (
                EdgeCoord::new(coords[1], coords[2]),
                CliffLowerSide::Unresolved,
            ),
        ];

        for &(e, lower) in &edges {
            map_a.edges.insert(
                e,
                EdgeData {
                    edge_type: EdgeType::Cliff,
                    cliff_lower_side: lower,
                },
            );
        }
        for &(e, lower) in edges.iter().rev() {
            map_b.edges.insert(
                e,
                EdgeData {
                    edge_type: EdgeType::Cliff,
                    cliff_lower_side: lower,
                },
            );
        }

        let seed = WorldSeed::new(100);
        let top_a =
            generate_hex_face_topology_with_profile(&map_a, seed, HexDeformationProfile::Organic)
                .expect("Topology failed");
        let top_b =
            generate_hex_face_topology_with_profile(&map_b, seed, HexDeformationProfile::Organic)
                .expect("Topology failed");

        let bound_a = bind_cliff_edges(&map_a, &top_a).expect("Binding failed");
        let bound_b = bind_cliff_edges(&map_b, &top_b).expect("Binding failed");

        assert_eq!(bound_a, bound_b);
    }

    #[test]
    fn fast_144_case_cliff_binding_matrix() {
        let profiles = [
            HexDeformationProfile::Subtle,
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ];
        let shapes = 0..6usize;
        let seeds = [1u32, 7, 42, 101, 169, 203, 500, 999];

        let mut case_count = 0;

        for &profile in &profiles {
            for shape_id in shapes.clone() {
                for &seed_val in &seeds {
                    case_count += 1;
                    let seed = WorldSeed::new(seed_val);
                    let mut map = generate_test_shape(shape_id);

                    let unique_coords: Vec<_> = map.tiles.keys().copied().collect();
                    for &coord in &unique_coords {
                        for n in coord.neighbors() {
                            if map.tiles.contains_key(&n) {
                                let edge = EdgeCoord::new(coord, n);
                                map.edges.entry(edge).or_insert(EdgeData {
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

                    let topology = generate_hex_face_topology_with_profile(&map, seed, profile)
                        .expect("Topology generation failed");

                    let bound =
                        bind_cliff_edges(&map, &topology).expect("144-matrix edge binding failed");

                    for b in &bound.edges {
                        let he_a = &topology.half_edges[b.half_edge_a.index()];
                        let he_b = &topology.half_edges[b.half_edge_b.index()];
                        assert_eq!(he_a.incident_face, b.face_a);
                        assert_eq!(he_b.incident_face, b.face_b);
                        assert_eq!(he_a.twin, Some(b.half_edge_b));
                        assert_eq!(he_b.twin, Some(b.half_edge_a));
                    }
                }
            }
        }

        assert_eq!(case_count, 144);
    }
}
