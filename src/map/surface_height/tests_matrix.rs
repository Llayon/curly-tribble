// src/map/surface_height/tests_matrix.rs
//! Canonical 144-case matrix proof, production smoke gate, and synthetic 4,608 combinatorial stress matrix.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{CliffLowerSide, EdgeCoord, MapData, OceanState, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::height_constraints::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::height_graph::types::{
        CliffNodeRelation, HeightConstraintGraph, HeightNode, HeightNodeId,
    };
    use crate::map::surface_height::guide::{
        derive_legacy_height_guide, HeightGuideSample, LegacyHeightGuide,
    };
    use crate::map::surface_height::hard_constraints::compile_hard_constraints;
    use crate::map::surface_height::solver::solve_surface_heights;
    use crate::map::surface_height::targets::compile_height_targets;
    use crate::map::surface_height::types::HeightSolverConfig;
    use crate::map::surface_height::validation::validate_surface_height_layer;
    use crate::map::surface_topology::generate_surface_topology;
    use crate::map::surface_topology::types::SurfaceTopology;
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceHeightMatrixTestsPlugin;

    impl Plugin for SurfaceHeightMatrixTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    fn build_test_surface(map_data: &MapData) -> SurfaceTopology {
        let seed = WorldSeed::new(42);
        let face_top =
            generate_hex_face_topology_with_profile(map_data, seed, HexDeformationProfile::Organic)
                .unwrap();
        generate_surface_topology(&face_top).unwrap()
    }

    #[test]
    fn production_default_landscape_is_solver_feasible() {
        for seed in q::FAST_SEEDS {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);

            let mut map_data = MapData::default();
            let radius = 10i32;
            for q in -radius..=radius {
                for r in -radius..=radius {
                    if (q + r).abs() <= radius {
                        let hex = HexCoord::new(q, r);
                        let elev =
                            0.10 + 0.001 * (q * 13 + r * 17 + (seed as i32)).abs() as f32 % 0.70;
                        map_data.tiles.insert(
                            hex,
                            TileData {
                                elevation: elev,
                                ocean_state: OceanState::Land,
                                ..Default::default()
                            },
                        );
                    }
                }
            }

            let surface = build_test_surface(&map_data);
            let constraints = compile_height_constraints(&map_data, &surface).unwrap();
            let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
            let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();

            let config = HeightSolverConfig::default();
            let targets = compile_height_targets(&graph, &guide, &config).unwrap();
            let hard = compile_hard_constraints(&graph, &guide, &config).unwrap();
            let layer = solve_surface_heights(&graph, &guide, &targets, &hard, &config).unwrap();

            validate_surface_height_layer(&layer, &graph, &guide, &hard, &config).unwrap();
            assert_eq!(layer.heights.len(), graph.nodes.len());
        }
    }

    #[test]
    fn canonical_144_case_surface_height_matrix() {
        let config = HeightSolverConfig::default();

        for (_shape_name, map_template) in q::all_shapes() {
            let mut baseline_bits: Option<Vec<u32>> = None;

            for profile in q::all_profiles() {
                for seed_val in q::FAST_SEEDS {
                    let seed = WorldSeed::new(seed_val);
                    let map_data = map_template.clone();

                    let face_top =
                        generate_hex_face_topology_with_profile(&map_data, seed, profile).unwrap();
                    let surface = generate_surface_topology(&face_top).unwrap();
                    let constraints = compile_height_constraints(&map_data, &surface).unwrap();
                    let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
                    let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();

                    let targets = compile_height_targets(&graph, &guide, &config).unwrap();
                    let hard = compile_hard_constraints(&graph, &guide, &config).unwrap();
                    let layer =
                        solve_surface_heights(&graph, &guide, &targets, &hard, &config).unwrap();

                    validate_surface_height_layer(&layer, &graph, &guide, &hard, &config).unwrap();

                    let bits: Vec<u32> = layer.heights.iter().map(|h| h.to_bits()).collect();
                    if let Some(ref base) = baseline_bits {
                        // Proof of geometry-independence: same logical graph identity produces bit-exact height layer
                        assert_eq!(&bits, base);
                    } else {
                        baseline_bits = Some(bits);
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "Extended 4,608 synthetic graph-level solver matrix"]
    fn synthetic_graph_extended_4608_matrix() {
        // 6 graph families x 256 seeds x 3 solver profiles = 4,608 cases
        let mut total_cases = 0;

        let profiles = [
            HeightSolverConfig {
                guide_weight: 8.0,
                region_weight: 1.0,
                smoothness_weight: 0.10,
                ..Default::default()
            }, // GuideDominant
            HeightSolverConfig::default(), // Balanced
            HeightSolverConfig {
                guide_weight: 2.0,
                region_weight: 4.0,
                smoothness_weight: 0.80,
                ..Default::default()
            }, // Smooth
        ];

        for family in 0..6 {
            for seed in 0..256u32 {
                for config in &profiles {
                    let node_count = 10 + (seed as usize % 15);
                    let mut nodes = Vec::with_capacity(node_count);
                    let mut samples = Vec::with_capacity(node_count);

                    for i in 0..node_count {
                        nodes.push(HeightNode {
                            surface_vertex:
                                crate::map::surface_topology::types::SurfaceVertexId::new(i),
                            incident_faces: vec![
                                crate::map::surface_topology::types::SurfaceFaceId::new(i),
                            ],
                        });
                        let target_val =
                            0.10 + (0.03 * (i as f32 + seed as f32 % 10.0)).clamp(0.0, 0.80);
                        samples.push(HeightGuideSample {
                            target: target_val,
                            hard_pin: if family == 5 && i == 0 {
                                Some(0.0)
                            } else {
                                None
                            },
                        });
                    }

                    let mut cliff_relations = Vec::new();
                    if family != 0 {
                        // Generate deterministic DAG edges
                        for i in 0..(node_count - 1) {
                            if (i + seed as usize) % 2 == 0 {
                                cliff_relations.push(CliffNodeRelation {
                                    logical_edge: EdgeCoord::new(HexCoord::new(i as i32, 0), HexCoord::new((i + 1) as i32, 0)),
                                    surface_vertex: crate::map::surface_topology::types::SurfaceVertexId::new(i),
                                    node_a: HeightNodeId::new(i),
                                    node_b: HeightNodeId::new(i + 1),
                                    lower_side: CliffLowerSide::A,
                                });
                            }
                        }
                    }

                    let graph = HeightConstraintGraph {
                        nodes,
                        cliff_relations,
                        ..Default::default()
                    };
                    let guide = LegacyHeightGuide { samples };

                    let targets = compile_height_targets(&graph, &guide, config).unwrap();
                    let hard = compile_hard_constraints(&graph, &guide, config).unwrap();
                    let layer =
                        solve_surface_heights(&graph, &guide, &targets, &hard, config).unwrap();

                    validate_surface_height_layer(&layer, &graph, &guide, &hard, config).unwrap();
                    total_cases += 1;
                }
            }
        }

        assert_eq!(total_cases, 4608);
    }
}
