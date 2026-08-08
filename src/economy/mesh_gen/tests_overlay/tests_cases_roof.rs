//! Roof overlay case integration tests.

use bevy::prelude::*;

pub struct OverlayCasesRoofTestsPlugin;

impl Plugin for OverlayCasesRoofTestsPlugin {
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
    fn roof_overlay_exact_boundary_and_constant_offset_y() {
        let mut map = MapData::default();
        let coord = HexCoord::new(0, 0);
        let mut tile = TileData::default();
        tile.roof_state = RoofState::Roofed;
        map.tiles.insert(coord, tile);

        let (face_topo, terrain_topo) = build_test_topology(&map, HexDeformationProfile::Organic);
        let (_terrain, _water, roof_opt) = create_global_map_meshes(
            &map,
            &terrain_topo,
            &face_topo,
            EditorPhase::Shape,
            &FactionManager::default(),
            &TerrainConfig::default(),
        )
        .expect("valid map meshes");

        let roof_mesh = roof_opt.expect("roof mesh should be created for Roofed state");
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            roof_mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("missing roof positions");
        };

        assert_eq!(positions.len(), 7, "roof cell must have 7 vertices");

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
                "roof corner X must match source vertex X bit-identically"
            );
            assert_eq!(
                pos[2].to_bits(),
                v.y.to_bits(),
                "roof corner Z must match source vertex Z bit-identically"
            );
        }
        let expected_center = corner_sum / 6.0;
        let center_pos = positions[0];
        assert_eq!(center_pos[0].to_bits(), expected_center.x.to_bits());
        assert_eq!(center_pos[2].to_bits(), expected_center.y.to_bits());

        for pos in positions {
            assert_eq!(pos[1], 2.5, "Flat phase roof Y must be 2.5");
        }

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
            "Organic profile must displace at least one roof corner from regular geometry"
        );

        roof_mesh.attribute(Mesh::ATTRIBUTE_NORMAL).map_or_else(
            || panic!("missing roof normals"),
            |normals| {
                if let bevy::mesh::VertexAttributeValues::Float32x3(nor) = normals {
                    for n in nor {
                        assert!(
                            n[1] > 0.999,
                            "roof surface normals must point upward (+Y), got {:?}",
                            n
                        );
                    }
                }
            },
        );
    }

    #[test]
    fn height_phase_compatibility() {
        let mut map = MapData::default();
        let c = HexCoord::new(0, 0);
        let mut tile = TileData::default();
        tile.elevation = 4.0;
        tile.ocean_state = OceanState::Land;
        tile.landscape_feature = LandscapeFeature::Lake;
        tile.roof_state = RoofState::Roofed;
        map.tiles.insert(c, tile);

        let (face_topo, terrain_topo) = build_test_topology(&map, HexDeformationProfile::Subtle);

        let (_, w_flat, r_flat) = create_global_map_meshes(
            &map,
            &terrain_topo,
            &face_topo,
            EditorPhase::Shape,
            &FactionManager::default(),
            &TerrainConfig::default(),
        )
        .expect("valid map meshes");
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            w_flat.unwrap().attribute(Mesh::ATTRIBUTE_POSITION)
        {
            assert_eq!(pos[0][1], 0.0);
        }
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            r_flat.unwrap().attribute(Mesh::ATTRIBUTE_POSITION)
        {
            assert_eq!(pos[0][1], 2.5);
        }

        let (_, w_3d, r_3d) = create_global_map_meshes(
            &map,
            &terrain_topo,
            &face_topo,
            EditorPhase::Height3D,
            &FactionManager::default(),
            &TerrainConfig::default(),
        )
        .expect("valid map meshes");
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            w_3d.unwrap().attribute(Mesh::ATTRIBUTE_POSITION)
        {
            assert_eq!(pos[0][1], map.get_hex_height(0, 0));
        }
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            r_3d.unwrap().attribute(Mesh::ATTRIBUTE_POSITION)
        {
            assert_eq!(pos[0][1], map.get_hex_height(0, 0) + 2.5);
        }
    }
}
