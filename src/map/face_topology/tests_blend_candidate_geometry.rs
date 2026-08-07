//! Candidate generation contracts: the two-level production baseline (normal
//! entry point bit-identical to the explicit production policy, which in turn
//! matches the literal `9ad12ae` fingerprints) and the honest candidate
//! matrix, where every candidate is generated through its own full pipeline
//! and must be valid, backoff-free, and genuinely different from the raw law.
#[cfg(test)]
mod blend_candidate_geometry_tests {
    use crate::map::data::MapData;
    use crate::map::face_topology::blend::PRODUCTION_BLEND_RELIABILITY_POLICY;
    use crate::map::face_topology::blend_policy::DISABLED_BLEND_RELIABILITY_POLICY;
    use crate::map::face_topology::fingerprint::topology_fingerprints;
    use crate::map::face_topology::generator::{
        generate_hex_face_topology, generate_hex_face_topology_with_profile,
        generate_hex_face_topology_with_profile_and_policy,
    };
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_blend_candidate_shared::shared as c;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::validate_complete_topology;
    use crate::map::WorldSeed;

    /// Level 1 of the baseline contract: the public entry points must be
    /// bit-identical to the explicit production-policy pipeline over the full
    /// fast matrix (3 profiles x 8 seeds x 6 shapes).
    #[test]
    fn production_pipeline_is_bit_identical_to_explicit_production_policy() {
        for (shape, map) in q::all_shapes() {
            for &seed in &q::FAST_SEEDS {
                for profile in q::all_profiles() {
                    let via_entry = generate_hex_face_topology_with_profile(
                        &map,
                        WorldSeed::new(seed),
                        profile,
                    )
                    .unwrap_or_else(|e| panic!("{shape} seed={seed}: {e:?}"));
                    let via_policy = generate_hex_face_topology_with_profile_and_policy(
                        &map,
                        WorldSeed::new(seed),
                        profile,
                        PRODUCTION_BLEND_RELIABILITY_POLICY,
                    )
                    .unwrap_or_else(|e| panic!("{shape} seed={seed}: {e:?}"));
                    assert_eq!(
                        via_entry, via_policy,
                        "{shape} seed={seed} profile={profile:?}: production entry and explicit \
                         production policy must agree bit for bit"
                    );
                    if profile == HexDeformationProfile::Subtle {
                        let default = generate_hex_face_topology(&map, WorldSeed::new(seed))
                            .unwrap_or_else(|e| panic!("{shape} seed={seed}: {e:?}"));
                        assert_eq!(default, via_entry, "Subtle default entry point");
                    }
                }
            }
        }
    }

    /// Level 2 of the baseline contract: the explicit production policy must
    /// reproduce the literal geometry/connectivity fingerprints of the hardened
    /// radial stabilization baseline (commit `6454046`) on the canonical 40x40 map
    /// (seed 42 and the weakest fixture seed 194). Historical pre-fix baseline
    /// `9ad12ae` is documented in ADR 0007.
    #[test]
    fn production_pipeline_matches_radial_stabilization_baseline() {
        let map = q::map_40x40();
        let expected = [
            (
                HexDeformationProfile::Organic,
                42_u32,
                16785404514996163090_u64,
                0xced2_a662_5361_af97,
            ),
            (
                HexDeformationProfile::Organic,
                194,
                12135014724753035092_u64,
                0xe403_a880_7d05_777f,
            ),
            (
                HexDeformationProfile::PagoniaLike,
                42,
                3198784433102359063_u64,
                0xced2_a662_5361_af97,
            ),
            (
                HexDeformationProfile::PagoniaLike,
                194,
                5745594850622409527_u64,
                0xe403_a880_7d05_777f,
            ),
        ];
        for (profile, seed, geometry, connectivity) in expected {
            let topology = generate_hex_face_topology_with_profile_and_policy(
                &map,
                WorldSeed::new(seed),
                profile,
                PRODUCTION_BLEND_RELIABILITY_POLICY,
            )
            .unwrap_or_else(|e| panic!("seed={seed} {profile:?}: {e:?}"));
            let fingerprints = topology_fingerprints(&map, WorldSeed::new(seed), &topology);
            assert_eq!(
                (fingerprints.geometry, fingerprints.connectivity),
                (geometry, connectivity),
                "seed={seed} {profile:?} must match the literal 6454046 radial stabilization baseline"
            );
        }
    }

    /// Every candidate must produce valid, backoff-free geometry on the
    /// canonical map across all fast seeds: the candidate path is a real
    /// generator, never a re-classification.
    #[test]
    fn every_candidate_generates_valid_backoff_free_topology_on_canonical_map() {
        let map = q::map_40x40();
        for candidate in c::candidates() {
            for profile in c::BLENDED_PROFILES {
                for &seed in &q::FAST_SEEDS {
                    let topology = c::generate(&map, seed, profile, candidate.policy);
                    validate_complete_topology(&topology, &map).unwrap_or_else(|e| {
                        panic!("{} {profile:?} seed={seed}: {e:?}", candidate.name)
                    });
                    assert_eq!(
                        topology.stats.reduced_vertices, 0,
                        "{} {profile:?} seed={seed}: candidate must not need backoff",
                        candidate.name
                    );
                }
            }
        }
    }

    /// The candidate path also stays valid on the irregular shapes; three
    /// policy extremes cover the matrix (raw, production, strongest floor).
    #[test]
    fn every_candidate_generates_valid_geometry_on_irregular_shapes() {
        let map_owned: Vec<(&'static str, MapData)> = q::all_shapes()
            .into_iter()
            .filter(|(name, _)| *name != "40x40")
            .collect();
        for (shape, map) in &map_owned {
            for candidate in c::candidates()
                .iter()
                .filter(|c| c.name == "raw" || c.name == "1/64_len" || c.name == "1/16_proj")
            {
                for profile in c::BLENDED_PROFILES {
                    for &seed in &q::FAST_SEEDS {
                        let topology = c::generate(map, seed, profile, candidate.policy);
                        validate_complete_topology(&topology, map).unwrap_or_else(|e| {
                            panic!("{} {shape} {profile:?} seed={seed}: {e:?}", candidate.name)
                        });
                        assert_eq!(
                            topology.stats.reduced_vertices, 0,
                            "{} {shape} {profile:?} seed={seed}: must not need backoff",
                            candidate.name
                        );
                    }
                }
            }
        }
    }

    /// Non-raw candidates must build genuinely different geometry from the raw
    /// law (own pipeline, measured by fingerprints), so the scan never
    /// re-classifies one shared topology.
    #[test]
    fn non_raw_candidates_build_genuinely_different_geometry() {
        let map = q::map_40x40();
        for candidate in c::candidates().iter().filter(|c| c.name != "raw") {
            for profile in c::BLENDED_PROFILES {
                let differs = q::FAST_SEEDS.iter().any(|&seed| {
                    let candidate_topology = c::generate(&map, seed, profile, candidate.policy);
                    let raw_topology =
                        c::generate(&map, seed, profile, DISABLED_BLEND_RELIABILITY_POLICY);
                    topology_fingerprints(&map, WorldSeed::new(seed), &candidate_topology).geometry
                        != topology_fingerprints(&map, WorldSeed::new(seed), &raw_topology).geometry
                });
                assert!(
                    differs,
                    "{} {profile:?}: must change real geometry on at least one fast seed",
                    candidate.name
                );
            }
        }
    }
}
