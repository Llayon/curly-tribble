// src/map/height_graph/tests_determinism.rs
//! Tile insertion order determinism tests for HeightConstraintGraph.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightGraphDeterminismTestsPlugin;

impl Plugin for HeightGraphDeterminismTestsPlugin {
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
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::HexCoord;

    #[test]
    fn normal_reverse_lcg_insertion_order_determinism() {
        let coords = vec![
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(0, 1),
            HexCoord::new(-1, 1),
            HexCoord::new(-1, 0),
            HexCoord::new(0, -1),
            HexCoord::new(1, -1),
        ];

        let mut map_normal = MapData::default();
        for &c in &coords {
            map_normal.tiles.insert(
                c,
                TileData {
                    landscape_feature: LandscapeFeature::Mountain,
                    ..Default::default()
                },
            );
        }
        let e = EdgeCoord::new(coords[0], coords[1]);
        map_normal.edges.insert(
            e,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::A,
            },
        );

        let seed = WorldSeed::new(42);
        let face_top = generate_hex_face_topology_with_profile(
            &map_normal,
            seed,
            HexDeformationProfile::Organic,
        )
        .unwrap();
        let surface = generate_surface_topology(&face_top).unwrap();
        let c_normal = compile_height_constraints(&map_normal, &surface).unwrap();
        let g_normal = build_height_constraint_graph(&surface, &c_normal).unwrap();

        // Reverse insertion order
        let mut map_reverse = MapData::default();
        for &c in coords.iter().rev() {
            map_reverse.tiles.insert(
                c,
                TileData {
                    landscape_feature: LandscapeFeature::Mountain,
                    ..Default::default()
                },
            );
        }
        map_reverse.edges.insert(
            e,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::A,
            },
        );

        let c_reverse = compile_height_constraints(&map_reverse, &surface).unwrap();
        let g_reverse = build_height_constraint_graph(&surface, &c_reverse).unwrap();

        assert_eq!(g_normal.nodes, g_reverse.nodes);
        assert_eq!(g_normal.face_nodes, g_reverse.face_nodes);
        assert_eq!(g_normal.continuity_edges, g_reverse.continuity_edges);
        assert_eq!(g_normal.components, g_reverse.components);
        assert_eq!(g_normal.regions, g_reverse.regions);
        assert_eq!(g_normal.cliff_relations, g_reverse.cliff_relations);
        assert_eq!(g_normal.diagnostics, g_reverse.diagnostics);
    }
}
