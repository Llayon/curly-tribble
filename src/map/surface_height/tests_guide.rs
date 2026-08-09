// src/map/surface_height/tests_guide.rs
//! Direct unit tests for LegacyHeightGuide derivation semantics and determinism.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{MapData, OceanState, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::surface_height::guide::derive_legacy_height_guide;
    use crate::map::surface_topology::generate_surface_topology;
    use crate::map::surface_topology::types::SurfaceTopology;
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceHeightGuideTestsPlugin;

    impl Plugin for SurfaceHeightGuideTestsPlugin {
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
    fn single_hex_elevation_guide() {
        let mut map_data = MapData::default();
        map_data.tiles.insert(
            HexCoord::new(0, 0),
            TileData {
                elevation: 0.45,
                ocean_state: OceanState::Land,
                ..Default::default()
            },
        );

        let surface = build_test_surface(&map_data);
        let constraints = compile_height_constraints(&map_data, &surface).unwrap();
        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();

        let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();
        assert_eq!(guide.samples.len(), graph.nodes.len());
        for sample in &guide.samples {
            assert!((sample.target - 0.45).abs() < 1e-5);
            assert_eq!(sample.hard_pin, None);
        }
    }

    #[test]
    fn pure_ocean_hard_pin_zero() {
        let mut map_data = MapData::default();
        map_data.tiles.insert(
            HexCoord::new(0, 0),
            TileData {
                elevation: 0.20,
                ocean_state: OceanState::Ocean,
                ..Default::default()
            },
        );

        let surface = build_test_surface(&map_data);
        let constraints = compile_height_constraints(&map_data, &surface).unwrap();
        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();

        let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();
        for sample in &guide.samples {
            assert_eq!(sample.target, 0.0);
            assert_eq!(sample.hard_pin, Some(0.0));
        }
    }

    #[test]
    fn coastline_mixed_node_soft_averaging() {
        let mut map_data = MapData::default();
        map_data.tiles.insert(
            HexCoord::new(0, 0),
            TileData {
                elevation: 0.60,
                ocean_state: OceanState::Land,
                ..Default::default()
            },
        );
        map_data.tiles.insert(
            HexCoord::new(1, 0),
            TileData {
                elevation: 0.10,
                ocean_state: OceanState::Ocean,
                ..Default::default()
            },
        );

        let surface = build_test_surface(&map_data);
        let constraints = compile_height_constraints(&map_data, &surface).unwrap();
        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();

        let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();
        let shared_node_sample = guide.samples.iter().find(|s| s.hard_pin.is_none());
        assert!(shared_node_sample.is_some());
        let sample = shared_node_sample.unwrap();
        assert!((sample.target - 0.30).abs() < 1e-5); // Average of 0.60 Land and 0.0 Ocean
    }
}
