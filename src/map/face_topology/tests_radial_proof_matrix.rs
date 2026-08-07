//! Exhaustive proof matrix and invariant test suite for radial blend stabilization.

#[cfg(test)]
#[rustfmt::skip]
mod radial_proof_matrix_tests {
    use crate::map::face_topology::acceptance::{ProfileAcceptanceCriteria, ProfileAcceptanceReport};
    use crate::map::face_topology::blend::{blend_to_displacement_q16, weighted_blend_diagnostics, FixedVectorQ16, WeightedBlendDiagnostics};
    use crate::map::face_topology::blend_diagnostics::{div_away_from_zero, scale_radial_component_q16};
    use crate::map::face_topology::blend_policy::DISABLED_BLEND_RELIABILITY_POLICY;
    use crate::map::face_topology::corner_key::regular_corner_position;
    use crate::map::face_topology::fingerprint::topology_fingerprints;
    use crate::map::face_topology::profiles::{interpolated_correlated_field, local_component_q16, HexDeformationProfile};
    use crate::map::face_topology::tests_blend_candidate_shared::shared as c;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::types::{HexFaceTopology, MapVertex, SharedCornerKey};
    use crate::map::{HexCoord, MapData, WorldSeed};
    use std::collections::HashMap;

    fn observations(seed: u32, key: SharedCornerKey, profile: HexDeformationProfile) -> (WeightedBlendDiagnostics, FixedVectorQ16) {
        let cfg = profile.config();
        let corr = interpolated_correlated_field(seed, key, profile);
        let loc = local_component_q16(seed, key, profile);
        (
            weighted_blend_diagnostics(corr, loc, cfg.correlated_weight_q16, cfg.local_weight_q16),
            blend_to_displacement_q16(corr, loc, cfg.correlated_weight_q16, cfg.local_weight_q16),
        )
    }

    fn unique_edge_pairs(topo: &HexFaceTopology) -> impl Iterator<Item = (&MapVertex, &MapVertex)> + '_ {
        topo.half_edges.iter().enumerate().filter(|(idx, edge)| edge.twin.is_none_or(|twin| *idx < twin.index())).map(|(_, edge)| {
            (&topo.vertices[edge.origin.index()], &topo.vertices[edge.destination.index()])
        })
    }

    fn isqrt(n: u128) -> u128 {
        if n == 0 { return 0; }
        let bits = 128 - n.leading_zeros();
        let mut x = 1_u128 << ((bits + 1) / 2);
        while x * x > n || (x + 1) * (x + 1) <= n { x = (x + n / x) / 2; }
        x
    }

    /// Pure integer mathematical property proof establishing floor reach without f64.
    #[test]
    fn full_radial_stabilization_arithmetic_property_proof() {
        for wx in -200_i64..=200 {
            for wy in -200_i64..=200 {
                if wx == 0 && wy == 0 { continue; }
                let s = (wx as i128 * wx as i128 + wy as i128 * wy as i128) as u128;
                let w = isqrt(s) as i64;
                if w < 1 { continue; }
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

    /// Full 12-way perturbation matrix across ALL corrected corners with incident-edge safety tracking.
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    fn full_radial_stabilization_perturbation_matrix() {
        let map = q::map_40x40();
        let exact_m1_bits = (-1.0_f32).to_bits();
        let mut executed = 0_usize;
        let mut skipped = 0_usize;
        let mut corrected_corners_count = 0_usize;

        for seed in 0..256_u32 {
            for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
                let cfg = profile.config();
                let raw_topo = c::generate(&map, seed, profile, DISABLED_BLEND_RELIABILITY_POLICY);
                let prod_topo = q::generate(&map, seed, profile);
                let edge_pairs: Vec<(&MapVertex, &MapVertex)> = unique_edge_pairs(&prod_topo).collect();
                let mut raw_edge_map = HashMap::new();
                for (o, d) in unique_edge_pairs(&raw_topo) {
                    let (Ok(or), Ok(dr)) = (regular_corner_position(o.canonical_key), regular_corner_position(d.canonical_key)) else { continue; };
                    let (od, dd) = ((o.position - or).normalize_or_zero(), (d.position - dr).normalize_or_zero());
                    raw_edge_map.insert((o.canonical_key.min(d.canonical_key), o.canonical_key.max(d.canonical_key)), od.dot(dd));
                }

                for vertex in &prod_topo.vertices {
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

                    let Ok(rv) = regular_corner_position(vertex.canonical_key) else { continue; };

                    for (p_corr, p_loc, p_wc, p_wl) in pert_inputs {
                        let p_diag = weighted_blend_diagnostics(p_corr, p_loc, p_wc, p_wl);
                        let pl = p_diag.weighted_length_q16;
                        let req = p_diag.minimum_reliable_length_q16;
                        if pl >= 1 {
                            let psx = scale_radial_component_q16(p_diag.weighted_x_q16, req, pl);
                            let psy = scale_radial_component_q16(p_diag.weighted_y_q16, req, pl);
                            let pst_s = (psx as i128 * psx as i128 + psy as i128 * psy as i128) as u128;
                            let pst_len = isqrt(pst_s) as i64;
                            assert!(pst_len >= req, "perturbed length >= floor");

                            let p_disp = blend_to_displacement_q16(p_corr, p_loc, p_wc, p_wl);
                            let v_pert = rv + bevy::prelude::Vec2::new(p_disp.x as f32 / 65536.0, p_disp.y as f32 / 65536.0);
                            let dir_v_pert = (v_pert - rv).normalize_or_zero();

                            for (o, d) in &edge_pairs {
                                let u = if o.canonical_key == vertex.canonical_key { d } else if d.canonical_key == vertex.canonical_key { o } else { continue; };
                                let Ok(ru) = regular_corner_position(u.canonical_key) else { continue; };
                                let dir_u = (u.position - ru).normalize_or_zero();
                                let p_edge_dot = dir_v_pert.dot(dir_u);
                                let key = (vertex.canonical_key.min(u.canonical_key), vertex.canonical_key.max(u.canonical_key));

                                if let Some(&raw_dot) = raw_edge_map.get(&key) {
                                    if raw_dot > -0.98 {
                                        assert_ne!(p_edge_dot.to_bits(), exact_m1_bits, "raw_dot > -0.98 must not become exact -1.0 under perturbation");
                                        assert!(p_edge_dot > -0.9995, "raw_dot > -0.98 must stay > -0.9995 under perturbation");
                                    }
                                    if raw_dot > 0.0 {
                                        assert!(p_edge_dot > -0.9995, "positive raw_dot must stay > -0.9995 under perturbation");
                                    }
                                }
                            }

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
        assert_eq!(zero_count, 0, "natural exact-zero weighted vectors is 0 across full Stage 2 domain");
    }

    /// Near-antiparallel & exact -1.0 inventory across ALL 256 seeds.
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    fn full_radial_stabilization_adjacency_inventory() {
        let map = q::map_40x40();
        let exact_m1_bits = (-1.0_f32).to_bits();
        let mut pos_to_near = 0_usize;
        let mut raw_above_m98_to_near = 0_usize;
        let mut newly_exact_m1 = 0_usize;

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
                        if p_dot.to_bits() == exact_m1_bits && r_dot.to_bits() != exact_m1_bits { newly_exact_m1 += 1; }
                    }
                }
            }
        }
        assert_eq!(pos_to_near, 0, "positive raw dot must never transition to near-antiparallel <= -0.9995");
        assert_eq!(raw_above_m98_to_near, 0, "raw dot > -0.98 must never transition to near-antiparallel <= -0.9995");
        assert_eq!(newly_exact_m1, 0, "0 newly created exact -1.0 edges involving corrected endpoints");
    }

    /// Full Stage 2 matrix test (2 profiles x 6 grid shapes x 256 seeds = 3,072 topologies) checking full acceptance criteria.
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    fn full_radial_stabilization_stage2_matrix() {
        for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
            let criteria = ProfileAcceptanceCriteria::for_profile(profile);
            for (shape_name, map) in q::all_shapes() {
                let mut generated_count = 0_usize;
                for seed in 0..256_u32 {
                    let topo = q::generate(&map, seed, profile);
                    generated_count += 1;
                    let rep = ProfileAcceptanceReport::from_topology(&topo);
                    let violations = rep.violations(criteria);
                    assert!(violations.is_empty(), "{profile:?} shape {shape_name} seed {seed}: violations: {violations:?}");
                }
                assert_eq!(generated_count, 256, "all 256 seeds generated for {profile:?} shape {shape_name}");
            }
        }
    }

    /// Insertion-order independence & full fingerprint determinism test with deterministic shuffled maps.
    #[test]
    fn full_radial_stabilization_determinism_matrix() {
        for (_shape_name, map) in q::all_shapes() {
            let mut rev_map = MapData::default();
            let mut shuf_map = MapData::default();
            rev_map.width = map.width; rev_map.height = map.height;
            shuf_map.width = map.width; shuf_map.height = map.height;
            let mut coords: Vec<HexCoord> = map.tiles.keys().copied().collect();
            for &coord in coords.iter().rev() { rev_map.tiles.insert(coord, q::tile()); }
            // Deterministic LCG swap permutation
            let mut rng_state = 0x1234_5678_u32;
            for i in (1..coords.len()).rev() {
                rng_state = rng_state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                let j = (rng_state as usize) % (i + 1);
                coords.swap(i, j);
            }
            for coord in coords { shuf_map.tiles.insert(coord, q::tile()); }

            for seed in [0_u32, 1, 42, 64, 126, 173, 194, 255] {
                for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
                    let topo1 = q::generate(&map, seed, profile);
                    let topo_rev = q::generate(&rev_map, seed, profile);
                    let topo_shuf = q::generate(&shuf_map, seed, profile);
                    let fp1 = topology_fingerprints(&map, WorldSeed::new(seed), &topo1);
                    let fp_rev = topology_fingerprints(&rev_map, WorldSeed::new(seed), &topo_rev);
                    let fp_shuf = topology_fingerprints(&shuf_map, WorldSeed::new(seed), &topo_shuf);
                    assert_eq!(fp1.geometry, fp_rev.geometry); assert_eq!(fp1.connectivity, fp_rev.connectivity);
                    assert_eq!(fp1.geometry, fp_shuf.geometry); assert_eq!(fp1.connectivity, fp_shuf.connectivity);
                }
            }
        }
    }
}
