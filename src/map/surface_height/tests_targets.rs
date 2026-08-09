// src/map/surface_height/tests_targets.rs
//! Direct unit tests for preferred height target compilation and weight accumulation.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{LandscapeFeature, MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::surface_height::guide::derive_legacy_height_guide;
    use crate::map::surface_height::targets::compile_height_targets;
    use crate::map::surface_height::types::HeightSolverConfig;
    use crate::map::surface_topology::generate_surface_topology;
    use crate::map::surface_topology::types::SurfaceTopology;
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceHeightTargetsTestsPlugin;

    impl Plugin for SurfaceHeightTargetsTestsPlugin {
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
    fn mountain_semantic_uplift() {
        let mut map_data = MapData::default();
        map_data.tiles.insert(
            HexCoord::new(0, 0),
            TileData {
                elevation: 0.50,
                landscape_feature: LandscapeFeature::Mountain,
                ..Default::default()
            },
        );

        let surface = build_test_surface(&map_data);
        let constraints = compile_height_constraints(&map_data, &surface).unwrap();
        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
        let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();

        let config = HeightSolverConfig::default();
        let targets = compile_height_targets(&graph, &guide, &config).unwrap();

        assert_eq!(targets.samples.len(), graph.nodes.len());
        for sample in &targets.samples {
            assert!(sample.target > 0.50); // Uplifted by mountain_bias
            assert!(sample.weight > config.guide_weight); // Guide weight + region weight
        }
    }

    #[test]
    fn lake_semantic_depression() {
        let mut map_data = MapData::default();
        map_data.tiles.insert(
            HexCoord::new(0, 0),
            TileData {
                elevation: 0.50,
                landscape_feature: LandscapeFeature::Lake,
                ..Default::default()
            },
        );

        let surface = build_test_surface(&map_data);
        let constraints = compile_height_constraints(&map_data, &surface).unwrap();
        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
        let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();

        let config = HeightSolverConfig::default();
        let targets = compile_height_targets(&graph, &guide, &config).unwrap();

        for sample in &targets.samples {
            assert!(sample.target < 0.50); // Depressed by lake_bias
        }
    }
}
