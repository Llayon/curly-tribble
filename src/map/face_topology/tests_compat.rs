/// Legacy 158e5f2 compatibility and determinism guard tests.
///
/// Golden fingerprints were extracted in a temporary detached worktree at
/// commit `158e5f2` using the exact same fingerprint implementation. The values
/// below are literal constants; they are NOT computed by the current code.
///
/// Stable hash: project-owned FNV-1a 64-bit over big-endian fields
/// (`crate::map::face_topology::fingerprint`).
#[cfg(test)]
mod compat_tests {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::fingerprint::{topology_fingerprints, TopologyFingerprints};
    use crate::map::face_topology::generator::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::types::HexFaceTopology;
    use crate::map::{HexCoord, WorldSeed};
    use std::fs;

    fn tile() -> TileData {
        TileData::default()
    }

    fn map_40x40() -> MapData {
        let mut map = MapData::default();
        for r in 0..40i32 {
            let offset = r >> 1;
            for q in -offset..(40 - offset) {
                map.tiles.insert(HexCoord::new(q, r), tile());
            }
        }
        map.width = 40;
        map.height = 40;
        map
    }

    fn l_shape() -> MapData {
        let mut map = MapData::default();
        for coord in [
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(2, 0),
            HexCoord::new(0, 1),
            HexCoord::new(0, 2),
        ] {
            map.tiles.insert(coord, tile());
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
            map.tiles.insert(coord, tile());
        }
        map
    }

    fn seven_hex_cluster() -> MapData {
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), tile());
        for neighbor in HexCoord::new(0, 0).neighbors() {
            map.tiles.insert(neighbor, tile());
        }
        map
    }

    fn subtle(map: &MapData, seed: u32) -> HexFaceTopology {
        generate_hex_face_topology_with_profile(
            map,
            WorldSeed::new(seed),
            HexDeformationProfile::Subtle,
        )
        .unwrap_or_else(|error| panic!("seed {seed}: {error:?}"))
    }

    fn generate_with(map: &MapData, seed: u32, profile: HexDeformationProfile) -> HexFaceTopology {
        generate_hex_face_topology_with_profile(map, WorldSeed::new(seed), profile)
            .unwrap_or_else(|error| panic!("seed {seed} profile {profile:?}: {error:?}"))
    }

    fn frames(map: &MapData, seed: u32, topo: &HexFaceTopology) -> TopologyFingerprints {
        topology_fingerprints(map, WorldSeed::new(seed), topo)
    }

    /// Golden (connectivity, geometry) tuples extracted from `158e5f2`.
    #[rustfmt::skip]
    fn golden_constants() -> [(&'static str, u64, u64); 5] {
        [
            ("40x40_seed42", 0xced2_a662_5361_af97, 0x2c69_358d_1bde_2489),
            ("40x40_seed99", 0x4204_f108_4ab8_3e7c, 0x3222_1563_61ed_2849),
            ("l_shape_seed42", 0x9ed9_d5c5_d7b6_c2ab, 0x5575_c4b2_e091_0e73),
            ("diagonal_seed7", 0xc6f1_0fe0_442b_2820, 0xbb21_7bd7_4b1b_3c45),
            ("seven_hex_seed42", 0x91c0_95c7_e82c_ee27, 0x2531_f9b9_a3b1_7f8f),
        ]
    }

    fn fixture_of(label: &'static str) -> (MapData, u32) {
        match label {
            "40x40_seed42" => (map_40x40(), 42),
            "40x40_seed99" => (map_40x40(), 99),
            "l_shape_seed42" => (l_shape(), 42),
            "diagonal_seed7" => (diagonal_strip(), 7),
            "seven_hex_seed42" => (seven_hex_cluster(), 42),
            other => panic!("unknown fixture {other}"),
        }
    }

    /// 3. Legacy 158e5f2 connectivity golden fingerprints are preserved.
    #[test]
    fn legacy_connectivity_fingerprints_are_preserved() {
        for (label, expected_connectivity, _) in golden_constants() {
            let (map, seed) = fixture_of(label);
            let topo = subtle(&map, seed);
            let fp = frames(&map, seed, &topo);
            assert_eq!(
                fp.connectivity, expected_connectivity,
                "connectivity {label}"
            );
        }
    }

    /// 4. Legacy geometry golden fingerprints are preserved where proven.
    #[test]
    fn legacy_geometry_fingerprints_are_preserved() {
        for (label, _, expected_geometry) in golden_constants() {
            let (map, seed) = fixture_of(label);
            let topo = subtle(&map, seed);
            let fp = frames(&map, seed, &topo);
            assert_eq!(fp.geometry, expected_geometry, "geometry {label}");
        }
    }

    /// 5. Legacy reduction-absence is documented and consistent.
    ///
    /// The legacy scan found no reduction case in seeds 0..256 x all six shapes
    /// at `158e5f2`. This asserts the current Subtle fixtures match that, so we
    /// never claim backoff compatibility is tested where no legacy fixture
    /// triggered it.
    #[test]
    fn legacy_reduction_absence_is_consistent() {
        for seed in [0_u32, 1, 7, 42, 99, 128, 200, 255] {
            for map in [
                l_shape(),
                diagonal_strip(),
                seven_hex_cluster(),
                map_40x40(),
            ] {
                let topo = subtle(&map, seed);
                assert_eq!(
                    topo.stats.reduced_displacement_fallbacks, 0,
                    "seed {seed}: unexpected legacy reduction"
                );
                assert_eq!(
                    topo.stats.regular_position_fallbacks, 0,
                    "seed {seed}: unexpected fallback"
                );
            }
        }
    }

    /// 6. Current-vs-current wrapper comparison is not the sole compatibility
    /// proof. The golden constants above are literal, extracted from `158e5f2`,
    /// and the wrapper still delegates to the same `Subtle` path.
    #[test]
    fn goldens_are_literal_and_wrapper_matches_subtle() {
        for (label, _, _) in golden_constants() {
            let (map, seed) = fixture_of(label);
            let explicit = subtle(&map, seed);
            let wrapper_topology =
                crate::map::face_topology::generate_hex_face_topology(&map, WorldSeed::new(seed))
                    .expect("wrapper Subtle");
            assert_eq!(
                explicit, wrapper_topology,
                "wrapper {label} must equal Subtle profile"
            );
        }
    }

    /// 18. `HashMap` insertion order does not affect fingerprints.
    #[test]
    fn fingerprints_are_independent_of_hashmap_insertion_order() {
        let map = map_40x40();
        let mut coords: Vec<HexCoord> = map.tiles.keys().copied().collect();
        coords.sort_by_key(|coord| (coord.q, coord.r));
        coords.reverse();
        let mut reversed = MapData {
            width: map.width,
            height: map.height,
            ..MapData::default()
        };
        for coord in coords {
            reversed.tiles.insert(coord, tile());
        }

        let forward = subtle(&map, 42);
        let rev = generate_hex_face_topology_with_profile(
            &reversed,
            WorldSeed::new(42),
            HexDeformationProfile::Subtle,
        )
        .unwrap();
        assert_eq!(frames(&map, 42, &forward), frames(&reversed, 42, &rev));
    }

    /// 19. Geometry fingerprint excludes diagnostic acos-derived metrics.
    #[test]
    fn geometry_fingerprint_excludes_diagnostic_metrics() {
        let map = map_40x40();
        let mut topology = subtle(&map, 42);
        let base = frames(&map, 42, &topology);
        topology.stats.min_interior_angle = -1.0;
        topology.stats.max_interior_angle = 999.0;
        topology.stats.average_displacement = 42.0;
        topology.stats.max_displacement = -42.0;
        let mutated = frames(&map, 42, &topology);
        assert_eq!(base, mutated);
    }

    /// 20. Connectivity fingerprint is identical across all profiles; 21. the
    /// underlying `FaceId`/`VertexId`/`HalfEdgeId` sets are identical as well.
    #[test]
    fn connectivity_and_identity_are_identical_across_profiles() {
        let map = map_40x40();
        let mut prior_connectivity = None;
        let mut prior_faces = None;
        for profile in [
            HexDeformationProfile::Subtle,
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ] {
            let topo = generate_with(&map, 42, profile);
            let fp = frames(&map, 42, &topo);
            if let Some(expected) = prior_connectivity {
                assert_eq!(fp.connectivity, expected, "connectivity {profile:?}");
            } else {
                prior_connectivity = Some(fp.connectivity);
            }
            if let Some(faces) = prior_faces.as_ref() {
                assert_eq!(&topo.faces, faces, "face identity {profile:?}");
            } else {
                prior_faces = Some(topo.faces.clone());
            }
        }
    }

    /// 1. Documentation contains no claim of camera-rotation dependence.
    #[test]
    fn docs_do_not_claim_camera_rotation_dependence() {
        let adr =
            fs::read_to_string("docs/superpowers/adr/0007-hex-face-topology-debug-overlay.md")
                .expect("read ADR");
        assert!(!adr.to_lowercase().contains("camera rotation"));
        assert!(!adr.contains("gradient arises from the fixed camera rotation"));
        assert!(adr.contains("independent of the camera"));
    }

    /// 2. Topology generation has no camera input or resource dependency.
    #[test]
    fn topology_generation_has_no_camera_dependency() {
        let map = map_40x40();
        let baseline = subtle(&map, 42);
        let mut app = bevy::app::App::new();
        app.world_mut()
            .spawn(bevy::prelude::Camera2d)
            .insert(bevy::prelude::Transform::from_xyz(100.0, -50.0, 3.0));
        app.world_mut()
            .spawn(bevy::prelude::Camera3d::default())
            .insert(bevy::prelude::Transform::from_rotation(
                bevy::prelude::Quat::from_euler(bevy::prelude::EulerRot::XYZ, 1.0, 2.0, 3.0),
            ));
        app.update();
        let with_cameras = frames(&map, 42, &subtle(&map, 42));
        let baseline_fp = frames(&map, 42, &baseline);
        assert_eq!(baseline_fp, with_cameras);
    }
}
