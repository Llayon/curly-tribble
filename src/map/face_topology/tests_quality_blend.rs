//! Blend-law diagnostics and synthetic near-cancellation coverage.
//!
//! The blend normalizes any non-zero weighted direction, even one extremely
//! close to zero, and only falls back when both weighted components are
//! exactly zero. These tests measure that behavior on real maps (with
//! integer/fixed-point calculations) and lock the documented law with pure
//! deterministic inputs. They do not change or endorse the behavior.
#[cfg(test)]
mod quality_blend_tests {
    use crate::map::face_topology::blend::{
        blend_to_displacement_q16, component_length_q16, weighted_blend_diagnostics,
        FixedVectorQ16, WeightedBlendDiagnostics,
    };
    use crate::map::face_topology::corner_key::regular_corner_position;
    use crate::map::face_topology::profiles::{
        interpolated_correlated_field, local_component_q16, HexDeformationProfile,
    };
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::types::SharedCornerKey;
    use crate::map::HexCoord;

    const Q16: i64 = 65_536;

    #[derive(Debug, Clone, Copy, Default)]
    struct BlendScanReport {
        min_weighted_length: i64,
        min_ratio_q16: i64,
        min_seed: u32,
        min_key: SharedCornerKey,
        count_below_64th: usize,
        count_below_32nd: usize,
        count_below_16th: usize,
        exact_zero_fallbacks: usize,
        anti_aligned_count: usize,
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
                self.min_key = key;
            }
            if ratio < self.min_ratio_q16 {
                self.min_ratio_q16 = ratio;
                self.min_seed = seed;
                self.min_key = key;
            }
            self.count_below_64th += usize::from(ratio < 1_024);
            self.count_below_32nd += usize::from(ratio < 2_048);
            self.count_below_16th += usize::from(ratio < 4_096);
            self.exact_zero_fallbacks +=
                usize::from(diagnostics.weighted_x_q16 == 0 && diagnostics.weighted_y_q16 == 0);
            self.anti_aligned_count += usize::from(diagnostics.anti_aligned);
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
                ..BlendScanReport::default()
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

    fn format_report(profile: HexDeformationProfile, report: BlendScanReport) -> String {
        format!("{profile:?} {report:?}")
    }

    /// Real-map near-zero blend audit over the canonical map and fast seeds.
    #[test]
    fn near_zero_weighted_blend_diagnostics_are_measured_for_fast_seeds() {
        for (profile, report) in scan_seeds(&q::FAST_SEEDS) {
            println!("{}", format_report(profile, report));
            assert!(report.samples > 0);
            assert!(report.count_below_64th <= report.samples);
            assert_eq!(
                report.exact_zero_fallbacks, 0,
                "no silent zero-direction fallback"
            );
        }
    }

    /// Full 256-seed real-map near-zero blend audit (ignored).
    #[test]
    #[ignore = "full near-zero blend scan"]
    fn near_zero_weighted_blend_diagnostics_are_measured_for_all_256_seeds() {
        let seeds: Vec<u32> = (0..256).collect();
        for (profile, report) in scan_seeds(&seeds) {
            println!("{}", format_report(profile, report));
        }
    }

    const ORGANIC_WC: u32 = 42_598;
    const ORGANIC_WL: u32 = 22_938;
    /// (Pago seed 194, Organic seed 64) weakest measured near-zero keys.
    const WORST_KEYS: [[(i32, i32); 3]; 2] =
        [[(6, 7), (6, 8), (7, 7)], [(14, 7), (15, 6), (15, 7)]];

    fn vector(x: i64, y: i64) -> FixedVectorQ16 {
        FixedVectorQ16 { x, y }
    }

    fn blend(correlated: FixedVectorQ16, local: FixedVectorQ16) -> FixedVectorQ16 {
        blend_to_displacement_q16(correlated, local, ORGANIC_WC, ORGANIC_WL)
    }

    /// Near-zero blend direction fixtures picked by the full 256-seed scan.
    ///
    /// The `PagoniaLike` case at seed 194 is the weakest measurement-supported
    /// direction on the canonical map: weighted length 1 (`Q16`) and magnitude
    /// 5/65536 of the target, yet the blend still normalizes it to full
    /// magnitude. These integer diagnostics are exact, so no tolerance is
    /// needed. The behavior itself is not endorsed, only locked for review.
    #[test]
    fn near_zero_blend_direction_fixtures_lock_the_weakest_measured_cases() {
        let map = q::map_40x40();
        let pago = fixture_report(&map, 194, HexDeformationProfile::PagoniaLike, WORST_KEYS[0]);
        assert_eq!(pago.weighted_length_q16, 1, "weakest weighted direction");
        assert_eq!(pago.weighted_over_target_q16, 5);
        let organic = fixture_report(&map, 64, HexDeformationProfile::Organic, WORST_KEYS[1]);
        assert_eq!(organic.weighted_length_q16, 8);
        assert_eq!(organic.weighted_over_target_q16, 59);
    }

    fn fixture_report(
        map: &crate::map::data::MapData,
        seed: u32,
        profile: HexDeformationProfile,
        coords: [(i32, i32); 3],
    ) -> WeightedBlendDiagnostics {
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
        weighted_blend_diagnostics(
            interpolated_correlated_field(seed, vertex.canonical_key, profile),
            local_component_q16(seed, vertex.canonical_key, profile),
            config.correlated_weight_q16,
            config.local_weight_q16,
        )
    }

    /// The documented blend law on selected deterministic inputs: magnitude is
    /// the stronger component length, direction is the normalized weighted sum,
    /// and exact cancellation falls back to the local component direction.
    #[test]
    fn synthetic_blend_law_is_deterministic_and_overflow_free() {
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

        let exact_cancel = blend(
            vector(i64::from(ORGANIC_WL), 0),
            vector(-(i64::from(ORGANIC_WC)), 0),
        );
        assert_eq!(
            exact_cancel,
            vector(-(i64::from(ORGANIC_WC)), 0),
            "exact cancellation must fall back to the local direction at its magnitude"
        );

        let near_cancel = blend(
            vector(i64::from(ORGANIC_WL) + 100, 0),
            vector(-(i64::from(ORGANIC_WC)), 0),
        );
        let near_diag = weighted_blend_diagnostics(
            vector(i64::from(ORGANIC_WL) + 100, 0),
            vector(-(i64::from(ORGANIC_WC)), 0),
            ORGANIC_WC,
            ORGANIC_WL,
        );
        assert_eq!(near_cancel, vector(i64::from(ORGANIC_WC), 0));
        assert!(near_diag.weighted_length_q16 > 0);
        assert!(
            near_diag.weighted_over_target_q16 < 1_024,
            "near-cancellation ratio must be below 1/64: {}",
            near_diag.weighted_over_target_q16
        );

        let both_zero = blend(zero, zero);
        assert_eq!(both_zero, zero, "both-zero input must not divide by zero");

        let local_only = blend(zero, vector(8_000, 0));
        assert_eq!(local_only, vector(8_000, 0));

        let global_zero = blend(vector(8_000, 0), zero);
        assert_eq!(global_zero, vector(8_000, 0));
    }

    /// Adjacent displacement-direction audit across unique topology edges on
    /// the canonical map: tracks the smallest normalized endpoint dot product.
    #[test]
    fn adjacent_displacement_direction_audit_on_canonical_map() {
        let map = q::map_40x40();
        let mut worst_dot = f32::MAX;
        let mut worst: Option<(u32, HexDeformationProfile, String)> = None;
        for seed in q::FAST_SEEDS {
            for profile in [
                HexDeformationProfile::Organic,
                HexDeformationProfile::PagoniaLike,
            ] {
                let topology = q::generate(&map, seed, profile);
                for (edge_index, edge) in topology.half_edges.iter().enumerate() {
                    let paired = edge.twin.is_some_and(|twin| edge_index < twin.index());
                    let border = edge.twin.is_none();
                    if !(paired || border) {
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
                    let origin_disp = origin.position - origin_regular;
                    let destination_disp = destination.position - destination_regular;
                    let dot = origin_disp
                        .normalize_or_zero()
                        .dot(destination_disp.normalize_or_zero());
                    if dot < worst_dot {
                        worst_dot = dot;
                        worst = Some((
                            seed,
                            profile,
                            format!(
                                "edge={edge_index} origin={:?} destination={:?}",
                                origin.canonical_key, destination.canonical_key
                            ),
                        ));
                    }
                }
            }
        }
        if let Some((seed, profile, edge)) = worst {
            println!(
                "worst adjacent direction-change: dot={worst_dot:.5} seed={seed} profile={profile:?} {edge}"
            );
        }
        // Measured over the fast seeds: the worst case is exactly anti-parallel
        // (dot == -1.0, seed 42 PagoniaLike edge 7744), so no tighter lower
        // bound than the unit range is measurement-supported.
        assert!(worst_dot.is_finite() && (-1.0..=1.0).contains(&worst_dot));
    }
}
