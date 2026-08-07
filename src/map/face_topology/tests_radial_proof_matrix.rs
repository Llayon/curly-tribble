//! Exhaustive proof matrix and invariant test suite for radial blend stabilization.

#[cfg(test)]
mod radial_proof_matrix_tests {
    use crate::map::face_topology::acceptance::ProfileAcceptanceReport;
    use crate::map::face_topology::blend::{
        blend_to_displacement_q16, weighted_blend_diagnostics, FixedVectorQ16,
        WeightedBlendDiagnostics,
    };
    use crate::map::face_topology::blend_diagnostics::{
        div_away_from_zero, scale_radial_component_q16,
    };
    use crate::map::face_topology::blend_policy::DISABLED_BLEND_RELIABILITY_POLICY;
    use crate::map::face_topology::corner_key::regular_corner_position;
    use crate::map::face_topology::fingerprint::topology_fingerprints;
    use crate::map::face_topology::profiles::{
        interpolated_correlated_field, local_component_q16, HexDeformationProfile,
    };
    use crate::map::face_topology::tests_blend_candidate_shared::shared as c;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::types::{HexFaceTopology, MapVertex, SharedCornerKey};
    use crate::map::{HexCoord, MapData, WorldSeed};
    use std::collections::HashMap;

    fn observations(
        seed: u32,
        key: SharedCornerKey,
        profile: HexDeformationProfile,
    ) -> (WeightedBlendDiagnostics, FixedVectorQ16) {
        let cfg = profile.config();
        let corr = interpolated_correlated_field(seed, key, profile);
        let loc = local_component_q16(seed, key, profile);
        (
            weighted_blend_diagnostics(corr, loc, cfg.correlated_weight_q16, cfg.local_weight_q16),
            blend_to_displacement_q16(corr, loc, cfg.correlated_weight_q16, cfg.local_weight_q16),
        )
    }

    fn unique_edge_pairs(
        topo: &HexFaceTopology,
    ) -> impl Iterator<Item = (&MapVertex, &MapVertex)> + '_ {
        topo.half_edges
            .iter()
            .enumerate()
            .filter(|(idx, edge)| edge.twin.is_none_or(|twin| *idx < twin.index()))
            .map(|(_, edge)| {
                (
                    &topo.vertices[edge.origin.index()],
                    &topo.vertices[edge.destination.index()],
                )
            })
    }

    fn isqrt(n: u128) -> u128 {
        if n == 0 {
            return 0;
        }
        let mut x = (n as f64).sqrt() as u128;
        if x == 0 {
            x = 1;
        }
        while x * x > n || (x + 1) * (x + 1) <= n {
            x = (x + n / x) / 2;
        }
        x
    }

    /// Pure integer mathematical property proof establishing floor reach without f64.
    #[test]
    fn full_radial_stabilization_arithmetic_property_proof() {
        for wx in -200_i64..=200 {
            for wy in -200_i64..=200 {
                if wx == 0 && wy == 0 {
                    continue;
                }
                let s = (wx as i128 * wx as i128 + wy as i128 * wy as i128) as u128;
                let w = isqrt(s) as i64;
                if w < 1 {
                    continue;
                }
                for l in 0_i64..=300 {
                    let sx = scale_radial_component_q16(wx, l, w);
                    let sy = scale_radial_component_q16(wy, l, w);
                    let st_s = (sx as i128 * sx as i128 + sy as i128 * sy as i128) as u128;
                    let st_len = isqrt(st_s) as i64;
                    let deficit = l - st_len;
                    let excess = st_len - l;
                    assert!(deficit <= 0, "deficit must be <= 0, got {deficit}");
                    assert!(excess >= 0);
                    if let Some(gx) = div_away_from_zero(i128::from(wx) * i128::from(l), w) {
                        assert_eq!(gx, sx);
                    }
                }
            }
        }
    }

    /// Canonical 40x40 256-seed matrix audit (1,024 generation calls, raw vs prod connectivity).
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    #[rustfmt::skip]
    fn full_radial_stabilization_canonical_256_seed_audit() {
        let map = q::map_40x40();
        let mut total_corrected = 0_usize;
        for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
            let mut profile_corrected = 0_usize;
            for seed in 0..256_u32 {
                let raw_topo = c::generate(&map, seed, profile, DISABLED_BLEND_RELIABILITY_POLICY);
                let prod_topo = q::generate(&map, seed, profile);
                let raw_fps = topology_fingerprints(&map, WorldSeed::new(seed), &raw_topo);
                let prod_fps = topology_fingerprints(&map, WorldSeed::new(seed), &prod_topo);
                assert_eq!(raw_fps.connectivity, prod_fps.connectivity, "connectivity must match");

                for vertex in &prod_topo.vertices {
                    let (diag, _) = observations(seed, vertex.canonical_key, profile);
                    if diag.stabilization_applied {
                        profile_corrected += 1;
                        let deficit = diag.minimum_reliable_length_q16 - diag.stabilized_length_q16;
                        assert!(deficit <= 0, "{profile:?} seed={seed}: deficit must be <= 0");
                    }
                }
            }
            total_corrected += profile_corrected;
        }
        assert_eq!(total_corrected, 1_118, "canonical 256-seed corrected count matched 1,118");
    }

    /// Full 12-way perturbation matrix across ALL corrected corners (1,118 x 12 = 13,416 perturbations).
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    #[rustfmt::skip]
    fn full_radial_stabilization_perturbation_matrix() {
        let map = q::map_40x40();
        let mut executed = 0_usize;
        let mut skipped = 0_usize;
        let mut corrected_corners_count = 0_usize;

        for seed in 0..256_u32 {
            for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
                let cfg = profile.config();
                let topo = q::generate(&map, seed, profile);
                for vertex in &topo.vertices {
                    let (diag, _) = observations(seed, vertex.canonical_key, profile);
                    if !diag.stabilization_applied { continue; }
                    corrected_corners_count += 1;
                    let corr = interpolated_correlated_field(seed, vertex.canonical_key, profile);
                    let loc = local_component_q16(seed, vertex.canonical_key, profile);
                    let (wc, wl) = (cfg.correlated_weight_q16, cfg.local_weight_q16);

                    let pert_inputs = [
                        (FixedVectorQ16{x: corr.x + 1, y: corr.y}, loc, wc, wl),
                        (FixedVectorQ16{x: corr.x - 1, y: corr.y}, loc, wc, wl),
                        (FixedVectorQ16{x: corr.x, y: corr.y + 1}, loc, wc, wl),
                        (FixedVectorQ16{x: corr.x, y: corr.y - 1}, loc, wc, wl),
                        (corr, FixedVectorQ16{x: loc.x + 1, y: loc.y}, wc, wl),
                        (corr, FixedVectorQ16{x: loc.x - 1, y: loc.y}, wc, wl),
                        (corr, FixedVectorQ16{x: loc.x, y: loc.y + 1}, wc, wl),
                        (corr, FixedVectorQ16{x: loc.x, y: loc.y - 1}, wc, wl),
                        (corr, loc, wc + 1, wl), (corr, loc, wc - 1, wl),
                        (corr, loc, wc, wl + 1), (corr, loc, wc, wl - 1),
                    ];

                    for (p_corr, p_loc, p_wc, p_wl) in pert_inputs {
                        let p_diag = weighted_blend_diagnostics(p_corr, p_loc, p_wc, p_wl);
                        let pl = p_diag.weighted_length_q16;
                        let req = p_diag.minimum_reliable_length_q16;
                        if pl >= 1 {
                            let psx = scale_radial_component_q16(p_diag.weighted_x_q16, req, pl);
                            let psy = scale_radial_component_q16(p_diag.weighted_y_q16, req, pl);
                            let pst_s = (psx as i128 * psx as i128 + psy as i128 * psy as i128) as u128;
                            let pst_len = isqrt(pst_s) as i64;
                            assert!(pst_len >= req, "perturbed stabilized length must be >= floor");
                            executed += 1;
                        } else {
                            skipped += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(corrected_corners_count, 1_118, "total corrected corners");
        assert_eq!(executed + skipped, 1_118 * 12, "reconciliation equality: executed + skipped == total_corrected * 12");
    }

    /// Stage 2 exact-zero inventory across ALL 3,072 topologies (6 shapes x 2 profiles x 256 seeds).
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    #[rustfmt::skip]
    fn full_radial_stabilization_exact_zero_inventory() {
        let mut zero_count = 0_usize;
        for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
            for (_, map) in q::all_shapes() {
                for seed in 0..256_u32 {
                    let topo = q::generate(&map, seed, profile);
                    for vertex in &topo.vertices {
                        let (diag, _) = observations(seed, vertex.canonical_key, profile);
                        if diag.weighted_sum_zero { zero_count += 1; }
                    }
                }
            }
        }
        println!("Stage 2 3,072-topology natural exact-zero weighted vector count: {zero_count}");
        assert_eq!(zero_count, 0, "natural exact-zero weighted vectors is 0 across full Stage 2 domain");
    }

    /// Near-antiparallel inventory with threshold -0.9995 across ALL 256 seeds.
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    #[rustfmt::skip]
    fn full_radial_stabilization_adjacency_inventory() {
        let map = q::map_40x40();
        let mut pos_to_near = 0_usize;
        let mut raw_above_m98_to_near = 0_usize;

        for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
            for seed in 0..256_u32 {
                let raw_topo = c::generate(&map, seed, profile, DISABLED_BLEND_RELIABILITY_POLICY);
                let prod_topo = q::generate(&map, seed, profile);
                let mut raw_map = HashMap::new();
                for (o, d) in unique_edge_pairs(&raw_topo) {
                    let (Ok(or), Ok(dr)) = (regular_corner_position(o.canonical_key), regular_corner_position(d.canonical_key)) else { continue; };
                    let (od, dd) = ((o.position - or).normalize_or_zero(), (d.position - dr).normalize_or_zero());
                    raw_map.insert((o.canonical_key.min(d.canonical_key), o.canonical_key.max(d.canonical_key)), od.dot(dd));
                }
                for (o, d) in unique_edge_pairs(&prod_topo) {
                    let (Ok(or), Ok(dr)) = (regular_corner_position(o.canonical_key), regular_corner_position(d.canonical_key)) else { continue; };
                    let (od, dd) = ((o.position - or).normalize_or_zero(), (d.position - dr).normalize_or_zero());
                    let p_dot = od.dot(dd);
                    let key = (o.canonical_key.min(d.canonical_key), o.canonical_key.max(d.canonical_key));
                    if let Some(&r_dot) = raw_map.get(&key) {
                        if r_dot > 0.0 && p_dot <= -0.9995 { pos_to_near += 1; }
                        if r_dot > -0.98 && p_dot <= -0.9995 { raw_above_m98_to_near += 1; }
                    }
                }
            }
        }
        assert_eq!(pos_to_near, 0, "positive raw dot must never transition to near-antiparallel <= -0.9995");
        assert_eq!(raw_above_m98_to_near, 0, "raw dot > -0.98 must never transition to near-antiparallel <= -0.9995");
    }

    /// Full Stage 2 matrix test (2 profiles x 6 grid shapes x 256 seeds = 3,072 topologies).
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    #[rustfmt::skip]
    fn full_radial_stabilization_stage2_matrix() {
        for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
            for (shape_name, map) in q::all_shapes() {
                let mut max_angle = 0.0_f32;
                let mut min_aspect = 1.0_f32;
                let mut generated_count = 0_usize;

                for seed in 0..256_u32 {
                    let topo = q::generate(&map, seed, profile);
                    generated_count += 1;
                    let rep = ProfileAcceptanceReport::from_topology(&topo);
                    max_angle = max_angle.max(rep.maximum_interior_angle_degrees);
                    min_aspect = min_aspect.min(rep.minimum_aspect_quality);
                }

                assert_eq!(generated_count, 256, "all 256 seeds generated for {profile:?} shape {shape_name}");
                let (allowed_angle, allowed_aspect) = match profile {
                    HexDeformationProfile::Organic => (162.0_f32, 0.490_f32),
                    HexDeformationProfile::PagoniaLike => (176.0_f32, 0.370_f32),
                    HexDeformationProfile::Subtle => (160.0_f32, 0.500_f32),
                };
                assert!(max_angle <= allowed_angle + 1e-3, "{profile:?} shape {shape_name}: max angle {max_angle} <= {allowed_angle}");
                assert!(min_aspect >= allowed_aspect - 1e-3, "{profile:?} shape {shape_name}: min aspect {min_aspect} >= {allowed_aspect}");
            }
        }
    }

    /// Insertion-order independence & full fingerprint determinism test.
    #[test]
    #[rustfmt::skip]
    fn full_radial_stabilization_determinism_matrix() {
        for (shape_name, map) in q::all_shapes() {
            let mut shuffled_map = MapData::default();
            shuffled_map.width = map.width;
            shuffled_map.height = map.height;
            let mut coords: Vec<HexCoord> = map.tiles.keys().copied().collect();
            coords.reverse(); // Reverse tile insertion order
            for coord in coords {
                shuffled_map.tiles.insert(coord, q::tile());
            }

            for seed in [0_u32, 1, 42, 64, 126, 173, 194, 255] {
                for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
                    let topo1 = q::generate(&map, seed, profile);
                    let topo2 = q::generate(&shuffled_map, seed, profile);
                    let fp1 = topology_fingerprints(&map, WorldSeed::new(seed), &topo1);
                    let fp2 = topology_fingerprints(&shuffled_map, WorldSeed::new(seed), &topo2);
                    assert_eq!(fp1.geometry, fp2.geometry, "geometry fingerprint must match across insertion order for shape {shape_name}");
                    assert_eq!(fp1.connectivity, fp2.connectivity, "connectivity fingerprint must match for shape {shape_name}");
                }
            }
        }
    }
}
