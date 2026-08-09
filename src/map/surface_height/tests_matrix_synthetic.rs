// src/map/surface_height/tests_matrix_synthetic.rs
//! Extended synthetic 4,608-case solver stress matrix.
//! 6 distinct graph families x 256 seeds x 3 solver profiles.
//! Family builders extracted to tests_matrix_builders module.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{CliffLowerSide, EdgeCoord};
    use crate::map::height_graph::types::{
        CliffNodeRelation, HeightConstraintGraph, HeightContinuityEdge, HeightNode, HeightNodeId,
    };
    use crate::map::surface_height::guide::{HeightGuideSample, LegacyHeightGuide};
    use crate::map::surface_height::hard_constraints::compile_hard_constraints;
    use crate::map::surface_height::solver::solve_surface_heights;
    use crate::map::surface_height::targets::compile_height_targets;
    use crate::map::surface_height::types::HeightSolverConfig;
    use crate::map::surface_height::validation::validate_surface_height_layer;
    use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceVertexId};
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceHeightMatrixSyntheticTestsPlugin;

    impl Plugin for SurfaceHeightMatrixSyntheticTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// 6 graph families x 256 seeds x 3 solver profiles = 4,608 cases.
    /// Feasibility invariant: max_path_edges(23) x cliff_min_drop(0.03) = 0.69 < 1.0.
    #[test]
    #[ignore = "Extended 4,608 synthetic graph-level solver matrix"]
    fn synthetic_graph_extended_4608_matrix() {
        let profiles = [
            HeightSolverConfig {
                guide_weight: 8.0,
                region_weight: 1.0,
                smoothness_weight: 0.10,
                cliff_min_drop: 0.03,
                ..Default::default()
            },
            HeightSolverConfig {
                cliff_min_drop: 0.03,
                ..Default::default()
            },
            HeightSolverConfig {
                guide_weight: 2.0,
                region_weight: 4.0,
                smoothness_weight: 0.80,
                cliff_min_drop: 0.03,
                ..Default::default()
            },
        ];

        let mut total_cases = 0;
        for family in 0..6usize {
            for seed in 0..256u32 {
                for config in &profiles {
                    let (graph, guide) = build_family_graph(family, seed);
                    let targets = compile_height_targets(&graph, &guide, config)
                        .unwrap_or_else(|e| panic!("family={family} seed={seed}: targets: {e:?}"));
                    let hard = compile_hard_constraints(&graph, &guide, config)
                        .unwrap_or_else(|e| panic!("family={family} seed={seed}: hard: {e:?}"));
                    let layer = solve_surface_heights(&graph, &guide, &targets, &hard, config)
                        .unwrap_or_else(|e| panic!("family={family} seed={seed}: solver: {e:?}"));
                    validate_surface_height_layer(&layer, &graph, &guide, &hard, config)
                        .unwrap_or_else(|e| panic!("family={family} seed={seed}: validate: {e:?}"));
                    total_cases += 1;
                }
            }
        }
        assert_eq!(total_cases, 4608);
    }

    fn dummy_node_at(idx: usize) -> HeightNode {
        HeightNode {
            surface_vertex: SurfaceVertexId::new(idx),
            incident_faces: vec![SurfaceFaceId::new(idx)],
        }
    }

    fn dummy_guide_for(n: usize, seed: u32, ocean_pin: Option<usize>) -> LegacyHeightGuide {
        LegacyHeightGuide {
            samples: (0..n)
                .map(|i| HeightGuideSample {
                    target: (0.10 + 0.03 * (i as f32 + (seed % 10) as f32)).clamp(0.0, 0.80),
                    hard_pin: if ocean_pin == Some(i) {
                        Some(0.0)
                    } else {
                        None
                    },
                })
                .collect(),
        }
    }

    fn build_family_graph(family: usize, seed: u32) -> (HeightConstraintGraph, LegacyHeightGuide) {
        let n = 10 + (seed as usize % 15); // 10..=24
        match family {
            0 => build_no_cliffs(n, seed),
            1 => build_linear_chain(n, seed),
            2 => build_branching_dag(n, seed),
            3 => build_converging_dag(n, seed),
            4 => build_split_sheet(n, seed),
            _ => build_ocean_pinned_dag(n, seed),
        }
    }

    fn make_cont_edge(i: usize, j: usize) -> HeightContinuityEdge {
        HeightContinuityEdge::new(HeightNodeId::new(i), HeightNodeId::new(j))
    }

    fn make_cliff(i: usize, j: usize, r: i32) -> CliffNodeRelation {
        CliffNodeRelation {
            logical_edge: EdgeCoord::new(HexCoord::new(i as i32, r), HexCoord::new(j as i32, r)),
            surface_vertex: SurfaceVertexId::new(i),
            node_a: HeightNodeId::new(i),
            node_b: HeightNodeId::new(j),
            lower_side: CliffLowerSide::A,
        }
    }

    fn build_no_cliffs(n: usize, seed: u32) -> (HeightConstraintGraph, LegacyHeightGuide) {
        let graph = HeightConstraintGraph {
            nodes: (0..n).map(dummy_node_at).collect(),
            continuity_edges: (0..n.saturating_sub(1))
                .map(|i| make_cont_edge(i, i + 1))
                .collect(),
            ..Default::default()
        };
        (graph, dummy_guide_for(n, seed, None))
    }

    fn build_linear_chain(n: usize, seed: u32) -> (HeightConstraintGraph, LegacyHeightGuide) {
        let graph = HeightConstraintGraph {
            nodes: (0..n).map(dummy_node_at).collect(),
            continuity_edges: (0..n.saturating_sub(1))
                .map(|i| make_cont_edge(i, i + 1))
                .collect(),
            cliff_relations: (0..n.saturating_sub(1))
                .map(|i| make_cliff(i, i + 1, 0))
                .collect(),
            ..Default::default()
        };
        (graph, dummy_guide_for(n, seed, None))
    }

    fn build_branching_dag(n: usize, seed: u32) -> (HeightConstraintGraph, LegacyHeightGuide) {
        let mut cont = Vec::new();
        let mut cliffs = Vec::new();
        if n >= 4 {
            for &(a, b) in &[(0usize, 1), (0, 2), (1, 3), (2, 3)] {
                if a < n && b < n {
                    cont.push(make_cont_edge(a, b));
                    cliffs.push(make_cliff(a, b, 0));
                }
            }
            for i in 4..n.saturating_sub(1) {
                cont.push(make_cont_edge(i, i + 1));
                cliffs.push(make_cliff(i, i + 1, 0));
            }
        } else {
            for i in 0..n.saturating_sub(1) {
                cont.push(make_cont_edge(i, i + 1));
            }
        }
        let graph = HeightConstraintGraph {
            nodes: (0..n).map(dummy_node_at).collect(),
            continuity_edges: cont,
            cliff_relations: cliffs,
            ..Default::default()
        };
        (graph, dummy_guide_for(n, seed, None))
    }

    fn build_converging_dag(n: usize, seed: u32) -> (HeightConstraintGraph, LegacyHeightGuide) {
        let summit = n - 1;
        let graph = HeightConstraintGraph {
            nodes: (0..n).map(dummy_node_at).collect(),
            continuity_edges: (0..summit).map(|i| make_cont_edge(i, summit)).collect(),
            cliff_relations: (0..summit).map(|i| make_cliff(i, summit, 0)).collect(),
            ..Default::default()
        };
        (graph, dummy_guide_for(n, seed, None))
    }

    fn build_split_sheet(n: usize, seed: u32) -> (HeightConstraintGraph, LegacyHeightGuide) {
        let half = n / 2;
        let cont: Vec<_> = (0..half.saturating_sub(1))
            .map(|i| make_cont_edge(i, i + 1))
            .chain((half..n.saturating_sub(1)).map(|i| make_cont_edge(i, i + 1)))
            .collect();
        let cliffs: Vec<_> = (0..half.saturating_sub(1))
            .map(|i| make_cliff(i, i + 1, 0))
            .chain((half..n.saturating_sub(1)).map(|i| make_cliff(i, i + 1, 1)))
            .collect();
        let graph = HeightConstraintGraph {
            nodes: (0..n).map(dummy_node_at).collect(),
            continuity_edges: cont,
            cliff_relations: cliffs,
            ..Default::default()
        };
        (graph, dummy_guide_for(n, seed, None))
    }

    fn build_ocean_pinned_dag(n: usize, seed: u32) -> (HeightConstraintGraph, LegacyHeightGuide) {
        let graph = HeightConstraintGraph {
            nodes: (0..n).map(dummy_node_at).collect(),
            continuity_edges: (0..n.saturating_sub(1))
                .map(|i| make_cont_edge(i, i + 1))
                .collect(),
            cliff_relations: (0..n.saturating_sub(1))
                .map(|i| make_cliff(i, i + 1, 0))
                .collect(),
            ..Default::default()
        };
        (graph, dummy_guide_for(n, seed, Some(0)))
    }
}
