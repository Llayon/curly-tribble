// src/map/terrain_bake/tests_compat.rs
//! Compatibility adapter proof: derive_terrain_topology_from_bake vs legacy adapter.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{MapData, OceanState, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::height_constraints::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::surface_height::guide::derive_legacy_height_guide;
    use crate::map::surface_height::hard_constraints::compile_hard_constraints;
    use crate::map::surface_height::solver::solve_surface_heights;
    use crate::map::surface_height::targets::compile_height_targets;
    use crate::map::surface_height::types::HeightSolverConfig;
    use crate::map::surface_topology::generate_surface_topology;
    use crate::map::surface_topology::terrain_adapter::derive_terrain_topology_from_surface;
    use crate::map::terrain_bake::builder::build_surface_terrain_bake;
    use crate::map::terrain_bake::compat::derive_terrain_topology_from_bake;
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct TerrainBakeCompatTestsPlugin;

    impl Plugin for TerrainBakeCompatTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// For no-cliff maps: compat topology must match derive_terrain_topology_from_surface exactly.
    /// XZ positions bit-exact, triangle count equal, owner hexes equal.
    #[test]
    fn no_cliff_compat_matches_surface_adapter() {
        let config = HeightSolverConfig::default();

        // Use a single-hex map (guaranteed no cliffs)
        let mut map = MapData::default();
        map.tiles.insert(
            HexCoord::new(0, 0),
            TileData {
                ocean_state: OceanState::Land,
                elevation: 0.5,
                ..Default::default()
            },
        );
        map.tiles.insert(
            HexCoord::new(1, 0),
            TileData {
                ocean_state: OceanState::Land,
                elevation: 0.3,
                ..Default::default()
            },
        );

        for &seed_val in &q::FAST_SEEDS {
            let seed = WorldSeed::new(seed_val);
            let face_top =
                generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                    .unwrap();
            let surface = generate_surface_topology(&face_top).unwrap();
            let constraints = compile_height_constraints(&map, &surface).unwrap();
            let graph = build_height_constraint_graph(&surface, &constraints).unwrap();

            // Only run if no cliff splits occurred
            if graph.stats.split_surface_vertex_count > 0 {
                continue;
            }

            let guide = derive_legacy_height_guide(&map, &surface, &graph).unwrap();
            let targets = compile_height_targets(&graph, &guide, &config).unwrap();
            let hard = compile_hard_constraints(&graph, &guide, &config).unwrap();
            let layer = solve_surface_heights(&graph, &guide, &targets, &hard, &config).unwrap();

            let bake = build_surface_terrain_bake(&surface, &graph, &layer).unwrap();
            let compat = derive_terrain_topology_from_bake(&bake).unwrap();
            let legacy = derive_terrain_topology_from_surface(&surface).unwrap();

            assert_eq!(
                compat.vertices_xz.len(),
                legacy.vertices_xz.len(),
                "seed={seed_val}: compat vertex count mismatch"
            );
            assert_eq!(
                compat.triangles.len(),
                legacy.triangles.len(),
                "seed={seed_val}: compat triangle count mismatch"
            );

            // XZ bit-exact (same surface vertex positions)
            for (i, (c, l)) in compat
                .vertices_xz
                .iter()
                .zip(&legacy.vertices_xz)
                .enumerate()
            {
                assert_eq!(
                    c.x.to_bits(),
                    l.x.to_bits(),
                    "seed={seed_val} vertex {i}: XZ x mismatch"
                );
                assert_eq!(
                    c.y.to_bits(),
                    l.y.to_bits(),
                    "seed={seed_val} vertex {i}: XZ y mismatch"
                );
            }
        }
    }

    /// Cliff bake: compat topology vertices >= surface vertices (splits create duplicates).
    #[test]
    fn cliff_bake_compat_has_more_vertices_than_surface() {
        use crate::map::data::{CliffLowerSide, EdgeCoord, EdgeData, EdgeType};

        let mut map = MapData::default();
        let hex_a = HexCoord::new(0, 0);
        let hex_b = HexCoord::new(1, 0);
        map.tiles.insert(
            hex_a,
            TileData {
                ocean_state: OceanState::Land,
                elevation: 0.1,
                ..Default::default()
            },
        );
        map.tiles.insert(
            hex_b,
            TileData {
                ocean_state: OceanState::Land,
                elevation: 0.9,
                ..Default::default()
            },
        );

        // Insert cliff edge
        let edge = EdgeCoord::new(hex_a, hex_b);
        map.edges.insert(
            edge,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::A,
            },
        );

        let config = HeightSolverConfig::default();
        let seed = WorldSeed::new(42);
        let face_top =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .unwrap();
        let surface = generate_surface_topology(&face_top).unwrap();
        let constraints = compile_height_constraints(&map, &surface).unwrap();
        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();

        if graph.stats.split_surface_vertex_count == 0 {
            // No actual splits → skip test
            return;
        }

        let guide = derive_legacy_height_guide(&map, &surface, &graph).unwrap();
        let targets = compile_height_targets(&graph, &guide, &config).unwrap();
        let hard = compile_hard_constraints(&graph, &guide, &config).unwrap();
        let layer = solve_surface_heights(&graph, &guide, &targets, &hard, &config).unwrap();

        let bake = build_surface_terrain_bake(&surface, &graph, &layer).unwrap();
        let compat = derive_terrain_topology_from_bake(&bake).unwrap();

        assert!(
            compat.vertices_xz.len() >= surface.vertices.len(),
            "cliff bake: compat vertex count {} should be >= surface vertex count {}",
            compat.vertices_xz.len(),
            surface.vertices.len()
        );
        assert!(
            bake.stats.split_surface_vertex_count > 0,
            "cliff bake must report split vertices"
        );
    }

    /// Empty bake → empty compat topology.
    #[test]
    fn empty_bake_compat_is_default() {
        use crate::map::terrain_bake::types::SurfaceTerrainBake;

        let compat = derive_terrain_topology_from_bake(&SurfaceTerrainBake::default()).unwrap();
        assert!(compat.vertices_xz.is_empty());
        assert!(compat.triangles.is_empty());
    }
}
