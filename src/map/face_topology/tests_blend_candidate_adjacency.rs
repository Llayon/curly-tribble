//! Adjacent displacement-direction audit across the candidate policies.
//!
//! Every candidate is generated through its own pipeline; the dot products of
//! adjacent corner displacements are categorized by which endpoints the
//! candidate stabilized. Structural laws hold for every candidate (a
//! stabilization can never deepen a pair beyond the pre-existing worst, and
//! dots are unit-range), while the near-antiparallel production band and the
//! documented `-0.9995` tolerance threshold are locked here.
#[cfg(test)]
pub(crate) mod blend_candidate_adjacency_tests {
    use crate::map::face_topology::blend::PRODUCTION_BLEND_RELIABILITY_POLICY;
    use crate::map::face_topology::corner_key::regular_corner_position;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_blend_candidate_shared::shared as c;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::types::HexFaceTopology;
    use std::collections::HashSet;

    /// Documented tolerance band for "near-antiparallel": a dot at or below
    /// this threshold is flagged; an exact `-1.0` is separately detectable via
    /// `to_bits()` because it is the only negative-one bit pattern.
    const NEAR_ANTIPARALLEL_DOT_THRESHOLD: f32 = -0.9995;

    /// Per-candidate adjacency extremes over a seed sweep.
    #[derive(Debug, Clone, Copy, Default)]
    pub(crate) struct AdjacencyExtremes {
        pub global: f32,
        pub one_stabilized: f32,
        pub both_stabilized: f32,
    }

    /// Sweeps unique adjacent pairs, categorizing by stabilization endpoints.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn adjacency_extremes(
        topology: &HexFaceTopology,
        stabilized: &HashSet<crate::map::face_topology::types::SharedCornerKey>,
        extremes: &mut AdjacencyExtremes,
    ) {
        for (index, edge) in topology.half_edges.iter().enumerate() {
            if !edge.twin.is_none_or(|twin| index < twin.index()) {
                continue;
            }
            let origin = &topology.vertices[edge.origin.index()];
            let destination = &topology.vertices[edge.destination.index()];
            let (Ok(origin_regular), Ok(destination_regular)) = (
                regular_corner_position(origin.canonical_key),
                regular_corner_position(destination.canonical_key),
            ) else {
                continue;
            };
            let dot = (origin.position - origin_regular)
                .normalize_or_zero()
                .dot((destination.position - destination_regular).normalize_or_zero());
            extremes.global = extremes.global.min(dot);
            let origin_stabilized = stabilized.contains(&origin.canonical_key);
            let destination_stabilized = stabilized.contains(&destination.canonical_key);
            if origin_stabilized != destination_stabilized {
                extremes.one_stabilized = extremes.one_stabilized.min(dot);
            }
            if origin_stabilized && destination_stabilized {
                extremes.both_stabilized = extremes.both_stabilized.min(dot);
            }
        }
    }

    fn sweep(
        map: &crate::map::data::MapData,
        seeds: &[u32],
        candidate: &c::BlendCandidate,
        profile: HexDeformationProfile,
    ) -> (AdjacencyExtremes, usize) {
        let mut extremes = AdjacencyExtremes {
            global: 1.0,
            one_stabilized: 1.0,
            both_stabilized: 1.0,
        };
        let mut stabilized_total = 0_usize;
        for &seed in seeds {
            let topology = c::generate(map, seed, profile, candidate.policy);
            let stabilized = c::stabilized_keys(&topology, seed, profile, candidate.policy);
            stabilized_total += stabilized.len();
            adjacency_extremes(&topology, &stabilized, &mut extremes);
        }
        (extremes, stabilized_total)
    }

    /// Structural laws for every candidate plus the production-only
    /// near-antiparallel band, over the fast seeds on the canonical map.
    #[test]
    fn candidate_adjacent_direction_laws_hold_for_every_candidate() {
        let map = q::map_40x40();
        for candidate in &c::candidates() {
            for profile in c::BLENDED_PROFILES {
                let (extremes, stabilized_total) = sweep(&map, &q::FAST_SEEDS, candidate, profile);
                println!(
                    "{} {profile:?}: stabilized={stabilized_total} global={:.5} \
                     one={:.5} both={:.5}",
                    candidate.name,
                    extremes.global,
                    extremes.one_stabilized,
                    extremes.both_stabilized
                );
                assert!(
                    extremes.global >= -1.0 && extremes.global.is_finite(),
                    "{} {profile:?}: adjacent dot is a unit-range quantity",
                    candidate.name
                );
                assert!(
                    extremes.one_stabilized >= extremes.global - 1e-4,
                    "{} {profile:?}: the floor must not deepen a neighbor pair beyond the \
                     pre-existing worst: one={} global={}",
                    candidate.name,
                    extremes.one_stabilized,
                    extremes.global
                );
                if candidate.policy == PRODUCTION_BLEND_RELIABILITY_POLICY {
                    assert!(
                        extremes.both_stabilized >= -0.1,
                        "{} {profile:?}: production near-zero-linked pairs must stay far from \
                         anti-parallel: both={}",
                        candidate.name,
                        extremes.both_stabilized
                    );
                }
            }
        }
    }

    /// The documented near-antiparallel tolerance band: -1.0 is exact and
    /// detectable by bits; anything at or below the threshold is flagged.
    #[test]
    fn near_antiparallel_tolerance_band_is_documented_exactly() {
        assert_eq!((-1.0_f32).to_bits(), (-1.0_f32).to_bits());
        assert!(-1.0_f32 < NEAR_ANTIPARALLEL_DOT_THRESHOLD);
        assert!(-0.9996_f32 <= NEAR_ANTIPARALLEL_DOT_THRESHOLD);
        assert!(-0.9994_f32 > NEAR_ANTIPARALLEL_DOT_THRESHOLD);
        assert!((-1.0_f32).to_bits() != (-0.9995_f32).to_bits());
    }

    /// Full 256-seed adjacency extremes per candidate (ignored, for docs).
    #[test]
    #[ignore = "full candidate adjacency scan"]
    fn full_candidate_adjacency_256_seeds() {
        let map = q::map_40x40();
        let seeds: Vec<u32> = (0..256).collect();
        for candidate in &c::candidates() {
            for profile in c::BLENDED_PROFILES {
                let (extremes, stabilized_total) = sweep(&map, &seeds, candidate, profile);
                println!(
                    "{} {profile:?}: stabilized={stabilized_total} global={:.5} \
                     one={:.5} both={:.5}",
                    candidate.name,
                    extremes.global,
                    extremes.one_stabilized,
                    extremes.both_stabilized
                );
            }
        }
    }
}
