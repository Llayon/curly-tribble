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
    use crate::map::height_graph::diagnostics::HeightDiagnosticSeverity;
    use crate::map::surface_topology::generator::generate_surface_topology;
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
}
