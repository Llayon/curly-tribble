//! Advanced case-specific integration tests for warped water and roof overlays.

use bevy::prelude::*;

pub struct OverlayCasesAdvancedTestsPlugin;

impl Plugin for OverlayCasesAdvancedTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::economy::mesh_gen::generator::create_global_map_meshes;
    use crate::game_state::{EditorPhase, FactionManager};
    use crate::map::data::{MapData, OceanState, RoofState, TileData};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::terrain_gen::TerrainConfig;
    use crate::map::topology::derive_terrain_topology;
    use crate::map::{HexCoord, LandscapeFeature, WorldSeed};
    use bevy::prelude::*;
    use std::collections::HashMap;

    fn build_test_topology(
        map: &MapData,
        profile: HexDeformationProfile,
    ) -> (
        crate::map::face_topology::types::HexFaceTopology,
        crate::map::topology::TerrainTopology,
    ) {
        let face_topo = generate_hex_face_topology_with_profile(map, WorldSeed::new(42), profile)
            .expect("valid face topology");
        let terrain_topo =
            derive_terrain_topology(map, &face_topo).expect("valid terrain topology");
        (face_topo, terrain_topo)
    }

    #[test]
    fn deterministic_insertion_order() {
        let base_coords: Vec<HexCoord> = (0..5)
            .flat_map(|q| (0..5).map(move |r| HexCoord::new(q, r)))
            .collect();

        let make_map = |coords: &[HexCoord]| {
            let mut map = MapData::default();
            for &c in coords {
                let mut tile = TileData::default();
                tile.ocean_state = OceanState::Land;
                tile.landscape_feature = LandscapeFeature::Lake;
                tile.roof_state = RoofState::Roofed;
                map.tiles.insert(c, tile);
            }
            map
        };

        let map_normal = make_map(&base_coords);
        let (face_t, terr_t) = build_test_topology(&map_normal, HexDeformationProfile::Organic);
        let (_, w1, r1) = create_global_map_meshes(
            &map_normal,
            &terr_t,
            &face_t,
            EditorPhase::Shape,
            &FactionManager::default(),
            &TerrainConfig::default(),
        );

        let mut rev_coords = base_coords.clone();
        rev_coords.reverse();
        let map_rev = make_map(&rev_coords);
        let (_, w2, r2) = create_global_map_meshes(
            &map_rev,
            &terr_t,
            &face_t,
            EditorPhase::Shape,
            &FactionManager::default(),
            &TerrainConfig::default(),
        );

        let mut shuf_coords = base_coords.clone();
        let mut state = 0x1234_5678_u32;
        for i in (1..shuf_coords.len()).rev() {
            state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let j = (state as usize) % (i + 1);
            shuf_coords.swap(i, j);
        }
        let map_shuf = make_map(&shuf_coords);
        let (_, w3, r3) = create_global_map_meshes(
            &map_shuf,
            &terr_t,
            &face_t,
            EditorPhase::Shape,
            &FactionManager::default(),
            &TerrainConfig::default(),
        );

        let extract_pos = |m: &Mesh| {
            if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
                m.attribute(Mesh::ATTRIBUTE_POSITION)
            {
                p.clone()
            } else {
                vec![]
            }
        };

        let w1_p = extract_pos(&w1.unwrap());
        let w2_p = extract_pos(&w2.unwrap());
        let w3_p = extract_pos(&w3.unwrap());

        assert_eq!(w1_p, w2_p, "water positions must be order-independent");
        assert_eq!(w1_p, w3_p, "water positions must be order-independent");

        let r1_p = extract_pos(&r1.unwrap());
        let r2_p = extract_pos(&r2.unwrap());
        let r3_p = extract_pos(&r3.unwrap());

        assert_eq!(r1_p, r2_p, "roof positions must be order-independent");
        assert_eq!(r1_p, r3_p, "roof positions must be order-independent");
    }

    #[test]
    fn shared_adjacent_overlay_boundaries_agree_bit_identically() {
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);

        let mut map = MapData::default();
        let mut t1 = TileData::default();
        t1.ocean_state = OceanState::Land;
        t1.landscape_feature = LandscapeFeature::Lake;
        map.tiles.insert(c1, t1);

        let mut t2 = TileData::default();
        t2.ocean_state = OceanState::Land;
        t2.landscape_feature = LandscapeFeature::Lake;
        map.tiles.insert(c2, t2);

        let (face_t, terr_t) = build_test_topology(&map, HexDeformationProfile::Organic);
        let (_, w_opt, _) = create_global_map_meshes(
            &map,
            &terr_t,
            &face_t,
            EditorPhase::Shape,
            &FactionManager::default(),
            &TerrainConfig::default(),
        );

        let water = w_opt.unwrap();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            water.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("no water pos");
        };

        let cell1_corners: HashMap<[u32; 2], [f32; 3]> = pos[1..7]
            .iter()
            .map(|&p| ([p[0].to_bits(), p[2].to_bits()], p))
            .collect();
        let cell2_corners: HashMap<[u32; 2], [f32; 3]> = pos[8..14]
            .iter()
            .map(|&p| ([p[0].to_bits(), p[2].to_bits()], p))
            .collect();

        let mut shared_match_count = 0;
        for (key, p1) in &cell1_corners {
            if let Some(p2) = cell2_corners.get(key) {
                assert_eq!(p1[0].to_bits(), p2[0].to_bits());
                assert_eq!(p1[2].to_bits(), p2[2].to_bits());
                shared_match_count += 1;
            }
        }

        assert_eq!(
            shared_match_count, 2,
            "adjacent hex faces must share exactly 2 boundary corners bit-identically"
        );
    }
}
