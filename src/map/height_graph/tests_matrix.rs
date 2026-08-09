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
        CliffLowerSide, EdgeCoord, EdgeData, EdgeType, LandscapeFeature, WorldSeed,
    };
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::height_constraints::compiler::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::height_graph::validation::validate_height_constraint_graph;
    use crate::map::surface_topology::generator::generate_surface_topology;

    #[test]
    fn canonical_144_case_height_graph_matrix() {
        let mut total_cases = 0;

        for (shape_name, map_template) in q::all_shapes() {
            for seed_val in q::FAST_SEEDS {
                for profile in q::all_profiles() {
                    total_cases += 1;
                    let seed = WorldSeed::new(seed_val);

                    let mut map = map_template.clone();
                    let mut sorted_tiles: Vec<_> = map.tiles.keys().copied().collect();
                    sorted_tiles.sort_by_key(|c| (c.q, c.r));

                    for (idx, &hex) in sorted_tiles.iter().enumerate() {
                        let feature = match idx % 5 {
                            1 => LandscapeFeature::Mountain,
                            2 => LandscapeFeature::Plateau,
                            3 => LandscapeFeature::Lake,
                            4 => LandscapeFeature::River,
                            _ => LandscapeFeature::None,
                        };
                        if let Some(tile) = map.tiles.get_mut(&hex) {
                            tile.landscape_feature = feature;
                        }
                    }

                    let mut cliff_idx = 0;
                    for &hex in &sorted_tiles {
                        for neighbor in hex.neighbors() {
                            if map.tiles.contains_key(&neighbor) {
                                let edge = EdgeCoord::new(hex, neighbor);
                                if !map.edges.contains_key(&edge) {
                                    cliff_idx += 1;
                                    let lower_side = match cliff_idx % 3 {
                                        1 => CliffLowerSide::A,
                                        2 => CliffLowerSide::B,
                                        _ => CliffLowerSide::Unresolved,
                                    };
                                    map.edges.insert(
                                        edge,
                                        EdgeData {
                                            edge_type: EdgeType::Cliff,
                                            cliff_lower_side: lower_side,
                                        },
                                    );
                                }
                            }
                        }
                    }

                    let face_top =
                        generate_hex_face_topology_with_profile(&map, seed, profile).unwrap();
                    let surface = generate_surface_topology(&face_top).unwrap();
                    let constraints = compile_height_constraints(&map, &surface).unwrap();

                    let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
                    validate_height_constraint_graph(&graph, &surface, &constraints).unwrap();

                    assert_eq!(
                        graph.face_nodes.len(),
                        surface.faces.len(),
                        "Face nodes count mismatch in shape {shape_name}"
                    );
                    assert!(
                        graph.nodes.len() >= surface.vertices.len(),
                        "Node count must be >= surface vertices count in shape {shape_name}"
                    );

                    let total_occurrences: usize =
                        graph.face_nodes.iter().map(|fn_arr| fn_arr.len()).sum();
                    assert_eq!(
                        total_occurrences,
                        surface.faces.len() * 3,
                        "Total occurrences must equal face_count * 3 in shape {shape_name}"
                    );
                }
            }
        }

        assert_eq!(total_cases, 144);
    }

    #[test]
    #[ignore = "Extended 4,608 case combinatorial matrix proof for HeightConstraintGraph"]
    fn height_graph_extended_4608_matrix() {
        let mut total_cases = 0;

        for (shape_name, map_template) in q::all_shapes() {
            for seed_val in 0..256u32 {
                for profile in q::all_profiles() {
                    total_cases += 1;
                    let seed = WorldSeed::new(seed_val);

                    let mut map = map_template.clone();
                    let mut sorted_tiles: Vec<_> = map.tiles.keys().copied().collect();
                    sorted_tiles.sort_by_key(|c| (c.q, c.r));

                    for (idx, &hex) in sorted_tiles.iter().enumerate() {
                        let feature = match idx % 5 {
                            1 => LandscapeFeature::Mountain,
                            2 => LandscapeFeature::Plateau,
                            3 => LandscapeFeature::Lake,
                            4 => LandscapeFeature::River,
                            _ => LandscapeFeature::None,
                        };
                        if let Some(tile) = map.tiles.get_mut(&hex) {
                            tile.landscape_feature = feature;
                        }
                    }

                    let mut cliff_idx = 0;
                    for &hex in &sorted_tiles {
                        for neighbor in hex.neighbors() {
                            if map.tiles.contains_key(&neighbor) {
                                let edge = EdgeCoord::new(hex, neighbor);
                                if !map.edges.contains_key(&edge) {
                                    cliff_idx += 1;
                                    let lower_side = match cliff_idx % 3 {
                                        1 => CliffLowerSide::A,
                                        2 => CliffLowerSide::B,
                                        _ => CliffLowerSide::Unresolved,
                                    };
                                    map.edges.insert(
                                        edge,
                                        EdgeData {
                                            edge_type: EdgeType::Cliff,
                                            cliff_lower_side: lower_side,
                                        },
                                    );
                                }
                            }
                        }
                    }

                    let face_top =
                        generate_hex_face_topology_with_profile(&map, seed, profile).unwrap();
                    let surface = generate_surface_topology(&face_top).unwrap();
                    let constraints = compile_height_constraints(&map, &surface).unwrap();

                    let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
                    validate_height_constraint_graph(&graph, &surface, &constraints).unwrap();

                    assert_eq!(
                        graph.face_nodes.len(),
                        surface.faces.len(),
                        "Face nodes count mismatch in shape {shape_name}"
                    );
                    assert!(
                        graph.nodes.len() >= surface.vertices.len(),
                        "Node count must be >= surface vertices count in shape {shape_name}"
                    );
                }
            }
        }

        assert_eq!(total_cases, 4608);
    }
}
