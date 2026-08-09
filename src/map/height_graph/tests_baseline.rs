// src/map/height_graph/tests_baseline.rs
//! Baseline unit tests for HeightConstraintGraph without cliffs.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightGraphBaselineTestsPlugin;

impl Plugin for HeightGraphBaselineTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compiler::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::HexCoord;

    #[test]
    fn baseline_no_cliffs_has_one_node_per_surface_vertex() {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());

        let seed = WorldSeed::new(42);
        let face_top =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Organic)
                .unwrap();
        let surface = generate_surface_topology(&face_top).unwrap();
        let constraints = compile_height_constraints(&map, &surface).unwrap();

        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();

        assert_eq!(graph.nodes.len(), surface.vertices.len());
        assert_eq!(graph.face_nodes.len(), surface.faces.len());
        assert_eq!(graph.stats.split_surface_vertex_count, 0);
        assert_eq!(graph.stats.error_diagnostic_count, 0);
    }
}
