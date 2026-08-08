// src/map/height_constraints/tests_matrix.rs
//! Canonical 144-case proof matrix and extended stress matrix tests for HeightConstraintSet compilation.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightConstraintMatrixTestsPlugin;

impl Plugin for HeightConstraintMatrixTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::EdgeCoord;
    use crate::map::data::{CliffLowerSide, EdgeData, EdgeType, LandscapeFeature, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::height_constraints::compiler::compile_height_constraints;
    use crate::map::height_constraints::validation::validate_height_constraint_set;
    use crate::map::surface_topology::generator::generate_surface_topology;

    #[test]
    fn canonical_144_case_height_constraint_matrix() {
        let mut cases = 0;
        let mut total_regions = 0;
        let mut total_cliffs = 0;

        for (shape, map_template) in q::all_shapes() {
            for seed_val in q::FAST_SEEDS {
                for profile in q::all_profiles() {
                    cases += 1;
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

                    let face_topology =
                        generate_hex_face_topology_with_profile(&map, seed, profile)
                            .expect("Face topology failed");

                    let surface =
                        generate_surface_topology(&face_topology).expect("Surface topology failed");

                    let constraints = compile_height_constraints(&map, &surface)
                        .expect("Height constraints compilation failed");

                    validate_height_constraint_set(&constraints, &map, &surface)
                        .expect("Height constraint set validation failed");

                    let expected_regions_count = map
                        .tiles
                        .values()
                        .filter(|t| t.landscape_feature != LandscapeFeature::None)
                        .count();
                    let expected_cliffs_count = map
                        .edges
                        .values()
                        .filter(|e| e.edge_type == EdgeType::Cliff)
                        .count();

                    assert_eq!(
                        constraints.regions.len(),
                        expected_regions_count,
                        "Shape {shape} seed {seed_val}: region count mismatch"
                    );
                    assert_eq!(
                        constraints.cliffs.len(),
                        expected_cliffs_count,
                        "Shape {shape} seed {seed_val}: cliff count mismatch"
                    );

                    for cliff in &constraints.cliffs {
                        assert_eq!(
                            cliff.segments.len(),
                            2,
                            "Shape {shape} seed {seed_val}: Fixed24 must have exactly 2 surface boundary segments per adjacent cliff"
                        );
                    }

                    total_regions += constraints.regions.len();
                    total_cliffs += constraints.cliffs.len();
                }
            }
        }

        assert_eq!(cases, 144);
        assert!(total_regions > 0);
        assert!(total_cliffs > 0);
    }

    #[test]
    #[ignore]
    fn height_constraint_extended_4608_matrix() {
        let mut cases = 0;
        let mut total_regions = 0;
        let mut total_cliffs = 0;

        for (_shape, map_template) in q::all_shapes() {
            for seed_val in 0..256 {
                for profile in q::all_profiles() {
                    cases += 1;
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

                    let face_topology =
                        generate_hex_face_topology_with_profile(&map, seed, profile)
                            .expect("Face topology failed");

                    let surface =
                        generate_surface_topology(&face_topology).expect("Surface topology failed");

                    let constraints = compile_height_constraints(&map, &surface)
                        .expect("Height constraints compilation failed");

                    validate_height_constraint_set(&constraints, &map, &surface)
                        .expect("Height constraint set validation failed");

                    total_regions += constraints.regions.len();
                    total_cliffs += constraints.cliffs.len();
                }
            }
        }

        assert_eq!(cases, 4608);
        assert!(total_regions > 0);
        assert!(total_cliffs > 0);
    }
}
