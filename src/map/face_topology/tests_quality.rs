//! Quality regression contracts for the tuned hex deformation profiles.
//!
//! This file hardens the fast 144-topology matrix, locks the pairwise
//! geometry/connectivity fingerprint contracts across profiles, and proves the
//! topology identity structure is insertion-order independent. Worst-case
//! visual fixtures selected by the full 4,608 scan live here too.
#[cfg(test)]
mod quality_tests {
    use crate::map::data::MapData;
    use crate::map::face_topology::blend::weighted_blend_diagnostics;
    use crate::map::face_topology::blend::{blend_to_displacement_q16, FixedVectorQ16};
    use crate::map::face_topology::fingerprint::topology_fingerprints;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::validate_complete_topology;
    use crate::map::HexCoord;
    use crate::map::WorldSeed;

    fn vector(x: i64, y: i64) -> FixedVectorQ16 {
        FixedVectorQ16 { x, y }
    }

    #[test]
    fn fast_144_topology_matrix_is_fully_hardened() {
        let mut failures = Vec::new();
        for (shape, map) in q::all_shapes() {
            for seed in q::FAST_SEEDS {
                for profile in q::all_profiles() {
                    failures.extend(q::case_failures(&map, shape, seed, profile));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// Profile default is the only distinct band configuration source; the
    /// matrix exercised every (shape, seed, profile) combination.
    #[test]
    fn fast_matrix_covers_exactly_144_topologies() {
        let mut cases = 0;
        let mut fingerprints = Vec::new();
        for (_, map) in q::all_shapes() {
            for seed in q::FAST_SEEDS {
                for profile in q::all_profiles() {
                    let topology = q::generate(&map, seed, profile);
                    validate_complete_topology(&topology, &map).unwrap_or_else(|error| {
                        panic!("seed {seed} profile {profile:?}: {error:?}")
                    });
                    fingerprints.push(topology_fingerprints(&map, WorldSeed::new(seed), &topology));
                    cases += 1;
                }
            }
        }
        assert_eq!(cases, 144);
        assert_eq!(fingerprints.len(), 144);
    }

    /// Connectivity must be identical across profiles; geometry must be
    /// pairwise distinct. The profile enum stored in `TopologyStats` is not
    /// proof that geometry differs, so the bit fingerprints are compared.
    #[test]
    fn profile_fingerprint_contracts_hold_on_canonical_map() {
        let map = q::map_40x40();
        for seed in q::FAST_SEEDS {
            let subtle = q::generate(&map, seed, HexDeformationProfile::Subtle);
            let organic = q::generate(&map, seed, HexDeformationProfile::Organic);
            let pago = q::generate(&map, seed, HexDeformationProfile::PagoniaLike);
            let fp_subtle = topology_fingerprints(&map, WorldSeed::new(seed), &subtle);
            let fp_organic = topology_fingerprints(&map, WorldSeed::new(seed), &organic);
            let fp_pago = topology_fingerprints(&map, WorldSeed::new(seed), &pago);
            assert_eq!(fp_subtle.connectivity, fp_organic.connectivity);
            assert_eq!(fp_organic.connectivity, fp_pago.connectivity);
            assert_ne!(fp_subtle.geometry, fp_organic.geometry);
            assert_ne!(fp_organic.geometry, fp_pago.geometry);
            assert_ne!(fp_subtle.geometry, fp_pago.geometry);
        }
    }

    /// Exact topology identity is profile-independent: counts, ordered id
    /// sets, face boundary cycles, half-edge endpoints/twins/ownership and the
    /// hex-to-face mapping all match across the three profiles.
    #[test]
    fn topology_identity_is_exactly_equal_across_profiles() {
        let map = q::map_40x40();
        let seed = 42;
        let reference = q::generate(&map, seed, HexDeformationProfile::Organic);
        assert_eq!(reference.faces.len(), q::CANONICAL_FACES);
        assert_eq!(reference.vertices.len(), q::CANONICAL_VERTICES);
        assert_eq!(reference.half_edges.len(), q::CANONICAL_HALF_EDGES);
        assert_eq!(reference.stats.paired_edge_count, q::CANONICAL_PAIRED_EDGES);
        assert_eq!(reference.stats.border_edge_count, q::CANONICAL_BORDER_EDGES);
        assert_eq!(
            reference.stats.paired_edge_count + reference.stats.border_edge_count,
            q::CANONICAL_UNIQUE_LOGICAL_EDGES
        );
        for profile in q::all_profiles() {
            let candidate = q::generate(&map, seed, profile);
            assert_eq!(candidate.faces, reference.faces, "faces for {profile:?}");
            assert_eq!(
                candidate.half_edges, reference.half_edges,
                "half-edges {profile:?}"
            );
            assert_eq!(
                candidate.hex_to_face, reference.hex_to_face,
                "ownership {profile:?}"
            );
            assert_eq!(candidate.vertices.len(), reference.vertices.len());
        }
    }

    /// The same logical map inserted in normal, reverse, and deterministic
    /// shuffled order must produce byte-identical topology for every profile.
    #[test]
    fn hashmap_insertion_order_does_not_change_any_profile() {
        let normal = q::map_40x40();
        let reverse = build_reverse_map();
        let shuffled = build_shuffled_map(42);
        let mut permutations = Vec::new();
        permutations.push(("normal", &normal));
        permutations.push(("reverse", &reverse));
        permutations.push(("shuffled", &shuffled));
        for seed in [0_u32, 42, 255] {
            for profile in q::all_profiles() {
                let reference = q::generate(&normal, seed, profile);
                for (label, map) in permutations.iter().copied() {
                    let candidate = q::generate(map, seed, profile);
                    assert_eq!(
                        candidate.vertices, reference.vertices,
                        "{label} seed {seed}"
                    );
                    assert_eq!(candidate.faces, reference.faces, "{label} seed {seed}");
                    assert_eq!(
                        candidate.half_edges, reference.half_edges,
                        "{label} seed {seed}"
                    );
                    assert_eq!(candidate.stats, reference.stats, "{label} seed {seed}");
                    let fp = topology_fingerprints(map, WorldSeed::new(seed), &candidate);
                    let ref_fp = topology_fingerprints(&normal, WorldSeed::new(seed), &reference);
                    assert_eq!(fp, ref_fp, "{label} seed {seed} profile {profile:?}");
                }
            }
        }
    }

    fn build_reverse_map() -> MapData {
        let reference = q::map_40x40();
        let mut coords: Vec<HexCoord> = reference.tiles.keys().copied().collect();
        coords.sort_by_key(|coord| (coord.q, coord.r));
        let mut map = MapData {
            width: reference.width,
            height: reference.height,
            ..MapData::default()
        };
        for coord in coords.into_iter().rev() {
            map.tiles.insert(coord, q::tile());
        }
        map
    }

    /// Deterministic Fisher-Yates permutation of the canonical coords.
    #[allow(clippy::cast_possible_truncation)]
    fn build_shuffled_map(seed: u32) -> MapData {
        let reference = q::map_40x40();
        let mut coords: Vec<HexCoord> = reference.tiles.keys().copied().collect();
        coords.sort_by_key(|coord| (coord.q, coord.r));
        let mut state = u64::from(seed).wrapping_add(0x9e37_79b9_7f4a_7c15);
        for index in (1..coords.len()).rev() {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let swap = (state % (index as u64 + 1)) as usize;
            coords.swap(index, swap);
        }
        let mut map = MapData {
            width: reference.width,
            height: reference.height,
            ..MapData::default()
        };
        for coord in coords {
            map.tiles.insert(coord, q::tile());
        }
        map
    }

    /// Worst-case visual fixtures selected by the full 4,608 scan. Each locks
    /// the exact measured extreme for its case so any future geometry change
    /// that shifts these values must be reviewed against the documented
    /// acceptance bands (which sit a small margin beyond these extrema).
    #[test]
    fn pagonia_max_angle_worst_case_is_seed_58() {
        let map = q::map_40x40();
        let quality = q::measured_quality(&map, 58, HexDeformationProfile::PagoniaLike);
        assert!(q::core_failures(&map, "40x40", 58, HexDeformationProfile::PagoniaLike).is_empty());
        assert_angle_close(quality.maximum_interior_angle_degrees, 175.195, 0.05);
        assert!(quality.minimum_aspect_quality > 0.370);
    }

    #[test]
    fn pagonia_min_aspect_worst_case_is_seed_169() {
        let map = q::map_40x40();
        let quality = q::measured_quality(&map, 169, HexDeformationProfile::PagoniaLike);
        assert!(
            q::core_failures(&map, "40x40", 169, HexDeformationProfile::PagoniaLike).is_empty()
        );
        assert_angle_close(quality.minimum_aspect_quality, 0.37826, 0.002);
        assert!(quality.maximum_interior_angle_degrees < 176.0);
    }

    #[test]
    fn organic_max_angle_worst_case_is_seed_203() {
        let map = q::map_40x40();
        let quality = q::measured_quality(&map, 203, HexDeformationProfile::Organic);
        assert!(q::core_failures(&map, "40x40", 203, HexDeformationProfile::Organic).is_empty());
        assert_angle_close(quality.maximum_interior_angle_degrees, 161.111, 0.05);
        assert!(quality.minimum_aspect_quality > 0.490);
    }

    #[test]
    fn organic_min_aspect_worst_case_is_seed_74() {
        let map = q::map_40x40();
        let quality = q::measured_quality(&map, 74, HexDeformationProfile::Organic);
        assert!(q::core_failures(&map, "40x40", 74, HexDeformationProfile::Organic).is_empty());
        assert_angle_close(quality.minimum_aspect_quality, 0.49371, 0.002);
        assert!(quality.maximum_interior_angle_degrees < 162.0);
    }

    fn assert_angle_close(measured: f32, expected: f32, tolerance: f32) {
        let delta = (measured - expected).abs();
        assert!(
            delta <= tolerance,
            "measured {measured} differs from {expected} by {delta} > {tolerance}"
        );
    }

    /// Repeated blend calls return byte-identical fixed-point results and no
    /// valid profile component range overflows the integer arithmetic.
    #[test]
    fn synthetic_blend_ranges_never_overflow_and_repeat_exactly() {
        for profile in [
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ] {
            let config = profile.config();
            let wc = config.correlated_weight_q16;
            let wl = config.local_weight_q16;
            let magnitudes = [
                -i64::from(config.component_magnitude_max_q16),
                -i64::from(config.component_magnitude_min_q16),
                0,
                i64::from(config.component_magnitude_min_q16),
                i64::from(config.component_magnitude_max_q16),
            ];
            for &correlated_x in &magnitudes {
                for &correlated_y in &magnitudes {
                    for &local_x in &magnitudes {
                        for &local_y in &magnitudes {
                            let correlated = vector(correlated_x, correlated_y);
                            let local = vector(local_x, local_y);
                            let first = blend_to_displacement_q16(correlated, local, wc, wl);
                            let second = blend_to_displacement_q16(correlated, local, wc, wl);
                            assert_eq!(first, second);
                            let diagnostics = weighted_blend_diagnostics(correlated, local, wc, wl);
                            assert!(diagnostics.target_magnitude_q16 <= 1 << 20);
                        }
                    }
                }
            }
        }
    }
}
