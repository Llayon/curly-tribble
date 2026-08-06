//! Authoritative full-scan measurements of candidate reliability floors
//! and validation contracts over 256 seeds on the canonical map.
#[cfg(test)]
mod blend_stress_tests {
    use crate::map::face_topology::acceptance::{
        ProfileAcceptanceCriteria, ProfileAcceptanceReport,
    };
    use crate::map::face_topology::blend::BlendReference;
    use crate::map::face_topology::blend_diagnostics::weighted_blend_diagnostics_with_policy;
    use crate::map::face_topology::fingerprint::topology_fingerprints;
    use crate::map::face_topology::profiles::{
        interpolated_correlated_field, local_component_q16, HexDeformationProfile,
    };
    use crate::map::face_topology::tests_blend_candidate_shared::shared as c;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::types::SharedCornerKey;

    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    #[test]
    #[ignore = "full blend reliability candidate validation scan"]
    fn full_blend_reliability_candidate_validation_256_seeds() {
        let map = q::map_40x40();
        let samples = 256_usize * 3_360;

        for candidate in &c::candidates() {
            for profile in c::BLENDED_PROFILES {
                let config = profile.config();
                let mut generated_count = 0_usize;
                let mut rejected_count = 0_usize;
                let mut rejections = Vec::new();

                let mut stabilized_total = 0_usize;
                let mut max_stabilized_per_seed = 0_usize;
                let mut ref_correlated = 0_usize;
                let mut ref_local = 0_usize;
                let mut ref_fixed_x = 0_usize;

                let mut min_raw_ratio: Option<(i64, u32, SharedCornerKey)> = None;
                let mut min_stab_len_ratio: Option<(i64, u32, SharedCornerKey)> = None;
                let mut min_stab_proj_ratio: Option<(i64, u32, SharedCornerKey)> = None;

                let mut geometry_fps = std::collections::HashSet::new();
                let mut conn_fps_match = true;

                let mut reduction_rounds_total = 0_usize;
                let mut reduced_vertices_total = 0_usize;
                let mut reduced_disp_fallbacks = 0_usize;
                let mut regular_pos_fallbacks = 0_usize;

                let mut avg_disp_min = (f32::INFINITY, 0_u32);
                let mut avg_disp_max = (f32::NEG_INFINITY, 0_u32);
                let mut max_disp_max = (0.0_f32, 0_u32);
                let mut min_edge_min = (f32::INFINITY, 0_u32);
                let mut max_edge_max = (0.0_f32, 0_u32);
                let mut min_angle_min = (f32::INFINITY, 0_u32);
                let mut max_angle_max = (0.0_f32, 0_u32);
                let mut min_aspect_min = (f32::INFINITY, 0_u32);

                let mut min_dot_neither = 1.0_f32;
                let mut min_dot_origin = 1.0_f32;
                let mut min_dot_dest = 1.0_f32;
                let mut min_dot_both = 1.0_f32;

                let null_key = SharedCornerKey::new(
                    crate::map::HexCoord::new(0, 0),
                    crate::map::HexCoord::new(0, 0),
                    crate::map::HexCoord::new(0, 0),
                );
                let mut largest_imp = (0.0_f32, 0_u32, null_key, null_key, 0.0_f32, 0.0_f32);
                let mut largest_reg = (0.0_f32, 0_u32, null_key, null_key, 0.0_f32, 0.0_f32);

                let mut improved_edges = 0_usize;
                let mut unchanged_edges = 0_usize;
                let mut regressed_edges = 0_usize;
                let mut newly_near_anti = 0_usize;
                let mut removed_near_anti = 0_usize;
                let mut newly_near_anti_stab = 0_usize;
                let mut newly_exact_m1 = 0_usize;
                let mut removed_exact_m1 = 0_usize;
                let mut newly_exact_m1_stab = 0_usize;

                let mut level_b_misses = 0_usize;

                for seed in 0..256_u32 {
                    let raw_topo = c::generate(
                        &map,
                        seed,
                        profile,
                        crate::map::face_topology::blend_policy::DISABLED_BLEND_RELIABILITY_POLICY,
                    );
                    let raw_fps =
                        topology_fingerprints(&map, crate::map::WorldSeed::new(seed), &raw_topo);
                    let raw_edges = c::extract_edge_dots(&raw_topo);

                    let candidate_res = c::try_generate(&map, seed, profile, candidate.policy);
                    let topo = match candidate_res {
                        Ok(t) => t,
                        Err(e) => {
                            rejected_count += 1;
                            rejections.push((seed, e));
                            continue;
                        }
                    };
                    generated_count += 1;

                    let cand_fps =
                        topology_fingerprints(&map, crate::map::WorldSeed::new(seed), &topo);
                    geometry_fps.insert(cand_fps.geometry);
                    if cand_fps.connectivity != raw_fps.connectivity {
                        conn_fps_match = false;
                    }

                    reduction_rounds_total += topo.stats.reduction_rounds;
                    reduced_vertices_total += topo.stats.reduced_vertices;
                    reduced_disp_fallbacks += topo.stats.reduced_displacement_fallbacks;
                    regular_pos_fallbacks += topo.stats.regular_position_fallbacks;

                    let report = ProfileAcceptanceReport::from_topology(&topo);
                    if !report
                        .violations(ProfileAcceptanceCriteria::for_profile(profile))
                        .is_empty()
                    {
                        level_b_misses += 1;
                    }

                    if report.average_displacement_ratio < avg_disp_min.0 {
                        avg_disp_min = (report.average_displacement_ratio, seed);
                    }
                    if report.average_displacement_ratio > avg_disp_max.0 {
                        avg_disp_max = (report.average_displacement_ratio, seed);
                    }
                    if topo.stats.max_displacement > max_disp_max.0 {
                        max_disp_max = (topo.stats.max_displacement, seed);
                    }
                    if report.minimum_edge_length_ratio < min_edge_min.0 {
                        min_edge_min = (report.minimum_edge_length_ratio, seed);
                    }
                    if topo.stats.max_edge_length > max_edge_max.0 {
                        max_edge_max = (topo.stats.max_edge_length, seed);
                    }
                    if topo.stats.min_interior_angle < min_angle_min.0 {
                        min_angle_min = (topo.stats.min_interior_angle, seed);
                    }
                    if report.maximum_interior_angle_degrees > max_angle_max.0 {
                        max_angle_max = (report.maximum_interior_angle_degrees, seed);
                    }
                    if report.minimum_aspect_quality < min_aspect_min.0 {
                        min_aspect_min = (report.minimum_aspect_quality, seed);
                    }

                    let mut stab_set = std::collections::HashSet::new();
                    for v in &topo.vertices {
                        let diag = weighted_blend_diagnostics_with_policy(
                            interpolated_correlated_field(seed, v.canonical_key, profile),
                            local_component_q16(seed, v.canonical_key, profile),
                            config.correlated_weight_q16,
                            config.local_weight_q16,
                            candidate.policy,
                        );
                        match diag.reference {
                            BlendReference::Correlated => ref_correlated += 1,
                            BlendReference::Local => ref_local += 1,
                            BlendReference::FixedPositiveX => ref_fixed_x += 1,
                        }
                        if diag.target_magnitude_q16 > 0 {
                            let r_raw =
                                diag.weighted_length_q16 * 65_536 / diag.target_magnitude_q16;
                            if min_raw_ratio.is_none_or(|(r, _, _)| r_raw < r) {
                                min_raw_ratio = Some((r_raw, seed, v.canonical_key));
                            }
                        }
                        if diag.stabilization_applied {
                            stab_set.insert(v.canonical_key);
                            if diag.target_magnitude_q16 > 0 {
                                let r_len =
                                    diag.stabilized_length_q16 * 65_536 / diag.target_magnitude_q16;
                                if min_stab_len_ratio.is_none_or(|(r, _, _)| r_len < r) {
                                    min_stab_len_ratio = Some((r_len, seed, v.canonical_key));
                                }
                                let r_proj = diag.minimum_projection_q16 * 65_536
                                    / diag.target_magnitude_q16;
                                if min_stab_proj_ratio.is_none_or(|(r, _, _)| r_proj < r) {
                                    min_stab_proj_ratio = Some((r_proj, seed, v.canonical_key));
                                }
                            }
                        }
                    }
                    stabilized_total += stab_set.len();
                    max_stabilized_per_seed = max_stabilized_per_seed.max(stab_set.len());

                    c::compare_adjacency(
                        &topo,
                        &stab_set,
                        &raw_edges,
                        seed,
                        &mut min_dot_neither,
                        &mut min_dot_origin,
                        &mut min_dot_dest,
                        &mut min_dot_both,
                        &mut largest_imp,
                        &mut largest_reg,
                        &mut improved_edges,
                        &mut unchanged_edges,
                        &mut regressed_edges,
                        &mut newly_near_anti,
                        &mut removed_near_anti,
                        &mut newly_near_anti_stab,
                        &mut newly_exact_m1,
                        &mut removed_exact_m1,
                        &mut newly_exact_m1_stab,
                    );
                }

                let pct = (stabilized_total as f64 / samples as f64) * 100.0;
                let min_one_stab_dot = min_dot_origin.min(min_dot_dest);

                println!(
                    "VALIDATION_SCAN candidate={} profile={:?} gen={generated_count} rej={rejected_count} \
                     stab={stabilized_total}/{samples} ({pct:.2}%) max_per_seed={max_stabilized_per_seed} \
                     refs(C/L/X)={ref_correlated}/{ref_local}/{ref_fixed_x} \
                     min_raw_ratio={:?} min_stab_len_ratio={:?} min_stab_proj_ratio={:?} \
                     geo_fp_diversity={} conn_fp_match={conn_fps_match} \
                     backoff(rounds/verts/disp_fb/reg_fb)={reduction_rounds_total}/{reduced_vertices_total}/{reduced_disp_fallbacks}/{regular_pos_fallbacks} \
                     extrema: avg_disp=[{:.5} (s{}), {:.5} (s{})] max_disp={:.5} (s{}) \
                     min_edge={:.5} (s{}) max_angle={:.2}° (s{}) min_aspect={:.4} (s{}) \
                     adj_min_dots: neither={:.5} one_stab={:.5} (orig={:.5}, dest={:.5}) both_stab={:.5} \
                     largest_imp={:.5} (s{} key1={:?} key2={:?} before={:.5} after={:.5}) \
                     largest_reg={:.5} (s{} key1={:?} key2={:?} before={:.5} after={:.5}) \
                     edges(imp/unch/reg)={improved_edges}/{unchanged_edges}/{regressed_edges} \
                     near_anti(new/rem/new_stab)={newly_near_anti}/{removed_near_anti}/{newly_near_anti_stab} \
                     exact_m1(new/rem/new_stab)={newly_exact_m1}/{removed_exact_m1}/{newly_exact_m1_stab} \
                     level_b_misses={level_b_misses}",
                    candidate.name,
                    profile,
                    min_raw_ratio.map(|r| (r.0, r.1)),
                    min_stab_len_ratio.map(|r| (r.0, r.1)),
                    min_stab_proj_ratio.map(|r| (r.0, r.1)),
                    geometry_fps.len(),
                    avg_disp_min.0, avg_disp_min.1,
                    avg_disp_max.0, avg_disp_max.1,
                    max_disp_max.0, max_disp_max.1,
                    min_edge_min.0, min_edge_min.1,
                    max_angle_max.0, max_angle_max.1,
                    min_aspect_min.0, min_aspect_min.1,
                    min_dot_neither,
                    min_one_stab_dot, min_dot_origin, min_dot_dest,
                    min_dot_both,
                    largest_imp.0, largest_imp.1, largest_imp.2, largest_imp.3, largest_imp.4, largest_imp.5,
                    largest_reg.0, largest_reg.1, largest_reg.2, largest_reg.3, largest_reg.4, largest_reg.5,
                );

                if candidate.name == "1/64_len" {
                    assert_eq!(
                        rejected_count, 0,
                        "production candidate must generate cleanly"
                    );
                    assert!(
                        conn_fps_match,
                        "production connectivity fingerprint must match raw"
                    );
                    assert_eq!(
                        level_b_misses, 0,
                        "production candidate must pass all Level B criteria"
                    );
                    assert_eq!(
                        reduction_rounds_total, 0,
                        "production candidate must be backoff free"
                    );
                    assert_eq!(reduced_vertices_total, 0);
                    assert_eq!(reduced_disp_fallbacks, 0);
                    assert_eq!(regular_pos_fallbacks, 0);
                    // assert_eq!(
                    //     newly_near_anti_stab, 0,
                    //     "production candidate no newly near-anti stabilized edge"
                    // );
                    assert_eq!(
                        newly_exact_m1_stab, 0,
                        "production candidate no newly exact -1.0 stabilized edge"
                    );
                    assert!(
                        min_dot_both >= -0.1,
                        "production candidate both-stabilized min dot >= -0.1"
                    );

                    if profile == HexDeformationProfile::Organic {
                        assert!(max_angle_max.0 <= 162.0, "Organic max angle <= 162°");
                        assert!(min_aspect_min.0 >= 0.490, "Organic min aspect >= 0.490");
                    } else if profile == HexDeformationProfile::PagoniaLike {
                        assert!(max_angle_max.0 <= 176.0, "PagoniaLike max angle <= 176°");
                        assert!(min_aspect_min.0 >= 0.370, "PagoniaLike min aspect >= 0.370");
                    }
                }
            }
        }
    }
}
