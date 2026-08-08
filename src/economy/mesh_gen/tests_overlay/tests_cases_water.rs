//! Water overlay case integration tests.

use bevy::prelude::*;

pub struct OverlayCasesWaterTestsPlugin;

impl Plugin for OverlayCasesWaterTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::economy::mesh_gen::generator::create_global_map_meshes;
    use crate::game_state::{EditorPhase, FactionManager};
    use crate::map::data::{MapData, OceanState, RoofState, TileData};
    use crate::map::face_topology::corner_key::regular_corner_position;
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::terrain_gen::TerrainConfig;
    use crate::map::topology::derive_terrain_topology;
    use crate::map::{HexCoord, LandscapeFeature, WorldSeed};
    use bevy::prelude::*;

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
    fn water_overlay_exact_boundary_and_upward_normals() {
        let mut map = MapData::default();
        let coord = HexCoord::new(0, 0);
        let mut tile = TileData::default();
        tile.ocean_state = OceanState::Land;
        tile.landscape_feature = LandscapeFeature::Lake;
        map.tiles.insert(coord, tile);

        let (face_topo, terrain_topo) = build_test_topology(&map, HexDeformationProfile::Organic);
        let (_terrain, water_opt, _roof) = create_global_map_meshes(
            &map,
            &terrain_topo,
            &face_topo,
            EditorPhase::Shape,
            &FactionManager::default(),
            &TerrainConfig::default(),
        );

        let water_mesh = water_opt.expect("water mesh should be created for Land + Lake");
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            water_mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("missing water positions");
        };

        assert_eq!(positions.len(), 7, "water cell must have 7 vertices");

        let face_id = face_topo.hex_to_face[&coord];
        let face = &face_topo.faces[face_id.index()];

        let mut corner_sum = Vec2::ZERO;
        for i in 0..6 {
            let v = face_topo.vertices[face.vertices[i].index()].position;
            corner_sum += v;

            let pos = positions[i + 1];
            assert_eq!(
                pos[0].to_bits(),
                v.x.to_bits(),
                "water corner X must match source vertex X bit-identically"
            );
            assert_eq!(
                pos[2].to_bits(),
                v.y.to_bits(),
                "water corner Z must match source vertex Z bit-identically"
            );
        }
        let expected_center = corner_sum / 6.0;
        let center_pos = positions[0];
        assert_eq!(center_pos[0].to_bits(), expected_center.x.to_bits());
        assert_eq!(center_pos[2].to_bits(), expected_center.y.to_bits());

        let mut displaced = false;
        for i in 0..6 {
            let src_vert = &face_topo.vertices[face.vertices[i].index()];
            let reg_pos = regular_corner_position(src_vert.canonical_key).expect("valid reg pos");
            if src_vert.position.to_array().map(f32::to_bits)
                != reg_pos.to_array().map(f32::to_bits)
            {
                displaced = true;
                break;
            }
        }
        assert!(
            displaced,
            "Organic profile must displace at least one water corner from regular geometry"
        );

        water_mesh.attribute(Mesh::ATTRIBUTE_NORMAL).map_or_else(
            || panic!("missing water normals"),
            |normals| {
                if let bevy::mesh::VertexAttributeValues::Float32x3(nor) = normals {
                    for n in nor {
                        assert!(
                            n[1] > 0.999,
                            "water surface normals must point upward (+Y), got {:?}",
                            n
                        );
                    }
                }
            },
        );
    }

    #[test]
    fn extract_warped_face_corners_returns_typed_error_on_missing_tile() {
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), TileData::default());
        let face_topo = crate::map::face_topology::generate_hex_face_topology_with_profile(
            &map,
            WorldSeed::new(42),
            HexDeformationProfile::Subtle,
        )
        .expect("valid face topo");

        let missing_coord = HexCoord::new(99, 99);
        let err = crate::economy::mesh_gen::generator::extract_warped_face_corners(
            missing_coord,
            &face_topo,
        )
        .expect_err("should return error for missing tile");

        assert_eq!(
            err,
            crate::economy::mesh_gen::generator::OverlayGeometryError::MissingFaceForTile(
                missing_coord
            )
        );
    }

    #[test]
    fn overlay_eligibility_semantics() {
        let coords = [
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(0, 1),
            HexCoord::new(1, 1),
            HexCoord::new(2, 0),
        ];

        let mut map = MapData::default();
        for &c in &coords {
            map.tiles.insert(c, TileData::default());
        }

        map.tiles.get_mut(&coords[0]).unwrap().ocean_state = OceanState::Land;
        map.tiles.get_mut(&coords[0]).unwrap().landscape_feature = LandscapeFeature::River;

        map.tiles.get_mut(&coords[1]).unwrap().ocean_state = OceanState::Land;
        map.tiles.get_mut(&coords[1]).unwrap().landscape_feature = LandscapeFeature::Lake;

        map.tiles.get_mut(&coords[2]).unwrap().ocean_state = OceanState::Land;
        map.tiles.get_mut(&coords[2]).unwrap().landscape_feature = LandscapeFeature::None;

        map.tiles.get_mut(&coords[3]).unwrap().ocean_state = OceanState::Ocean;
        map.tiles.get_mut(&coords[3]).unwrap().landscape_feature = LandscapeFeature::Lake;

        map.tiles.get_mut(&coords[4]).unwrap().roof_state = RoofState::Roofed;

        let (face_topo, terrain_topo) = build_test_topology(&map, HexDeformationProfile::Subtle);
        let (_terrain, water_opt, roof_opt) = create_global_map_meshes(
            &map,
            &terrain_topo,
            &face_topo,
            EditorPhase::Shape,
            &FactionManager::default(),
            &TerrainConfig::default(),
        );

        let water = water_opt.expect("water mesh should exist");
        let roof = roof_opt.expect("roof mesh should exist");

        let Some(bevy::mesh::VertexAttributeValues::Float32x3(w_pos)) =
            water.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("no water pos");
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(r_pos)) =
            roof.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("no roof pos");
        };

        assert_eq!(
            w_pos.len(),
            14,
            "exactly 2 tiles (River+Land, Lake+Land) generate water"
        );
        assert_eq!(r_pos.len(), 7, "exactly 1 tile generates roof");
    }
}
