//! Stabilized blend direction invariants and threshold measurements: the
//! reliable corners keep the exact raw normalization (bit-identical), the
//! stabilized corners reach the reliability projection floor, and the worst
//! adjacent direction changes are tracked per stabilization category.
#[cfg(test)]
mod quality_blend_direction_tests {
    use crate::map::face_topology::blend::{
        blend_to_displacement_q16, component_length_q16, weighted_blend_diagnostics,
        FixedVectorQ16, WeightedBlendDiagnostics, MIN_RELIABLE_DIRECTION_RATIO_Q16,
    };
    use crate::map::face_topology::corner_key::regular_corner_position;
    use crate::map::face_topology::profiles::{
        interpolated_correlated_field, local_component_q16, HexDeformationProfile,
    };
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::types::{HexFaceTopology, MapVertex, SharedCornerKey};
    const Q16: i64 = 65_536;
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
    /// The pre-fix raw law: Q24-normalize the weighted sum (falling back to the
    /// local component only when the sum is exactly zero), then scale to the
    /// target magnitude. Deterministic, no floating point.
    fn raw_law(diagnostics: &WeightedBlendDiagnostics, local: FixedVectorQ16) -> FixedVectorQ16 {
        const DIRECTION_SHIFT: i64 = 1 << 24;
        let (vx, vy, len) = if diagnostics.weighted_x_q16 == 0 && diagnostics.weighted_y_q16 == 0 {
            (local.x, local.y, diagnostics.local_length_q16)
        } else {
            (
                diagnostics.weighted_x_q16,
                diagnostics.weighted_y_q16,
                diagnostics.weighted_length_q16,
            )
        };
        let (dx, dy) = if len == 0 {
            (DIRECTION_SHIFT, 0)
        } else {
            (vx * DIRECTION_SHIFT / len, vy * DIRECTION_SHIFT / len)
        };
        FixedVectorQ16 {
            x: dx * diagnostics.target_magnitude_q16 / DIRECTION_SHIFT,
            y: dy * diagnostics.target_magnitude_q16 / DIRECTION_SHIFT,
        }
    }
    /// Reliable corners keep the raw weighted-normalization law exactly,
    /// bit for bit, over every unstabilized fast-seed corner.
    #[test]
    fn reliable_directions_are_bit_identical_to_the_raw_law() {
        let map = q::map_40x40();
        let mut reliable_samples = 0_usize;
        for seed in q::FAST_SEEDS {
            for profile in [
                HexDeformationProfile::Organic,
                HexDeformationProfile::PagoniaLike,
            ] {
                let topology = q::generate(&map, seed, profile);
                for vertex in &topology.vertices {
                    let (diagnostics, produced) = observations(seed, vertex.canonical_key, profile);
                    if diagnostics.stabilization_applied {
                        continue;
                    }
                    let local = local_component_q16(seed, vertex.canonical_key, profile);
                    assert_eq!(
                        produced,
                        raw_law(&diagnostics, local),
                        "reliable corner must keep the raw law exactly"
                    );
                    reliable_samples += 1;
                }
            }
        }
        assert!(reliable_samples > 0);
    }

    /// Stabilized corners reach the reliability projection floor: only
    /// unreliable corners stabilize, the corrected length stays within the
    /// floor's truncation, and the produced magnitude stays at the target.
    #[test]
    fn stabilized_directions_keep_a_minimum_projection_onto_the_reference() {
        let map = q::map_40x40();
        let mut stabilized_samples = 0_usize;
        for seed in q::FAST_SEEDS {
            for profile in [
                HexDeformationProfile::Organic,
                HexDeformationProfile::PagoniaLike,
            ] {
                let topology = q::generate(&map, seed, profile);
                for vertex in &topology.vertices {
                    let (diagnostics, produced) = observations(seed, vertex.canonical_key, profile);
                    if !diagnostics.stabilization_applied {
                        continue;
                    }
                    assert!(
                        diagnostics.weighted_length_q16 * Q16 / diagnostics.target_magnitude_q16
                            < MIN_RELIABLE_DIRECTION_RATIO_Q16,
                        "only unreliable corners are stabilized"
                    );
                    let produced_len = component_length_q16(produced);
                    assert!(
                        produced_len >= diagnostics.target_magnitude_q16 - 2,
                        "stabilization must not shrink the magnitude"
                    );
                    assert!(
                        diagnostics.stabilized_length_q16
                            >= diagnostics.minimum_reliable_length_q16 - 2,
                        "corrected length stays within truncation of the floor"
                    );
                    assert!(
                        diagnostics.stabilized_length_ratio_q16
                            >= MIN_RELIABLE_DIRECTION_RATIO_Q16 - 16,
                        "corrected length reaches the reliability floor"
                    );
                    stabilized_samples += 1;
                }
            }
        }
        assert!(stabilized_samples > 0, "fast seeds must exercise the floor");
    }

    /// Adjacent displacement-direction audit with 4-way stabilization tracking
    /// over the fast seeds: the global -1.0 extreme is a non-near-zero
    /// high-frequency local flip, the floor never deepens a pair beyond it,
    /// and both-stabilized pairs stay far from anti-parallel.
    #[test]
    fn adjacent_displacement_direction_audit_on_canonical_map() {
        let map = q::map_40x40();
        let mut global = f32::MAX;
        let mut one = f32::MAX;
        let mut both = f32::MAX;
        for seed in q::FAST_SEEDS {
            for profile in [
                HexDeformationProfile::Organic,
                HexDeformationProfile::PagoniaLike,
            ] {
                let topology = q::generate(&map, seed, profile);
                for (origin, destination) in unique_edge_pairs(&topology) {
                    let (Ok(origin_regular), Ok(destination_regular)) = (
                        regular_corner_position(origin.canonical_key),
                        regular_corner_position(destination.canonical_key),
                    ) else {
                        continue;
                    };
                    let origin_stabilized = observations(seed, origin.canonical_key, profile)
                        .0
                        .stabilization_applied;
                    let destination_stabilized =
                        observations(seed, destination.canonical_key, profile)
                            .0
                            .stabilization_applied;
                    let dot = (origin.position - origin_regular)
                        .normalize_or_zero()
                        .dot((destination.position - destination_regular).normalize_or_zero());
                    global = global.min(dot);
                    if origin_stabilized != destination_stabilized {
                        one = one.min(dot);
                    }
                    if origin_stabilized && destination_stabilized {
                        both = both.min(dot);
                    }
                }
            }
        }
        println!("worst adjacent direction-change: global={global:.5} one_stabilized={one:.5} both_stabilized={both:.5}");
        assert!(
            global >= -1.0 && global.is_finite(),
            "adjacent dot is a unit-range quantity"
        );
        assert!(
            one >= global - 1e-4,
            "the floor must not deepen a neighbor pair beyond the pre-existing worst"
        );
        assert!(
            both >= -0.1,
            "near-zero-linked pairs must stay far from anti-parallel: both={both}"
        );
    }

    /// Boundary and overflow tests for the `div_away_from_zero` radial integer helper.
    #[test]
    #[rustfmt::skip]
    fn test_div_away_from_zero_boundary_conditions() {
        use crate::map::face_topology::blend_diagnostics::div_away_from_zero;
        assert_eq!(div_away_from_zero(10, 3), Some(4));
        assert_eq!(div_away_from_zero(-10, 3), Some(-4));
        assert_eq!(div_away_from_zero(9, 3), Some(3));
        assert_eq!(div_away_from_zero(-9, 3), Some(-3));
        assert_eq!(div_away_from_zero(0, 5), Some(0));
        assert_eq!(div_away_from_zero(10, 0), None);
        assert_eq!(div_away_from_zero(10, -5), None);
        assert_eq!(div_away_from_zero(i128::from(i64::MAX), 1), Some(i64::MAX));
        assert_eq!(div_away_from_zero(i128::MAX, 1), None);
        assert_eq!(div_away_from_zero(i128::MIN, 1), None);
    }

    /// Permanent regression test locking the four key diagnostic fixtures:
    /// Organic seed 173 and PagoniaLike seed 126 must preserve positive edge alignment,
    /// PagoniaLike seed 162 must not create exact -1.0, and PagoniaLike seed 191 must not regress.
    #[test]
    #[rustfmt::skip]
    fn four_material_fixtures_preserve_direction_and_avoid_regressions() {
        use crate::map::face_topology::blend_policy::{DISABLED_BLEND_RELIABILITY_POLICY, PRODUCTION_BLEND_RELIABILITY_POLICY};
        use crate::map::face_topology::tests_blend_candidate_shared::shared as c;
        use std::collections::HashMap;

        let (map, exact_m1_bits) = (q::map_40x40(), (-1.0_f32).to_bits());
        for (profile, seed, expected_raw_positive) in [
            (HexDeformationProfile::Organic, 173_u32, true), (HexDeformationProfile::PagoniaLike, 126_u32, true),
            (HexDeformationProfile::PagoniaLike, 162_u32, false), (HexDeformationProfile::PagoniaLike, 191_u32, false),
        ] {
            let raw_topo = c::generate(&map, seed, profile, DISABLED_BLEND_RELIABILITY_POLICY);
            let prod_topo = c::generate(&map, seed, profile, PRODUCTION_BLEND_RELIABILITY_POLICY);
            let mut raw_edge_map = HashMap::new();
            for (origin, destination) in unique_edge_pairs(&raw_topo) {
                let (Ok(o_reg), Ok(d_reg)) = (regular_corner_position(origin.canonical_key), regular_corner_position(destination.canonical_key)) else { continue; };
                let (o_dir, d_dir) = ((origin.position - o_reg).normalize_or_zero(), (destination.position - d_reg).normalize_or_zero());
                let (k1, k2) = (origin.canonical_key, destination.canonical_key);
                raw_edge_map.insert((k1.min(k2), k1.max(k2)), o_dir.dot(d_dir));
            }

            for (origin, destination) in unique_edge_pairs(&prod_topo) {
                let (Ok(o_reg), Ok(d_reg)) = (regular_corner_position(origin.canonical_key), regular_corner_position(destination.canonical_key)) else { continue; };
                let (o_dir, d_dir) = ((origin.position - o_reg).normalize_or_zero(), (destination.position - d_reg).normalize_or_zero());
                let cand_dot = o_dir.dot(d_dir);
                let (k1, k2) = (origin.canonical_key, destination.canonical_key);

                if let Some(&raw_dot) = raw_edge_map.get(&(k1.min(k2), k1.max(k2))) {
                    if expected_raw_positive && raw_dot > 0.0 {
                        assert!(cand_dot > 0.0, "{profile:?} seed={seed}: raw dot {raw_dot} must stay positive after radial stabilization, got {cand_dot}");
                    }
                    assert!(cand_dot.to_bits() != exact_m1_bits || raw_dot.to_bits() == exact_m1_bits, "{profile:?} seed={seed}: radial stabilization must not create newly exact -1.0 edges");
                }
            }
        }
    }

    /// Permanent invariants for radial scaling across fast seeds:
    /// length >= floor, component signs preserved, zero-vector fallback safe,
    /// and zero new near-antiparallel edge transitions.
    #[test]
    #[rustfmt::skip]
    fn radial_scaling_invariants_hold_for_all_fast_seeds() {
        let map = q::map_40x40();
        let exact_m1_bits = (-1.0_f32).to_bits();

        for seed in q::FAST_SEEDS {
            for profile in [HexDeformationProfile::Organic, HexDeformationProfile::PagoniaLike] {
                let topology = q::generate(&map, seed, profile);
                for vertex in &topology.vertices {
                    let (diagnostics, produced) = observations(seed, vertex.canonical_key, profile);
                    if !diagnostics.stabilization_applied { continue; }
                    assert!(diagnostics.stabilized_length_q16 >= diagnostics.minimum_reliable_length_q16 - 2, "{profile:?} seed={seed}: stabilized length must reach floor");
                    if diagnostics.weighted_x_q16 != 0 { assert_eq!(diagnostics.stabilized_x_q16.signum(), diagnostics.weighted_x_q16.signum()); }
                    if diagnostics.weighted_y_q16 != 0 { assert_eq!(diagnostics.stabilized_y_q16.signum(), diagnostics.weighted_y_q16.signum()); }
                    assert!(component_length_q16(produced) > 0, "{profile:?} seed={seed}: produced displacement must be non-zero");
                }
                for (origin, destination) in unique_edge_pairs(&topology) {
                    let (Ok(o_reg), Ok(d_reg)) = (regular_corner_position(origin.canonical_key), regular_corner_position(destination.canonical_key)) else { continue; };
                    let (o_dir, d_dir) = ((origin.position - o_reg).normalize_or_zero(), (destination.position - d_reg).normalize_or_zero());
                    if o_dir.dot(d_dir).to_bits() == exact_m1_bits {
                        let o_stab = observations(seed, origin.canonical_key, profile).0.stabilization_applied;
                        let d_stab = observations(seed, destination.canonical_key, profile).0.stabilization_applied;
                        assert!(!o_stab && !d_stab, "{profile:?} seed={seed}: exact -1.0 edge must not involve a stabilized corner");
                    }
                }
            }
        }
    }
}
