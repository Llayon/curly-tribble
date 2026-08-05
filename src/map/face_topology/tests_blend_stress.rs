//! Authoritative full-scan measurements of the candidate reliability floors
//! (ignored, run for the honest documentation table). Each candidate is built
//! through its own full pipeline over all 256 seeds on the canonical map;
//! every generation must succeed, stay backoff-free, and keep the production
//! hard caps (all enforced inside the generator).
#[cfg(test)]
mod blend_stress_tests {
    use crate::map::face_topology::acceptance::{ProfileAcceptanceCriteria, ProfileAcceptanceReport};
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_blend_candidate_adjacency::blend_candidate_adjacency_tests::{
        self, AdjacencyExtremes,
    };
    use crate::map::face_topology::tests_blend_candidate_shared::shared as c;
    use crate::map::face_topology::tests_quality_shared::shared as q;

    /// One candidate's authoritative table row over 256 seeds per profile.
    #[allow(clippy::too_many_arguments)]
    fn print_candidate_row(
        name: &str,
        profile: HexDeformationProfile,
        stabilized_total: usize,
        max_per_seed: usize,
        min_stabilized_ratio: Option<(i64, u32)>,
        extremes: AdjacencyExtremes,
        report: ProfileAcceptanceReport,
        seeds: usize,
    ) {
        let min_ratio = min_stabilized_ratio.map_or("n/a".to_owned(), |(ratio, seed)| {
            format!("{ratio} (seed {seed})")
        });
        println!(
            "{name} {profile:?}: stabilized={stabilized_total}/{seeds} max_per_seed={max_per_seed} \
             min_stabilized_ratio={min_ratio} global={:.5} one={:.5} both={:.5} avg={:.5} \
             min_edge={:.5} max_angle={:.2} min_aspect={:.4}",
            extremes.global,
            extremes.one_stabilized,
            extremes.both_stabilized,
            report.average_displacement_ratio,
            report.minimum_edge_length_ratio,
            report.maximum_interior_angle_degrees,
            report.minimum_aspect_quality,
        );
    }

    /// Full 256-seed candidate geometry scan: the authoritative reliability
    /// floor table (ignored; run explicitly, not in the default tier).
    #[test]
    #[ignore = "full blend reliability candidate geometry scan"]
    fn full_blend_reliability_candidate_geometry_256_seeds() {
        let map = q::map_40x40();
        let samples = 256_usize * 3_360;
        for candidate in &c::candidates() {
            for profile in c::BLENDED_PROFILES {
                let mut stabilized_total = 0_usize;
                let mut max_per_seed = 0_usize;
                let mut min_stabilized_ratio: Option<(i64, u32)> = None;
                let mut extremes = AdjacencyExtremes {
                    global: 1.0,
                    one_stabilized: 1.0,
                    both_stabilized: 1.0,
                };
                for seed in 0..256_u32 {
                    let topology = c::generate(&map, seed, profile, candidate.policy);
                    assert_eq!(
                        topology.stats.reduced_vertices, 0,
                        "{} {profile:?} seed={seed}: must stay backoff-free",
                        candidate.name
                    );
                    let config = profile.config();
                    let mut stabilized = std::collections::HashSet::new();
                    for vertex in &topology.vertices {
                        let diagnostics =
                            crate::map::face_topology::blend_diagnostics::weighted_blend_diagnostics_with_policy(
                                crate::map::face_topology::profiles::interpolated_correlated_field(
                                    seed,
                                    vertex.canonical_key,
                                    profile,
                                ),
                                crate::map::face_topology::profiles::local_component_q16(
                                    seed,
                                    vertex.canonical_key,
                                    profile,
                                ),
                                config.correlated_weight_q16,
                                config.local_weight_q16,
                                candidate.policy,
                            );
                        if !diagnostics.stabilization_applied {
                            continue;
                        }
                        stabilized.insert(vertex.canonical_key);
                        let ratio = if diagnostics.target_magnitude_q16 == 0 {
                            0
                        } else {
                            diagnostics.stabilized_length_q16 * 65_536
                                / diagnostics.target_magnitude_q16
                        };
                        if min_stabilized_ratio.is_none_or(|(best, _)| ratio < best) {
                            min_stabilized_ratio = Some((ratio, seed));
                        }
                    }
                    stabilized_total += stabilized.len();
                    max_per_seed = max_per_seed.max(stabilized.len());
                    blend_candidate_adjacency_tests::adjacency_extremes(
                        &topology,
                        &stabilized,
                        &mut extremes,
                    );
                }
                let report = ProfileAcceptanceReport::from_topology(&c::generate(
                    &map,
                    42,
                    profile,
                    candidate.policy,
                ));
                let failures = report.violations(ProfileAcceptanceCriteria::for_profile(profile));
                assert!(
                    failures.is_empty(),
                    "{} {profile:?} seed=42: {failures:?}",
                    candidate.name
                );
                print_candidate_row(
                    candidate.name,
                    profile,
                    stabilized_total,
                    max_per_seed,
                    min_stabilized_ratio,
                    extremes,
                    report,
                    samples,
                );
            }
        }
    }
}
