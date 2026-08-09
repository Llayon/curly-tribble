// src/map/height_graph/tests_cliff_seams.rs
//! Cliff seam partitioning and lower-side invariance tests for HeightConstraintGraph.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightGraphCliffSeamsTestsPlugin;

impl Plugin for HeightGraphCliffSeamsTestsPlugin {
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
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::HexCoord;

    #[test]
    fn cliff_seam_splits_height_nodes_across_boundary() {
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
                cliff_lower_side: CliffLowerSide::A,
            },
        );

        let seed = WorldSeed::new(42);
        let face_top =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Organic)
                .unwrap();
        let surface = generate_surface_topology(&face_top).unwrap();
        let constraints = compile_height_constraints(&map, &surface).unwrap();

        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();

        assert!(graph.nodes.len() > surface.vertices.len());
        assert_eq!(graph.cliff_relations.len(), 3); // Fixed24 2 segments -> 3 unique boundary vertex relations
    }

    #[test]
    fn lower_side_edit_does_not_repartition_height_nodes() {
        let mut map_unresolved = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map_unresolved.tiles.insert(c1, TileData::default());
        map_unresolved.tiles.insert(c2, TileData::default());

        let edge = EdgeCoord::new(c1, c2);
        map_unresolved.edges.insert(
            edge,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::Unresolved,
            },
        );

        let seed = WorldSeed::new(42);
        let face_top = generate_hex_face_topology_with_profile(
            &map_unresolved,
            seed,
            HexDeformationProfile::Organic,
        )
        .unwrap();
        let surface = generate_surface_topology(&face_top).unwrap();

        let c_unresolved = compile_height_constraints(&map_unresolved, &surface).unwrap();
        let g_unresolved = build_height_constraint_graph(&surface, &c_unresolved).unwrap();

        let mut map_a = map_unresolved.clone();
        map_a.edges.get_mut(&edge).unwrap().cliff_lower_side = CliffLowerSide::A;
        let c_a = compile_height_constraints(&map_a, &surface).unwrap();
        let g_a = build_height_constraint_graph(&surface, &c_a).unwrap();

        let mut map_b = map_unresolved.clone();
        map_b.edges.get_mut(&edge).unwrap().cliff_lower_side = CliffLowerSide::B;
        let c_b = compile_height_constraints(&map_b, &surface).unwrap();
        let g_b = build_height_constraint_graph(&surface, &c_b).unwrap();

        // Nodes, face_nodes, continuity_edges, and components are 100% identical
        assert_eq!(g_unresolved.nodes, g_a.nodes);
        assert_eq!(g_a.nodes, g_b.nodes);

        assert_eq!(g_unresolved.face_nodes, g_a.face_nodes);
        assert_eq!(g_a.face_nodes, g_b.face_nodes);

        assert_eq!(g_unresolved.continuity_edges, g_a.continuity_edges);
        assert_eq!(g_a.continuity_edges, g_b.continuity_edges);

        assert_eq!(g_unresolved.components, g_a.components);
        assert_eq!(g_a.components, g_b.components);

        // Lower side preserves ordering identity
        assert_eq!(g_a.cliff_relations[0].lower_side, CliffLowerSide::A);
        assert_eq!(g_b.cliff_relations[0].lower_side, CliffLowerSide::B);
    }

    #[test]
    fn barrier_removal_reunifies_height_nodes() {
        let mut map_cliff = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map_cliff.tiles.insert(c1, TileData::default());
        map_cliff.tiles.insert(c2, TileData::default());

        let edge = EdgeCoord::new(c1, c2);
        map_cliff.edges.insert(
            edge,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::A,
            },
        );

        let seed = WorldSeed::new(42);
        let face_top = generate_hex_face_topology_with_profile(
            &map_cliff,
            seed,
            HexDeformationProfile::Organic,
        )
        .unwrap();
        let surface = generate_surface_topology(&face_top).unwrap();

        let c_cliff = compile_height_constraints(&map_cliff, &surface).unwrap();
        let g_cliff = build_height_constraint_graph(&surface, &c_cliff).unwrap();

        let map_flat = MapData {
            tiles: map_cliff.tiles.clone(),
            edges: Default::default(),
            ..Default::default()
        };
        let c_flat = compile_height_constraints(&map_flat, &surface).unwrap();
        let g_flat = build_height_constraint_graph(&surface, &c_flat).unwrap();

        assert!(g_cliff.nodes.len() > g_flat.nodes.len());
        assert_eq!(g_flat.nodes.len(), surface.vertices.len());
    }
}
