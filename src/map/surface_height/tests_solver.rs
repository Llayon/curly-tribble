// src/map/surface_height/tests_solver.rs
//! Direct unit tests for surface height solver, config validation, and compatibility mode.

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
    use crate::map::surface_height::types::{HeightSolverConfig, HeightSolverConfigError};
    use crate::map::surface_topology::generate_surface_topology;
    use crate::map::surface_topology::types::SurfaceTopology;
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceHeightSolverTestsPlugin;

    impl Plugin for SurfaceHeightSolverTestsPlugin {
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
    fn invalid_config_rejected() {
        let mut config = HeightSolverConfig::default();
        config.guide_weight = 0.0;
        assert_eq!(
            config.validate_config(),
            Err(HeightSolverConfigError::GuideWeightNotPositive)
        );

        config.guide_weight = 4.0;
        config.relaxation = 1.5;
        assert_eq!(
            config.validate_config(),
            Err(HeightSolverConfigError::InvalidRelaxation)
        );
    }

    #[test]
    fn exact_no_cliff_compatibility_mode_proof() {
        let mut map_data = MapData::default();
        map_data.tiles.insert(
            HexCoord::new(0, 0),
            TileData {
                elevation: 0.35,
                ..Default::default()
            },
        );
        map_data.tiles.insert(
            HexCoord::new(1, 0),
            TileData {
                elevation: 0.65,
                ..Default::default()
            },
        );

        let surface = build_test_surface(&map_data);
        let constraints = compile_height_constraints(&map_data, &surface).unwrap();
        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
        let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();

        let mut config = HeightSolverConfig::default();
        config.guide_weight = 1.0;
        config.region_weight = 0.0;
        config.smoothness_weight = 0.0;
        config.cliff_min_drop = 0.0;

        let targets = compile_height_targets(&graph, &guide, &config).unwrap();
        let hard = compile_hard_constraints(&graph, &guide, &config).unwrap();
        let layer = solve_surface_heights(&graph, &guide, &targets, &hard, &config).unwrap();

        assert_eq!(layer.heights.len(), guide.samples.len());
        for (i, &solved_val) in layer.heights.iter().enumerate() {
            let guide_target = guide.samples[i].target;
            assert_eq!(solved_val.to_bits(), guide_target.to_bits());
        }
    }
}
