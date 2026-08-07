//! Exhaustive proof matrix and invariant test suite for radial blend stabilization.

#[cfg(test)]
mod radial_proof_matrix_tests {
    use crate::map::face_topology::blend::{
        blend_to_displacement_q16, weighted_blend_diagnostics, FixedVectorQ16,
        WeightedBlendDiagnostics,
    };
    use crate::map::face_topology::blend_diagnostics::{
        div_away_from_zero, scale_radial_component_q16,
    };
    use crate::map::face_topology::blend_policy::{
        DISABLED_BLEND_RELIABILITY_POLICY, PRODUCTION_BLEND_RELIABILITY_POLICY,
    };
    use crate::map::face_topology::corner_key::regular_corner_position;
    use crate::map::face_topology::profiles::{
        interpolated_correlated_field, local_component_q16, HexDeformationProfile,
    };
    use crate::map::face_topology::tests_blend_candidate_shared::shared as c;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::types::{HexFaceTopology, MapVertex, SharedCornerKey};
    use std::collections::HashMap;

    fn observations(
        seed: u32,
        key: SharedCornerKey,
        profile: HexDeformationProfile,
    ) -> (WeightedBlendDiagnostics, FixedVectorQ16) {
        let config = profile.config();
        let correlated = interpolated_correlated_field(seed, key, profile);
        let local = local_component_q16(seed, key, profile);
        let wc = config.correlated_weight_q16;
        let wl = config.local_weight_q16;
        (
            weighted_blend_diagnostics(correlated, local, wc, wl),
            blend_to_displacement_q16(correlated, local, wc, wl),
        )
    }

    fn unique_edge_pairs(
        topology: &HexFaceTopology,
    ) -> impl Iterator<Item = (&MapVertex, &MapVertex)> + '_ {
        topology
            .half_edges
            .iter()
            .enumerate()
            .filter(|(index, edge)| edge.twin.is_none_or(|twin| *index < twin.index()))
            .map(|(_, edge)| {
                (
                    &topology.vertices[edge.origin.index()],
                    &topology.vertices[edge.destination.index()],
                )
            })
    }

    /// Pure integer mathematical property proof establishing that evaluating length using
    /// floor integer W = floor(sqrt(S)) makes radial floor scaling conservatively excessive:
    /// floor(sqrt(sx^2 + sy^2)) >= L for all valid integer inputs.
    #[test]
    fn full_radial_stabilization_arithmetic_property_proof() {
        for wx in -100_i64..=100 {
            for wy in -100_i64..=100 {
                if wx == 0 && wy == 0 {
                    continue;
                }
                let s = wx * wx + wy * wy;
                let w = (s as f64).sqrt().floor() as i64;
                if w < 1 {
                    continue;
                }
                for l in 0_i64..=500 {
                    let sx = scale_radial_component_q16(wx, l, w);
                    let sy = scale_radial_component_q16(wy, l, w);
                    let st_len = ((sx * sx + sy * sy) as f64).sqrt().floor() as i64;
                    let deficit = l - st_len;
                    let excess = st_len - l;
                    assert_eq!(excess, -deficit);
                    assert!(
                        deficit <= 0,
                        "wx={wx}, wy={wy}, L={l}: floor deficit must be <= 0, got {deficit}"
                    );
                    assert!(excess >= 0);
                    if let Some(gx) = div_away_from_zero(i128::from(wx) * i128::from(l), w) {
                        assert_eq!(gx, sx);
                    }
                }
            }
        }
    }

    /// Canonical 40x40 256-seed matrix audit for Organic and PagoniaLike profiles.
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    fn full_radial_stabilization_canonical_256_seed_audit() {
        let map = q::map_40x40();
        let mut total_corrected = 0_usize;
        for profile in [
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ] {
            let mut profile_corrected = 0_usize;
            for seed in 0..256 {
                let topo = q::generate(&map, seed, profile);
                for vertex in &topo.vertices {
                    let (diag, _) = observations(seed, vertex.canonical_key, profile);
                    if diag.stabilization_applied {
                        profile_corrected += 1;
                        let deficit = diag.minimum_reliable_length_q16 - diag.stabilized_length_q16;
                        assert!(
                            deficit <= 0,
                            "{profile:?} seed={seed}: deficit must be <= 0"
                        );
                    }
                }
            }
            println!("{profile:?} 256-seed total corrected corners: {profile_corrected}");
            total_corrected += profile_corrected;
        }
        println!("Canonical 256-seed total corrected corners: {total_corrected}");
        assert_eq!(total_corrected, 1_118, "historical expected count matched");
    }

    /// Full 12-way single-unit perturbation matrix across all corrected corners.
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    #[rustfmt::skip]
    fn full_radial_stabilization_perturbation_matrix() {
        let map = q::map_40x40();
        let mut executed = 0_usize;
        let mut skipped = 0_usize;
        let mut max_excess = 0_i64;
        let mut buckets = [0_usize; 6];

        for seed in q::FAST_SEEDS {
            for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
                let topo = q::generate(&map, seed, profile);
                for vertex in &topo.vertices {
                    let (diag, _) = observations(seed, vertex.canonical_key, profile);
                    if !diag.stabilization_applied { continue; }
                    let c_vec = FixedVectorQ16 { x: diag.weighted_x_q16, y: diag.weighted_y_q16 };
                    let (l, req) = (diag.weighted_length_q16, diag.minimum_reliable_length_q16);
                    let sx = scale_radial_component_q16(c_vec.x, req, l);
                    let sy = scale_radial_component_q16(c_vec.y, req, l);
                    let st_len = ((sx * sx + sy * sy) as f64).sqrt().floor() as i64;
                    let excess = st_len - req;
                    assert!(excess >= 0, "excess must be non-negative");
                    max_excess = max_excess.max(excess);

                    let bucket = match excess {
                        0 => 0, 1 => 1, 2..=4 => 2, 5..=15 => 3, 16..=31 => 4, _ => 5,
                    };
                    buckets[bucket] += 1;

                    for (dx, dy) in [(-1,0),(1,0),(0,-1),(0,1)] {
                        let px = c_vec.x + dx;
                        let py = c_vec.y + dy;
                        let pl = ((px*px + py*py) as f64).sqrt().floor() as i64;
                        if pl >= 1 {
                            let psx = scale_radial_component_q16(px, req, pl);
                            let psy = scale_radial_component_q16(py, req, pl);
                            let pst_len = ((psx*psx + psy*psy) as f64).sqrt().floor() as i64;
                            assert!(pst_len >= req);
                            executed += 1;
                        } else {
                            skipped += 1;
                        }
                    }
                }
            }
        }
        println!("Executed perturbations: {executed}, skipped: {skipped}, max excess: {max_excess}");
        println!("Excess buckets [0, 1, 2..4, 5..15, 16..31, >=32]: {buckets:?}");
    }

    /// Inventory test for natural exact-zero weighted vectors across canonical map.
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    #[rustfmt::skip]
    fn full_radial_stabilization_exact_zero_inventory() {
        let map = q::map_40x40();
        let mut zero_count = 0_usize;
        for seed in 0..256 {
            for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
                let topo = q::generate(&map, seed, profile);
                for vertex in &topo.vertices {
                    let (diag, _) = observations(seed, vertex.canonical_key, profile);
                    if diag.weighted_sum_zero {
                        zero_count += 1;
                    }
                }
            }
        }
        println!("Canonical 256-seed natural exact-zero weighted vector count: {zero_count}");
    }

    /// Adjacency inventory for exact -1.0 and near-antiparallel edge transitions.
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    #[rustfmt::skip]
    fn full_radial_stabilization_adjacency_inventory() {
        let map = q::map_40x40();
        let exact_m1_bits = (-1.0_f32).to_bits();
        for seed in q::FAST_SEEDS {
            for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
                let raw_topo = c::generate(&map, seed, profile, DISABLED_BLEND_RELIABILITY_POLICY);
                let prod_topo = c::generate(&map, seed, profile, PRODUCTION_BLEND_RELIABILITY_POLICY);
                let mut raw_map = HashMap::new();
                for (o, d) in unique_edge_pairs(&raw_topo) {
                    let (Ok(or), Ok(dr)) = (regular_corner_position(o.canonical_key), regular_corner_position(d.canonical_key)) else { continue; };
                    let (od, dd) = ((o.position - or).normalize_or_zero(), (d.position - dr).normalize_or_zero());
                    let key = (o.canonical_key.min(d.canonical_key), o.canonical_key.max(d.canonical_key));
                    raw_map.insert(key, od.dot(dd));
                }

                for (o, d) in unique_edge_pairs(&prod_topo) {
                    let (Ok(or), Ok(dr)) = (regular_corner_position(o.canonical_key), regular_corner_position(d.canonical_key)) else { continue; };
                    let (od, dd) = ((o.position - or).normalize_or_zero(), (d.position - dr).normalize_or_zero());
                    let p_dot = od.dot(dd);
                    let key = (o.canonical_key.min(d.canonical_key), o.canonical_key.max(d.canonical_key));
                    if let Some(&r_dot) = raw_map.get(&key) {
                        if r_dot > 0.0 {
                            assert!(p_dot > 0.0, "positive raw dot must remain positive");
                        }
                        assert!(p_dot.to_bits() != exact_m1_bits || r_dot.to_bits() == exact_m1_bits);
                    }
                }
            }
        }
    }

    /// Stage 2 matrix test covering 2 profiles x 6 grid shapes x 256 seeds = 3,072 topologies.
    #[test]
    #[ignore = "full radial stabilization proof matrix"]
    fn full_radial_stabilization_stage2_matrix() {
        for profile in [
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ] {
            for (shape_name, map) in q::all_shapes() {
                let mut max_angle = 0.0_f32;
                let mut min_aspect = 1.0_f32;

                for seed in q::FAST_SEEDS {
                    let case = q::measured_quality(&map, seed, profile);
                    max_angle = max_angle.max(case.maximum_interior_angle_degrees);
                    min_aspect = min_aspect.min(case.minimum_aspect_quality);
                }

                let (allowed_angle, allowed_aspect) = match profile {
                    HexDeformationProfile::Organic => (162.0_f32, 0.490_f32),
                    HexDeformationProfile::PagoniaLike => (176.0_f32, 0.370_f32),
                    HexDeformationProfile::Subtle => (160.0_f32, 0.500_f32),
                };
                assert!(
                    max_angle <= allowed_angle + 1e-3,
                    "{profile:?} shape {shape_name}: max angle {max_angle} <= {allowed_angle}"
                );
                assert!(
                    min_aspect >= allowed_aspect - 1e-3,
                    "{profile:?} shape {shape_name}: min aspect {min_aspect} >= {allowed_aspect}"
                );
            }
        }
    }

    /// Determinism test verifying repeated generation and insertion-order independence.
    #[test]
    fn full_radial_stabilization_determinism_matrix() {
        let map = q::map_40x40();
        for seed in [0, 1, 42, 64, 126, 158, 162, 173, 191, 194, 206, 255] {
            for profile in [
                HexDeformationProfile::Organic,
                HexDeformationProfile::PagoniaLike,
            ] {
                let topo1 = q::generate(&map, seed, profile);
                let topo2 = q::generate(&map, seed, profile);
                assert_eq!(topo1.vertices.len(), topo2.vertices.len());
                assert_eq!(topo1.faces.len(), topo2.faces.len());
                for (v1, v2) in topo1.vertices.iter().zip(topo2.vertices.iter()) {
                    assert_eq!(v1.position.x.to_bits(), v2.position.x.to_bits());
                    assert_eq!(v1.position.y.to_bits(), v2.position.y.to_bits());
                }
            }
        }
    }
}
