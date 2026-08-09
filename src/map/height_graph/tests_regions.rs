// src/map/height_graph/tests_regions.rs
//! Region node binding tests for HeightConstraintGraph.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightGraphRegionsTestsPlugin;

impl Plugin for HeightGraphRegionsTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{LandscapeFeature, MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compiler::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::HexCoord;

    #[test]
    fn region_node_binding_is_complete_and_does_not_split_boundary_vertices() {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(
            c1,
            TileData {
                landscape_feature: LandscapeFeature::Mountain,
                ..Default::default()
            },
        );
        map.tiles.insert(
            c2,
            TileData {
                landscape_feature: LandscapeFeature::Plateau,
                ..Default::default()
            },
        );

        let seed = WorldSeed::new(42);
        let face_top =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Organic)
                .unwrap();
        let surface = generate_surface_topology(&face_top).unwrap();
        let constraints = compile_height_constraints(&map, &surface).unwrap();

        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();

        assert_eq!(graph.regions.len(), 2);
        assert_eq!(graph.nodes.len(), surface.vertices.len()); // Boundary vertex shared without cliff seam
    }
}
