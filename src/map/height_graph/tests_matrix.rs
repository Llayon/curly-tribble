// src/map/height_graph/tests_matrix.rs
//! Canonical 144-case and extended 4,608-case combinatorial matrix proof suite for HeightConstraintGraph.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightGraphMatrixTestsPlugin;

impl Plugin for HeightGraphMatrixTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{
        CliffLowerSide, EdgeCoord, EdgeData, EdgeType, LandscapeFeature, MapData, TileData,
        WorldSeed,
    };
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compiler::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::height_graph::validation::validate_height_constraint_graph;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::HexCoord;

    const FAST_SEEDS: [u32; 8] = [42, 99, 123, 777, 2024, 9999, 12345, 54321];
    const PROFILES: [HexDeformationProfile; 3] = [
        HexDeformationProfile::Subtle,
        HexDeformationProfile::Organic,
        HexDeformationProfile::PagoniaLike,
    ];

    fn generate_shape_map(shape_idx: usize) -> MapData {
        let mut map = MapData::default();
        let coords = match shape_idx {
            0 => vec![HexCoord::new(0, 0)],
            1 => vec![HexCoord::new(0, 0), HexCoord::new(1, 0)],
            2 => vec![
                HexCoord::new(0, 0),
                HexCoord::new(1, 0),
                HexCoord::new(0, 1),
            ],
            3 => vec![
                HexCoord::new(0, 0),
                HexCoord::new(1, 0),
                HexCoord::new(0, 1),
                HexCoord::new(-1, 1),
            ],
            4 => vec![
                HexCoord::new(0, 0),
                HexCoord::new(1, 0),
                HexCoord::new(0, 1),
                HexCoord::new(-1, 1),
                HexCoord::new(-1, 0),
            ],
            _ => vec![
                HexCoord::new(0, 0),
                HexCoord::new(1, 0),
                HexCoord::new(0, 1),
                HexCoord::new(-1, 1),
                HexCoord::new(-1, 0),
                HexCoord::new(0, -1),
                HexCoord::new(1, -1),
            ],
        };

        for (idx, &c) in coords.iter().enumerate() {
            let feature = match idx % 4 {
                0 => LandscapeFeature::Mountain,
                1 => LandscapeFeature::Plateau,
                2 => LandscapeFeature::Lake,
                _ => LandscapeFeature::None,
            };
            map.tiles.insert(
                c,
                TileData {
                    landscape_feature: feature,
                    ..Default::default()
                },
            );
        }

        if coords.len() >= 2 {
            let e = EdgeCoord::new(coords[0], coords[1]);
            map.edges.insert(
                e,
                EdgeData {
                    edge_type: EdgeType::Cliff,
                    cliff_lower_side: CliffLowerSide::A,
                },
            );
        }

        map
    }

    #[test]
    fn canonical_144_case_height_graph_matrix() {
        let mut total_cases = 0;

        for shape_idx in 0..6 {
            let map = generate_shape_map(shape_idx);
            for profile in PROFILES {
                for seed_val in FAST_SEEDS {
                    total_cases += 1;
                    let seed = WorldSeed::new(seed_val);
                    let face_top =
                        generate_hex_face_topology_with_profile(&map, seed, profile).unwrap();
                    let surface = generate_surface_topology(&face_top).unwrap();
                    let constraints = compile_height_constraints(&map, &surface).unwrap();

                    let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
                    validate_height_constraint_graph(&graph, &surface, &constraints).unwrap();

                    assert_eq!(graph.face_nodes.len(), surface.faces.len());
                    assert!(graph.nodes.len() >= surface.vertices.len());
                }
            }
        }

        assert_eq!(total_cases, 144);
    }

    #[test]
    #[ignore = "Extended 4,608 case combinatorial matrix proof for HeightConstraintGraph"]
    fn height_graph_extended_4608_matrix() {
        let mut total_cases = 0;

        for shape_idx in 0..6 {
            let map = generate_shape_map(shape_idx);
            for profile in PROFILES {
                for seed_val in 0..256u32 {
                    total_cases += 1;
                    let seed = WorldSeed::new(seed_val);
                    let face_top =
                        generate_hex_face_topology_with_profile(&map, seed, profile).unwrap();
                    let surface = generate_surface_topology(&face_top).unwrap();
                    let constraints = compile_height_constraints(&map, &surface).unwrap();

                    let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
                    validate_height_constraint_graph(&graph, &surface, &constraints).unwrap();

                    assert_eq!(graph.face_nodes.len(), surface.faces.len());
                    assert!(graph.nodes.len() >= surface.vertices.len());
                }
            }
        }

        assert_eq!(total_cases, 4608);
    }
}
