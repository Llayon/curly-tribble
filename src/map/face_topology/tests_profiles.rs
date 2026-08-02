/// Experimental deformation profile tests and profile stress tiers.
#[cfg(test)]
mod profile_tests {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::generator::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::{
        field_coordinate_q16, interpolated_correlated_field, local_component_q16,
        macro_field_node_vector, profile_displacement, FixedVectorQ16, HexDeformationProfile,
    };
    use crate::map::face_topology::validate_complete_topology;
    use crate::map::{HexCoord, WorldSeed};

    fn map_40x40() -> MapData {
        let mut map = MapData::default();
        for r in 0..40 {
            let offset = r >> 1;
            for q in -offset..(40 - offset) {
                map.tiles.insert(HexCoord::new(q, r), TileData::default());
            }
        }
        map.width = 40;
        map.height = 40;
        map
    }

    fn isolated_hex() -> MapData {
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), TileData::default());
        map
    }

    fn two_neighbors() -> MapData {
        let mut map = isolated_hex();
        map.tiles.insert(HexCoord::new(1, 0), TileData::default());
        map
    }

    fn seven_hex_cluster() -> MapData {
        let mut map = isolated_hex();
        for neighbor in HexCoord::new(0, 0).neighbors() {
            map.tiles.insert(neighbor, TileData::default());
        }
        map
    }

    fn sparse_l_shape() -> MapData {
        let mut map = MapData::default();
        for coord in [
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(2, 0),
            HexCoord::new(0, 1),
            HexCoord::new(0, 2),
        ] {
            map.tiles.insert(coord, TileData::default());
        }
        map
    }

    fn diagonal_strip() -> MapData {
        let mut map = MapData::default();
        for coord in [
            HexCoord::new(0, 0),
            HexCoord::new(1, 1),
            HexCoord::new(2, 2),
            HexCoord::new(3, 3),
        ] {
            map.tiles.insert(coord, TileData::default());
        }
        map
    }

    fn all_shapes() -> Vec<(&'static str, MapData)> {
        vec![
            ("40x40", map_40x40()),
            ("isolated", isolated_hex()),
            ("two_neighbors", two_neighbors()),
            ("seven_hex", seven_hex_cluster()),
            ("l_shape", sparse_l_shape()),
            ("diagonal", diagonal_strip()),
        ]
    }

    fn all_profiles() -> [HexDeformationProfile; 3] {
        [
            HexDeformationProfile::Subtle,
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ]
    }

    #[test]
    fn subtle_is_default_and_remains_bit_compatible() {
        assert_eq!(
            HexDeformationProfile::default(),
            HexDeformationProfile::Subtle
        );
        let map = two_neighbors();
        let seed = WorldSeed::new(42);
        let default_topology = crate::map::face_topology::generate_hex_face_topology(&map, seed)
            .expect("subtle default");
        let explicit_topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("subtle explicit");
        assert_eq!(default_topology, explicit_topology);
    }

    #[test]
    fn profiles_are_deterministic_and_preserve_topology_identity() {
        let map = map_40x40();
        let subtle = generate_hex_face_topology_with_profile(
            &map,
            WorldSeed::new(42),
            HexDeformationProfile::Subtle,
        )
        .expect("subtle");
        for profile in all_profiles() {
            let first = generate_hex_face_topology_with_profile(&map, WorldSeed::new(42), profile)
                .expect("profile");
            let second = generate_hex_face_topology_with_profile(&map, WorldSeed::new(42), profile)
                .expect("profile repeat");
            assert_eq!(first, second);
            assert_eq!(first.faces, subtle.faces);
            assert_eq!(first.half_edges, subtle.half_edges);
            assert_eq!(first.hex_to_face, subtle.hex_to_face);
            assert_eq!(first.vertices.len(), 3_360);
            assert_eq!(first.faces.len(), 1_600);
            assert_eq!(first.half_edges.len(), 9_600);
            assert_eq!(first.stats.paired_edge_count, 4_641);
            assert_eq!(first.stats.border_edge_count, 318);
            assert!(first.stats.min_aspect_quality > 0.0);
            assert!(first.stats.max_displacement.is_finite());
            validate_complete_topology(&first, &map).expect("profile validation");
        }
        let organic = generate_hex_face_topology_with_profile(
            &map,
            WorldSeed::new(42),
            HexDeformationProfile::Organic,
        )
        .expect("organic");
        assert_ne!(subtle.vertices, organic.vertices);
    }

    #[test]
    fn field_is_spatially_related_nonconstant_and_handles_negative_coordinates() {
        let key_a = crate::map::face_topology::canonical_corner_key(HexCoord::new(0, 0), 0);
        let key_b = crate::map::face_topology::canonical_corner_key(HexCoord::new(1, 0), 0);
        let key_far = crate::map::face_topology::canonical_corner_key(HexCoord::new(20, 20), 0);
        let near_a = interpolated_correlated_field(42, key_a, HexDeformationProfile::Organic);
        let near_b = interpolated_correlated_field(42, key_b, HexDeformationProfile::Organic);
        let far = interpolated_correlated_field(42, key_far, HexDeformationProfile::Organic);
        let near_delta = (near_a.x - near_b.x).abs() + (near_a.y - near_b.y).abs();
        assert!(near_delta < 13_000);
        assert!(near_a.x * near_b.x + near_a.y * near_b.y > 0);
        assert_ne!(near_a, far);
        let negative =
            field_coordinate_q16(crate::map::face_topology::types::SharedCornerKey::new(
                HexCoord::new(-6, -1),
                HexCoord::new(-5, -1),
                HexCoord::new(-5, 0),
            ));
        assert!(negative.0 < 0 && negative.1 < 0);
        assert_ne!(
            macro_field_node_vector(42, HexDeformationProfile::Organic, 0, 0),
            macro_field_node_vector(42, HexDeformationProfile::Organic, 1, 0)
        );
    }

    #[test]
    fn local_and_combined_profile_vectors_are_deterministic() {
        let key = crate::map::face_topology::canonical_corner_key(HexCoord::new(0, 0), 0);
        for profile in [
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ] {
            let expected = match profile {
                HexDeformationProfile::Organic => (
                    (FixedVectorQ16 { x: -3032, y: 7322 }),
                    (FixedVectorQ16 { x: -2991, y: 5971 }),
                    (FixedVectorQ16 { x: 3593, y: 8675 }),
                    (0xbc2b_8000, 0x3dd8_2800),
                ),
                HexDeformationProfile::PagoniaLike => (
                    (FixedVectorQ16 {
                        x: -4508,
                        y: -10884,
                    }),
                    (FixedVectorQ16 {
                        x: -4299,
                        y: -10559,
                    }),
                    (FixedVectorQ16 { x: -15358, y: 0 }),
                    (0xbddc_b800, 0xbdf7_7800),
                ),
                HexDeformationProfile::Subtle => unreachable!(),
            };
            assert_eq!(macro_field_node_vector(42, profile, 0, 0), expected.0);
            assert_eq!(interpolated_correlated_field(42, key, profile), expected.1);
            assert_eq!(local_component_q16(42, key, profile), expected.2);
            let displacement = profile_displacement(42, key, 1.0, profile);
            assert_eq!(
                (displacement.x.to_bits(), displacement.y.to_bits()),
                expected.3
            );
            assert_eq!(
                local_component_q16(42, key, profile),
                local_component_q16(42, key, profile)
            );
            assert_eq!(
                profile_displacement(42, key, 1.0, profile),
                profile_displacement(42, key, 1.0, profile)
            );
        }
    }

    fn validate_profiles_for_seeds(seeds: &[u32]) -> usize {
        let mut count = 0;
        for seed in seeds {
            for (_, map) in all_shapes() {
                for profile in all_profiles() {
                    let topology = generate_hex_face_topology_with_profile(
                        &map,
                        WorldSeed::new(*seed),
                        profile,
                    )
                    .unwrap_or_else(|error| panic!("seed={seed} profile={profile:?}: {error:?}"));
                    validate_complete_topology(&topology, &map).unwrap_or_else(|error| {
                        panic!("seed={seed} profile={profile:?}: {error:?}")
                    });
                    count += 1;
                }
            }
        }
        count
    }

    #[test]
    fn fast_profile_stress_covers_all_profiles_and_shapes() {
        assert_eq!(
            validate_profiles_for_seeds(&[0, 1, 7, 42, 99, 128, 200, 255]),
            144
        );
    }

    #[test]
    #[ignore = "full profile stress suite"]
    fn full_hex_deformation_profiles_stress_256_seeds() {
        let seeds: Vec<u32> = (0..256).collect();
        assert_eq!(validate_profiles_for_seeds(&seeds), 4_608);
    }
}
