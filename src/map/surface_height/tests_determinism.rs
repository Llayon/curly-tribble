// src/map/surface_height/tests_determinism.rs
//! Direct unit tests for bit-exact insertion order and iteration determinism.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::surface_height::guide::derive_legacy_height_guide;
    use crate::map::surface_height::hard_constraints::compile_hard_constraints;
    use crate::map::surface_height::solver::solve_surface_heights;
    use crate::map::surface_height::targets::compile_height_targets;
    use crate::map::surface_height::types::HeightSolverConfig;
    use crate::map::surface_topology::generate_surface_topology;
    use crate::map::surface_topology::types::SurfaceTopology;
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceHeightDeterminismTestsPlugin;

    impl Plugin for SurfaceHeightDeterminismTestsPlugin {
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
    fn normal_reverse_lcg_tile_insertion_order_determinism() {
        let coords: Vec<HexCoord> = (0..5)
            .flat_map(|q| (0..5).map(move |r| HexCoord::new(q, r)))
            .collect();

        // 1. Normal order insertion
        let mut map_data_normal = MapData::default();
        for &c in &coords {
            map_data_normal.tiles.insert(
                c,
                TileData {
                    elevation: 0.10 + 0.05 * (c.q + c.r) as f32,
                    ..Default::default()
                },
            );
        }

        // 2. Reverse order insertion
        let mut map_data_reverse = MapData::default();
        for &c in coords.iter().rev() {
            map_data_reverse.tiles.insert(
                c,
                TileData {
                    elevation: 0.10 + 0.05 * (c.q + c.r) as f32,
                    ..Default::default()
                },
            );
        }

        let config = HeightSolverConfig::default();

        let s_norm = build_test_surface(&map_data_normal);
        let hc_norm = compile_height_constraints(&map_data_normal, &s_norm).unwrap();
        let hg_norm = build_height_constraint_graph(&s_norm, &hc_norm).unwrap();
        let g_norm = derive_legacy_height_guide(&map_data_normal, &s_norm, &hg_norm).unwrap();
        let t_norm = compile_height_targets(&hg_norm, &g_norm, &config).unwrap();
        let hard_norm = compile_hard_constraints(&hg_norm, &g_norm, &config).unwrap();
        let layer_norm =
            solve_surface_heights(&hg_norm, &g_norm, &t_norm, &hard_norm, &config).unwrap();

        let s_rev = build_test_surface(&map_data_reverse);
        let hc_rev = compile_height_constraints(&map_data_reverse, &s_rev).unwrap();
        let hg_rev = build_height_constraint_graph(&s_rev, &hc_rev).unwrap();
        let g_rev = derive_legacy_height_guide(&map_data_reverse, &s_rev, &hg_rev).unwrap();
        let t_rev = compile_height_targets(&hg_rev, &g_rev, &config).unwrap();
        let hard_rev = compile_hard_constraints(&hg_rev, &g_rev, &config).unwrap();
        let layer_rev = solve_surface_heights(&hg_rev, &g_rev, &t_rev, &hard_rev, &config).unwrap();

        assert_eq!(layer_norm.heights.len(), layer_rev.heights.len());
        for (i, &val_norm) in layer_norm.heights.iter().enumerate() {
            let val_rev = layer_rev.heights[i];
            assert_eq!(val_norm.to_bits(), val_rev.to_bits());
        }
    }
}
