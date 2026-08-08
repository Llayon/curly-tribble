// src/map/height_constraints/tests_regions.rs
//! Unit tests for region height constraint compilation and independence.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightConstraintRegionsTestsPlugin;

impl Plugin for HeightConstraintRegionsTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{LandscapeFeature, MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compiler::compile_height_constraints;
    use crate::map::height_constraints::types::RegionHeightIntent;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::HexCoord;

    #[test]
    fn region_intent_1_to_1_mapping_and_none_omission() {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        let c3 = HexCoord::new(0, 1);
        let c4 = HexCoord::new(1, 1);
        let c5 = HexCoord::new(2, 0);

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
        map.tiles.insert(
            c3,
            TileData {
                landscape_feature: LandscapeFeature::Lake,
                ..Default::default()
            },
        );
        map.tiles.insert(
            c4,
            TileData {
                landscape_feature: LandscapeFeature::River,
                ..Default::default()
            },
        );
        map.tiles.insert(
            c5,
            TileData {
                landscape_feature: LandscapeFeature::None,
                ..Default::default()
            },
        );

        let seed = WorldSeed::new(42);
        let face_topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("Face topology failed");
        let surface = generate_surface_topology(&face_topology).expect("Surface topology failed");

        let constraints = compile_height_constraints(&map, &surface)
            .expect("Height constraints compilation failed");

        assert_eq!(constraints.regions.len(), 4);
        assert_eq!(constraints.stats.region_count, 4);

        let r1 = constraints.regions.iter().find(|r| r.hex == c1).unwrap();
        assert_eq!(r1.intent, RegionHeightIntent::Mountain);
        assert_eq!(r1.faces, surface.hex_to_faces[&c1]);

        let r2 = constraints.regions.iter().find(|r| r.hex == c2).unwrap();
        assert_eq!(r2.intent, RegionHeightIntent::Plateau);
        assert_eq!(r2.faces, surface.hex_to_faces[&c2]);

        let r3 = constraints.regions.iter().find(|r| r.hex == c3).unwrap();
        assert_eq!(r3.intent, RegionHeightIntent::Lake);
        assert_eq!(r3.faces, surface.hex_to_faces[&c3]);

        let r4 = constraints.regions.iter().find(|r| r.hex == c4).unwrap();
        assert_eq!(r4.intent, RegionHeightIntent::River);
        assert_eq!(r4.faces, surface.hex_to_faces[&c4]);

        assert!(constraints.regions.iter().all(|r| r.hex != c5));
    }

    #[test]
    fn elevation_independence_proof() {
        let mut map1 = MapData::default();
        let mut map2 = MapData::default();

        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);

        map1.tiles.insert(
            c1,
            TileData {
                landscape_feature: LandscapeFeature::Mountain,
                elevation: 0.0,
                ..Default::default()
            },
        );
        map1.tiles.insert(
            c2,
            TileData {
                landscape_feature: LandscapeFeature::Plateau,
                elevation: 1.0,
                ..Default::default()
            },
        );

        map2.tiles.insert(
            c1,
            TileData {
                landscape_feature: LandscapeFeature::Mountain,
                elevation: 10.0,
                ..Default::default()
            },
        );
        map2.tiles.insert(
            c2,
            TileData {
                landscape_feature: LandscapeFeature::Plateau,
                elevation: 25.0,
                ..Default::default()
            },
        );

        let seed = WorldSeed::new(42);
        let face_topology =
            generate_hex_face_topology_with_profile(&map1, seed, HexDeformationProfile::Subtle)
                .expect("Face topology failed");
        let surface = generate_surface_topology(&face_topology).expect("Surface topology failed");

        let constraints1 = compile_height_constraints(&map1, &surface).unwrap();
        let constraints2 = compile_height_constraints(&map2, &surface).unwrap();

        assert_eq!(constraints1, constraints2);
    }
}
