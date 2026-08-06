//! Blend-law diagnostics and synthetic near-cancellation coverage.
//!
//! Measures the weighted-blend law on real maps and locks the near-zero
//! fixtures picked by the full 256-seed scan. Stabilized-direction invariants
//! live in `tests_quality_blend_direction.rs`.
#[cfg(test)]
mod quality_blend_tests {
    use crate::map::data::MapData;
    use crate::map::face_topology::blend::{
        blend_reference, blend_to_displacement_q16, component_length_q16,
        weighted_blend_diagnostics, BlendReference, FixedVectorQ16, WeightedBlendDiagnostics,
        MIN_RELIABLE_DIRECTION_RATIO_Q16,
    };
    use crate::map::face_topology::profiles::{
        interpolated_correlated_field, local_component_q16, HexDeformationProfile,
    };
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::types::SharedCornerKey;
    use crate::map::HexCoord;
    const ORGANIC_WC: u32 = 42_598;
    const ORGANIC_WL: u32 = 22_938;
    #[derive(Debug, Clone, Copy)]
    struct BlendScanReport {
        min_weighted_length: i64,
        min_ratio_q16: i64,
        min_seed: u32,
        min_key: Option<SharedCornerKey>,
        count_below_64th: usize,
        count_below_32nd: usize,
        count_below_16th: usize,
        weighted_sum_zero_fallbacks: usize,
        anti_aligned_count: usize,
        stabilized_count: usize,
        samples: usize,
    }
    impl BlendScanReport {
        fn record(&mut self, seed: u32, key: SharedCornerKey, profile: HexDeformationProfile) {
            let config = profile.config();
            let diagnostics = weighted_blend_diagnostics(
                interpolated_correlated_field(seed, key, profile),
                local_component_q16(seed, key, profile),
                config.correlated_weight_q16,
                config.local_weight_q16,
            );
            let ratio = diagnostics.weighted_over_target_q16;
            if diagnostics.weighted_length_q16 < self.min_weighted_length {
                self.min_weighted_length = diagnostics.weighted_length_q16;
                self.min_seed = seed;
                self.min_key = Some(key);
            }
            if ratio < self.min_ratio_q16 {
                self.min_ratio_q16 = ratio;
                self.min_seed = seed;
                self.min_key = Some(key);
            }
            self.count_below_64th += usize::from(ratio < 1_024);
            self.count_below_32nd += usize::from(ratio < 2_048);
            self.count_below_16th += usize::from(ratio < 4_096);
            self.weighted_sum_zero_fallbacks +=
                usize::from(diagnostics.weighted_x_q16 == 0 && diagnostics.weighted_y_q16 == 0);
            self.anti_aligned_count += usize::from(diagnostics.anti_aligned);
            self.stabilized_count += usize::from(diagnostics.stabilization_applied);
            self.samples += 1;
        }
    }

    fn scan_seeds(seeds: &[u32]) -> Vec<(HexDeformationProfile, BlendScanReport)> {
        let map = q::map_40x40();
        [
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ]
        .into_iter()
        .map(|profile| {
            let mut report = BlendScanReport {
                min_weighted_length: i64::MAX,
                min_ratio_q16: i64::MAX,
                min_seed: u32::MAX,
                min_key: None,
                count_below_64th: 0,
                count_below_32nd: 0,
                count_below_16th: 0,
                weighted_sum_zero_fallbacks: 0,
                anti_aligned_count: 0,
                stabilized_count: 0,
                samples: 0,
            };
            for &seed in seeds {
                let topology = q::generate(&map, seed, profile);
                for vertex in &topology.vertices {
                    report.record(seed, vertex.canonical_key, profile);
                }
            }
            (profile, report)
        })
        .collect()
    }

    /// Real-map near-zero blend audit over the canonical map and fast seeds.
    #[test]
    fn near_zero_weighted_blend_diagnostics_are_measured_for_fast_seeds() {
        for (profile, report) in scan_seeds(&q::FAST_SEEDS) {
            println!("{profile:?} {report:?}");
            assert!(report.samples > 0);
            assert!(report.stabilized_count <= report.samples);
            assert_eq!(
                report.weighted_sum_zero_fallbacks, 0,
                "no silent zero-direction fallback"
            );
        }
    }

    /// (Pago seed 194, Organic seed 64) weakest measured near-zero keys.
    const WORST_KEYS: [[(i32, i32); 3]; 2] =
        [[(6, 7), (6, 8), (7, 7)], [(14, 7), (15, 6), (15, 7)]];

    fn vector(x: i64, y: i64) -> FixedVectorQ16 {
        FixedVectorQ16 { x, y }
    }

    fn blend(correlated: FixedVectorQ16, local: FixedVectorQ16) -> FixedVectorQ16 {
        blend_to_displacement_q16(correlated, local, ORGANIC_WC, ORGANIC_WL)
    }
    /// Real correlations, diagnostics, and produced displacement of a key.
    fn fixture(
        map: &MapData,
        seed: u32,
        profile: HexDeformationProfile,
        coords: [(i32, i32); 3],
    ) -> (WeightedBlendDiagnostics, FixedVectorQ16) {
        let expected = SharedCornerKey::new(
            HexCoord::new(coords[0].0, coords[0].1),
            HexCoord::new(coords[1].0, coords[1].1),
            HexCoord::new(coords[2].0, coords[2].1),
        );
        let config = profile.config();
        let topology = q::generate(map, seed, profile);
        let vertex = topology
            .vertices
            .iter()
            .find(|vertex| vertex.canonical_key == expected)
            .unwrap_or_else(|| panic!("key {expected:?} not present for seed {seed}"));
        let correlated = interpolated_correlated_field(seed, vertex.canonical_key, profile);
        let local = local_component_q16(seed, vertex.canonical_key, profile);
        let wc = config.correlated_weight_q16;
        let wl = config.local_weight_q16;
        (
            weighted_blend_diagnostics(correlated, local, wc, wl),
            blend_to_displacement_q16(correlated, local, wc, wl),
        )
    }

    /// Near-zero blend direction fixtures picked by the full 256-seed scan.
    ///
    /// The `PagoniaLike` case at seed 194 is the weakest measured direction on
    /// the canonical map: weighted length 1 (`Q16`) and magnitude 5/65536 of
    /// the target, yet resolved to full magnitude. Raw values must never
    /// change; the stabilized resolution is locked (exact integers).
    #[test]
    fn near_zero_blend_direction_fixtures_lock_the_weakest_measured_cases() {
        let map = q::map_40x40();
        let (pago, pago_disp) =
            fixture(&map, 194, HexDeformationProfile::PagoniaLike, WORST_KEYS[0]);
        assert_eq!(pago.weighted_length_q16, 1, "weakest weighted direction");
        assert_eq!(pago.weighted_over_target_q16, 5);
        assert!(pago.stabilization_applied);
        assert_eq!(pago.reference, BlendReference::Correlated);
        assert_eq!(pago.raw_projection_q16, 0);
        assert_eq!(pago.minimum_projection_q16, 197);
        assert_eq!(pago.correction_q16, 277);
        assert_eq!(pago.stabilized_x_q16, -197);
        assert_eq!(pago.stabilized_y_q16, -197);
        assert_eq!(pago.stabilized_length_q16, 278);
        assert_eq!(pago.stabilized_length_ratio_q16, 1_443);
        assert!(!pago.components_are_zero);
        assert_eq!(pago_disp.x, -8_945);
        assert_eq!(pago_disp.y, -8_945);

        let (organic, organic_disp) =
            fixture(&map, 64, HexDeformationProfile::Organic, WORST_KEYS[1]);
        assert_eq!(organic.weighted_length_q16, 8);
        assert_eq!(organic.weighted_over_target_q16, 59);
        assert!(organic.stabilization_applied);
        assert_eq!(organic.reference, BlendReference::Correlated);
        assert_eq!(organic.raw_projection_q16, 7);
        assert_eq!(organic.minimum_projection_q16, 138);
        assert_eq!(organic.correction_q16, 139);
        assert_eq!(organic.stabilized_x_q16, 138);
        assert_eq!(organic.stabilized_y_q16, 52);
        assert_eq!(organic.stabilized_length_q16, 147);
        assert_eq!(organic.stabilized_length_ratio_q16, 1_085);
        assert!(!organic.components_are_zero);
        assert_eq!(organic_disp.x, 8_335);
        assert_eq!(organic_disp.y, 3_140);
    }

    /// The documented blend law on selected deterministic inputs: magnitude is
    /// the stronger component length, direction is the normalized weighted
    /// sum, near-cancellation is corrected onto the reference direction, and
    /// exact cancellation without a correction target falls back to the local.
    #[test]
    fn synthetic_blend_law_is_deterministic_and_overflow_free() {
        let wc = i64::from(ORGANIC_WC);
        let wl = i64::from(ORGANIC_WL);
        let zero = vector(0, 0);
        let aligned = blend(vector(8_000, 0), vector(8_000, 0));
        assert_eq!(aligned, vector(8_000, 0));

        let anti_unequal = blend(vector(8_000, 0), vector(-4_000, 0));
        assert_eq!(
            anti_unequal,
            vector(8_000, 0),
            "anti-parallel cannot cancel"
        );

        let orthogonal = blend(vector(8_000, 0), vector(0, 8_000));
        let orthogonal_length = component_length_q16(orthogonal);
        assert!(
            (orthogonal_length - 8_000).abs() <= 2,
            "orthogonal magnitude must stay near the target, got {orthogonal_length}"
        );

        let aligned_diag =
            weighted_blend_diagnostics(vector(8_000, 0), vector(8_000, 0), ORGANIC_WC, ORGANIC_WL);
        assert!(
            !aligned_diag.stabilization_applied,
            "strong aligned keeps the raw direction"
        );

        let exact_cancel = blend(vector(wl, 0), vector(-wc, 0));
        assert_eq!(
            exact_cancel,
            vector(wc, 0),
            "exact cancel resolves onto the reference"
        );

        let near_correlated = vector(wl + 100, 0);
        let near_local = vector(-wc, 0);
        let near_cancel = blend(near_correlated, near_local);
        let near_diag =
            weighted_blend_diagnostics(near_correlated, near_local, ORGANIC_WC, ORGANIC_WL);
        assert_eq!(near_cancel, vector(wc, 0));
        assert!(near_diag.weighted_length_q16 > 0);
        assert!(
            near_diag.weighted_over_target_q16 < 1_024,
            "near-cancellation ratio must be below 1/64: {}",
            near_diag.weighted_over_target_q16
        );
        assert!(near_diag.stabilization_applied, "near-cancel is corrected");
        assert_eq!(near_diag.reference, BlendReference::Correlated);
        assert_eq!(near_diag.raw_projection_q16, 64);
        assert_eq!(near_diag.correction_q16, 601);
        assert_eq!(near_diag.minimum_projection_q16, 665);
        assert_eq!(near_diag.stabilized_x_q16, 665);
        assert_eq!(near_diag.stabilized_y_q16, 0);
        assert_eq!(near_diag.stabilized_length_q16, 665);
        assert!(
            near_diag.stabilized_length_ratio_q16 >= MIN_RELIABLE_DIRECTION_RATIO_Q16 - 1,
            "stabilized length reaches the reliability floor"
        );

        let both_zero = blend(zero, zero);
        assert_eq!(both_zero, zero, "both-zero input must not divide by zero");

        let local_only = blend(zero, vector(8_000, 0));
        assert_eq!(local_only, vector(8_000, 0));

        let global_zero = blend(vector(8_000, 0), zero);
        assert_eq!(global_zero, vector(8_000, 0));
    }
    /// Pure reference-selection policy for the reliability floor.
    #[test]
    fn blend_reference_policy_is_deterministic() {
        for (cx, cy, lx, ly, reference) in [
            (12_000, 0, 8_000, 0, BlendReference::Correlated),
            (8_000, 0, 12_000, 0, BlendReference::Correlated),
            (10_000, 0, 10_100, 0, BlendReference::Correlated),
            (9_000, 0, -9_000, 0, BlendReference::Correlated),
            (0, 0, 8_000, 0, BlendReference::Local),
            (8_000, 0, 0, 0, BlendReference::Correlated),
            (0, 0, 0, 0, BlendReference::FixedPositiveX),
            (-12_000, 0, 4_000, 0, BlendReference::Correlated),
        ] {
            assert_eq!(
                blend_reference(vector(cx, cy), vector(lx, ly), ORGANIC_WC, ORGANIC_WL),
                reference
            );
        }
        assert_eq!(
            blend_reference(vector(30_000, 0), vector(20_000, 0), 22_938, 65_536),
            BlendReference::Local,
            "weights overturn a raw-magnitude lead"
        );
        assert_eq!(
            blend_reference(vector(20_000, 0), vector(30_000, 0), 65_536, 22_938),
            BlendReference::Correlated,
            "weights confirm a raw-magnitude lead"
        );
    }
}
