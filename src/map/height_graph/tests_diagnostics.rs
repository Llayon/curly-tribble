// src/map/height_graph/tests_diagnostics.rs
//! Diagnostics tests for HeightConstraintGraph.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightGraphDiagnosticsTestsPlugin;

impl Plugin for HeightGraphDiagnosticsTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{
        CliffLowerSide, EdgeCoord, EdgeData, EdgeType, MapData, TileData, WorldSeed,
    };
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compiler::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::height_graph::builder_diagnostics::collect_height_graph_diagnostics;
    use crate::map::height_graph::diagnostics::{
        HeightDiagnosticSeverity, HeightGraphDiagnosticKind,
    };
    use crate::map::height_graph::types::{CliffNodeRelation, HeightNodeId};
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::surface_topology::types::SurfaceVertexId;
    use crate::map::HexCoord;

    #[test]
    fn unresolved_cliff_produces_warning_diagnostic() {
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
                cliff_lower_side: CliffLowerSide::Unresolved,
            },
        );

        let seed = WorldSeed::new(42);
        let face_top =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Organic)
                .unwrap();
        let surface = generate_surface_topology(&face_top).unwrap();
        let constraints = compile_height_constraints(&map, &surface).unwrap();

        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();

        let warnings = graph
            .diagnostics
            .iter()
            .filter(|d| d.severity == HeightDiagnosticSeverity::Warning)
            .count();
        assert!(warnings >= 1);
    }

    #[test]
    fn direct_diagnostics_algorithm_test_all_variants() {
        let e1 = EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        let e2 = EdgeCoord::new(HexCoord::new(1, 0), HexCoord::new(0, 1));
        let e3 = EdgeCoord::new(HexCoord::new(0, 1), HexCoord::new(0, 0));

        let v0 = SurfaceVertexId::new(0);
        let v1 = SurfaceVertexId::new(1);
        let n0 = HeightNodeId::new(0);
        let n1 = HeightNodeId::new(1);
        let n2 = HeightNodeId::new(2);

        // 1. UnsplittableCliff: all samples collapsed
        let rel_unsplit = vec![
            CliffNodeRelation {
                logical_edge: e1,
                surface_vertex: v0,
                node_a: n0,
                node_b: n0,
                lower_side: CliffLowerSide::A,
            },
            CliffNodeRelation {
                logical_edge: e1,
                surface_vertex: v1,
                node_a: n0,
                node_b: n0,
                lower_side: CliffLowerSide::A,
            },
        ];
        let diag1 = collect_height_graph_diagnostics(&rel_unsplit);
        assert_eq!(diag1.len(), 1);
        assert_eq!(diag1[0].severity, HeightDiagnosticSeverity::Error);
        assert_eq!(
            diag1[0].kind,
            HeightGraphDiagnosticKind::UnsplittableCliff { edge: e1 }
        );

        // 2. CollapsedCliffSample: partial sample collapsed
        let rel_collapsed = vec![
            CliffNodeRelation {
                logical_edge: e1,
                surface_vertex: v0,
                node_a: n0,
                node_b: n0,
                lower_side: CliffLowerSide::A,
            },
            CliffNodeRelation {
                logical_edge: e1,
                surface_vertex: v1,
                node_a: n0,
                node_b: n1,
                lower_side: CliffLowerSide::A,
            },
        ];
        let diag2 = collect_height_graph_diagnostics(&rel_collapsed);
        assert_eq!(diag2.len(), 1);
        assert_eq!(diag2[0].severity, HeightDiagnosticSeverity::Info);
        assert_eq!(
            diag2[0].kind,
            HeightGraphDiagnosticKind::CollapsedCliffSample {
                edge: e1,
                vertex: v0,
            }
        );

        // 3. OpposedCliffOrdering: 2-node cycle (A < B and B < A)
        let rel_opposed = vec![
            CliffNodeRelation {
                logical_edge: e1,
                surface_vertex: v0,
                node_a: n0,
                node_b: n1,
                lower_side: CliffLowerSide::A, // n0 < n1
            },
            CliffNodeRelation {
                logical_edge: e2,
                surface_vertex: v1,
                node_a: n0,
                node_b: n1,
                lower_side: CliffLowerSide::B, // n1 < n0
            },
        ];
        let diag3 = collect_height_graph_diagnostics(&rel_opposed);
        assert_eq!(diag3.len(), 1);
        assert_eq!(diag3[0].severity, HeightDiagnosticSeverity::Error);
        assert_eq!(
            diag3[0].kind,
            HeightGraphDiagnosticKind::OpposedCliffOrdering { a: n0, b: n1 }
        );

        // 4. DirectedCliffCycle: SCC cycle (>= 3 nodes)
        let rel_cycle = vec![
            CliffNodeRelation {
                logical_edge: e1,
                surface_vertex: v0,
                node_a: n0,
                node_b: n1,
                lower_side: CliffLowerSide::A, // n0 < n1
            },
            CliffNodeRelation {
                logical_edge: e2,
                surface_vertex: v1,
                node_a: n1,
                node_b: n2,
                lower_side: CliffLowerSide::A, // n1 < n2
            },
            CliffNodeRelation {
                logical_edge: e3,
                surface_vertex: v0,
                node_a: n2,
                node_b: n0,
                lower_side: CliffLowerSide::A, // n2 < n0
            },
        ];
        let diag4 = collect_height_graph_diagnostics(&rel_cycle);
        assert_eq!(diag4.len(), 1);
        assert_eq!(diag4[0].severity, HeightDiagnosticSeverity::Error);
        assert_eq!(
            diag4[0].kind,
            HeightGraphDiagnosticKind::DirectedCliffCycle {
                component_nodes: vec![n0, n1, n2]
            }
        );
    }
}
