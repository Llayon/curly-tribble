// src/map/surface_height/tests_cliffs.rs
//! Focused unit tests for cliff hard-constraint numeric invariants.
//! Tests operate on pure functions (no Bevy app).

#[cfg(test)]
pub mod tests {
    use crate::map::data::{CliffLowerSide, EdgeCoord};
    use crate::map::height_graph::types::{
        CliffNodeRelation, HeightConstraintGraph, HeightContinuityEdge, HeightNode, HeightNodeId,
    };
    use crate::map::surface_height::guide::{HeightGuideSample, LegacyHeightGuide};
    use crate::map::surface_height::hard_constraints::{
        compile_hard_constraints, HeightHardConstraintError,
    };
    use crate::map::surface_height::solver::solve_surface_heights;
    use crate::map::surface_height::targets::compile_height_targets;
    use crate::map::surface_height::types::HeightSolverConfig;
    use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceVertexId};
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceHeightCliffsTestsPlugin;

    impl Plugin for SurfaceHeightCliffsTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    fn dummy_node(idx: usize) -> HeightNode {
        HeightNode {
            surface_vertex: SurfaceVertexId::new(idx),
            incident_faces: vec![SurfaceFaceId::new(idx)],
        }
    }

    fn dummy_guide(n: usize) -> LegacyHeightGuide {
        LegacyHeightGuide {
            samples: (0..n)
                .map(|i| HeightGuideSample {
                    target: (0.10 + 0.08 * i as f32).clamp(0.0, 1.0),
                    hard_pin: None,
                })
                .collect(),
        }
    }

    fn two_node_graph_with_relation(lower_side: CliffLowerSide) -> HeightConstraintGraph {
        let logical_edge = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        HeightConstraintGraph {
            nodes: vec![dummy_node(0), dummy_node(1)],
            continuity_edges: vec![HeightContinuityEdge::new(
                HeightNodeId::new(0),
                HeightNodeId::new(1),
            )],
            cliff_relations: vec![CliffNodeRelation {
                logical_edge,
                surface_vertex: SurfaceVertexId::new(0),
                node_a: HeightNodeId::new(0),
                node_b: HeightNodeId::new(1),
                lower_side,
            }],
            ..Default::default()
        }
    }

    /// CliffLowerSide::A: node_a (index 0) is the lower node in compiled constraints.
    #[test]
    fn cliff_lower_a_produces_correct_lower_node() {
        let graph = two_node_graph_with_relation(CliffLowerSide::A);
        let guide = dummy_guide(2);
        let config = HeightSolverConfig::default();
        let constraints = compile_hard_constraints(&graph, &guide, &config).unwrap();
        assert_eq!(constraints.edges.len(), 1);
        assert_eq!(
            constraints.edges[0].lower_node.index(),
            0,
            "lower_side::A => node_a (0) should be lower"
        );
        assert_eq!(constraints.edges[0].higher_node.index(), 1);
    }

    /// CliffLowerSide::B: node_b (index 1) is the lower node in compiled constraints.
    #[test]
    fn cliff_lower_b_produces_correct_lower_node() {
        let graph = two_node_graph_with_relation(CliffLowerSide::B);
        let guide = dummy_guide(2);
        let config = HeightSolverConfig::default();
        let constraints = compile_hard_constraints(&graph, &guide, &config).unwrap();
        assert_eq!(constraints.edges.len(), 1);
        assert_eq!(
            constraints.edges[0].lower_node.index(),
            1,
            "lower_side::B => node_b (1) should be lower"
        );
        assert_eq!(constraints.edges[0].higher_node.index(), 0);
    }

    /// Unresolved cliff: excluded from hard constraint edges; counted in layer stats.
    #[test]
    fn unresolved_cliff_excluded_from_hard_constraints_counted_in_stats() {
        let graph = two_node_graph_with_relation(CliffLowerSide::Unresolved);
        let guide = dummy_guide(2);
        let config = HeightSolverConfig::default();

        // Part 1: compile_hard_constraints produces no edges for Unresolved
        let constraints = compile_hard_constraints(&graph, &guide, &config).unwrap();
        assert!(
            constraints.edges.is_empty(),
            "Unresolved cliff should produce no hard constraint edges"
        );

        // Part 2: solved layer stats count the unresolved relation
        let targets = compile_height_targets(&graph, &guide, &config).unwrap();
        let layer = solve_surface_heights(&graph, &guide, &targets, &constraints, &config).unwrap();
        assert_eq!(
            layer.stats.unresolved_cliff_relation_count, 1,
            "Unresolved cliff should be counted in stats.unresolved_cliff_relation_count"
        );
        assert_eq!(
            layer.stats.resolved_cliff_constraint_count, 0,
            "Unresolved cliff should not appear in resolved_cliff_constraint_count"
        );
    }

    /// Collapsed taper: relation where node_a == node_b is silently skipped.
    #[test]
    fn collapsed_taper_same_node_skipped_silently() {
        let logical_edge = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        let graph = HeightConstraintGraph {
            nodes: vec![dummy_node(0), dummy_node(1)],
            continuity_edges: vec![HeightContinuityEdge::new(
                HeightNodeId::new(0),
                HeightNodeId::new(1),
            )],
            cliff_relations: vec![CliffNodeRelation {
                logical_edge,
                surface_vertex: SurfaceVertexId::new(0),
                node_a: HeightNodeId::new(0),
                node_b: HeightNodeId::new(0), // same node: collapsed taper
                lower_side: CliffLowerSide::A,
            }],
            ..Default::default()
        };
        let guide = dummy_guide(2);
        let config = HeightSolverConfig::default();
        let constraints = compile_hard_constraints(&graph, &guide, &config).unwrap();
        assert!(
            constraints.edges.is_empty(),
            "Collapsed taper (same node) must be silently skipped"
        );
    }

    /// Multi-step chain: lower bounds propagate forward (chain 0->1->2, ocean at 0).
    /// With min_drop=0.10: lower_bounds[2] >= 0.20.
    #[test]
    fn multi_step_chain_lower_bound_propagates_forward() {
        let edge_ab = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        let edge_bc = EdgeCoord::new(HexCoord::new(1, 0), HexCoord::new(2, 0));
        let graph = HeightConstraintGraph {
            nodes: vec![dummy_node(0), dummy_node(1), dummy_node(2)],
            continuity_edges: vec![
                HeightContinuityEdge::new(HeightNodeId::new(0), HeightNodeId::new(1)),
                HeightContinuityEdge::new(HeightNodeId::new(1), HeightNodeId::new(2)),
            ],
            cliff_relations: vec![
                CliffNodeRelation {
                    logical_edge: edge_ab,
                    surface_vertex: SurfaceVertexId::new(0),
                    node_a: HeightNodeId::new(0),
                    node_b: HeightNodeId::new(1),
                    lower_side: CliffLowerSide::A, // 0 < 1
                },
                CliffNodeRelation {
                    logical_edge: edge_bc,
                    surface_vertex: SurfaceVertexId::new(1),
                    node_a: HeightNodeId::new(1),
                    node_b: HeightNodeId::new(2),
                    lower_side: CliffLowerSide::A, // 1 < 2
                },
            ],
            ..Default::default()
        };
        let guide = LegacyHeightGuide {
            samples: vec![
                HeightGuideSample {
                    target: 0.0,
                    hard_pin: Some(0.0), // ocean pin at node 0
                },
                HeightGuideSample {
                    target: 0.30,
                    hard_pin: None,
                },
                HeightGuideSample {
                    target: 0.50,
                    hard_pin: None,
                },
            ],
        };
        let config = HeightSolverConfig {
            cliff_min_drop: 0.10,
            ..Default::default()
        };
        let constraints = compile_hard_constraints(&graph, &guide, &config).unwrap();
        assert_eq!(
            constraints.edges.len(),
            2,
            "Two resolved cliff edges expected"
        );
        assert!(
            constraints.lower_bounds[2] >= 0.20 - 1e-4,
            "lower_bounds[2] must be >= 0.20 after propagation from ocean+two drops. Got: {}",
            constraints.lower_bounds[2]
        );
    }

    /// Ocean pin on cliff HIGH side → infeasible because upper_bound[lower_node] goes negative.
    /// node_a (0) is HIGH side (pinned ocean=0.0); node_b (1) is LOWER.
    /// upper_bound[1] = 0.0 - 0.10 = -0.10 < lower_bound[1] = 0.0 => infeasible.
    #[test]
    fn ocean_pin_on_cliff_high_side_returns_infeasible() {
        let logical_edge = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        let graph = HeightConstraintGraph {
            nodes: vec![dummy_node(0), dummy_node(1)],
            continuity_edges: vec![HeightContinuityEdge::new(
                HeightNodeId::new(0),
                HeightNodeId::new(1),
            )],
            cliff_relations: vec![CliffNodeRelation {
                logical_edge,
                surface_vertex: SurfaceVertexId::new(0),
                node_a: HeightNodeId::new(0),
                node_b: HeightNodeId::new(1),
                lower_side: CliffLowerSide::B, // node_b (1) lower => node_a (0) is HIGH
            }],
            ..Default::default()
        };
        let guide = LegacyHeightGuide {
            samples: vec![
                HeightGuideSample {
                    target: 0.0,
                    hard_pin: Some(0.0), // HIGH side pinned at 0.0
                },
                HeightGuideSample {
                    target: 0.30,
                    hard_pin: None,
                },
            ],
        };
        let config = HeightSolverConfig {
            cliff_min_drop: 0.10,
            ..Default::default()
        };
        let result = compile_hard_constraints(&graph, &guide, &config);
        assert!(
            matches!(
                result,
                Err(HeightHardConstraintError::InfeasibleHardConstraints { .. })
            ),
            "Ocean pin on HIGH side must return InfeasibleHardConstraints, got: {result:?}"
        );
    }
}
