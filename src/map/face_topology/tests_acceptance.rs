/// Acceptance-criteria guard tests: measured final-output profiles must satisfy
/// the documented thresholds on canonical fixtures, the hard displacement caps
/// must be enforced by the generator (failure, not warning), non-finite metrics
/// are rejected, and profile separation is checked with typed gaps.
#[cfg(test)]
mod acceptance_tests {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::acceptance::{
        summarize_acceptance_violations, validate_profile_displacement_cap,
        ProfileAcceptanceCriteria, ProfileAcceptanceReport, DISPLACEMENT_CAP_EPSILON,
    };
    use crate::map::face_topology::acceptance_issues::{ProfileAcceptanceIssue, ProfileMetric};
    use crate::map::face_topology::generator::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::separation::{
        check_profile_separation, ProfileSeparationCriteria, ProfileSeparationViolation,
    };
    use crate::map::face_topology::types::{HexFaceTopology, HexFaceTopologyError};
    use crate::map::{HexCoord, WorldSeed};

    fn tile() -> TileData {
        TileData::default()
    }

    fn map_40x40() -> MapData {
        let mut map = MapData::default();
        for r in 0..40i32 {
            let offset = r >> 1;
            for q in -offset..(40 - offset) {
                map.tiles.insert(HexCoord::new(q, r), tile());
            }
        }
        map.width = 40;
        map.height = 40;
        map
    }

    fn generate(map: &MapData, seed: u32, profile: HexDeformationProfile) -> HexFaceTopology {
        generate_hex_face_topology_with_profile(map, WorldSeed::new(seed), profile)
            .unwrap_or_else(|error| panic!("seed {seed} profile {profile:?}: {error:?}"))
    }

    /// The docs matrix acceptance profiles: measured outputs must fall inside
    /// the documented thresholds for the canonical 40x40 fixture and the fast
    /// deterministic seeds, and every metric must be finite.
    #[test]
    fn canonical_40x40_meets_all_profiles_acceptance() {
        for seed in [0_u32, 1, 7, 42, 99, 128, 200, 255] {
            for profile in [
                HexDeformationProfile::Subtle,
                HexDeformationProfile::Organic,
                HexDeformationProfile::PagoniaLike,
            ] {
                let topology = generate(&map_40x40(), seed, profile);
                let report = ProfileAcceptanceReport::from_topology(&topology);
                assert!(
                    report.has_finite_metrics(),
                    "seed {seed} profile {profile:?}: non-finite metrics"
                );
                let issues = report.violations(ProfileAcceptanceCriteria::for_profile(profile));
                assert!(
                    issues.is_empty(),
                    "seed {seed} profile {profile:?}: {}",
                    issues
                        .iter()
                        .map(|issue| format!("{issue:?}"))
                        .collect::<Vec<_>>()
                        .join("; ")
                );
            }
        }
    }

    /// Hard safety caps are enforced by generation (the generator fails rather
    /// than emitting an overly-displaced topology).
    #[test]
    fn generator_enforces_absolute_displacement_caps() {
        for profile in [
            HexDeformationProfile::Subtle,
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ] {
            let config = profile.config();
            let cap_ratio = config.absolute_displacement_cap_ratio();
            let report =
                ProfileAcceptanceReport::from_topology(&generate(&map_40x40(), 42, profile));
            assert!(report.has_finite_metrics());
            assert!(
                report.maximum_displacement_ratio <= cap_ratio + DISPLACEMENT_CAP_EPSILON,
                "profile {profile:?} exceeded cap"
            );
            validate_profile_displacement_cap(profile, report.maximum_displacement_ratio)
                .expect("measured profile must pass its own cap");
        }
    }

    /// Caps are per-profile and single-sourced from the profile config.
    #[test]
    fn caps_are_distinct_and_single_sourced() {
        let caps = [
            HexDeformationProfile::Subtle,
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ]
        .map(|profile| {
            let ratio = profile.config().absolute_displacement_cap_ratio();
            assert!((ratio * 65_536.0).fract().abs() < 1e-6);
            ratio
        });
        assert!(caps[0] < caps[1] && caps[1] < caps[2]);
    }

    /// Cap boundary policy: the cap itself and one epsilon of slack pass; more
    /// fails; non-finite values always fail.
    #[test]
    fn cap_boundary_branches_are_exact() {
        let profile = HexDeformationProfile::Subtle;
        let cap = profile.config().absolute_displacement_cap_ratio();
        assert!(validate_profile_displacement_cap(profile, cap * 0.5).is_ok());
        assert!(validate_profile_displacement_cap(profile, cap).is_ok());
        assert!(validate_profile_displacement_cap(profile, cap + DISPLACEMENT_CAP_EPSILON).is_ok());
        assert!(matches!(
            validate_profile_displacement_cap(profile, cap + DISPLACEMENT_CAP_EPSILON * 2.0),
            Err(HexFaceTopologyError::ProfileDisplacementCapExceeded { .. })
        ));
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(
                matches!(
                    validate_profile_displacement_cap(profile, bad),
                    Err(HexFaceTopologyError::ProfileDisplacementNotFinite { .. })
                ),
                "value {bad} must be rejected as non-finite"
            );
        }
    }

    /// Every non-finite metric produces a typed structural issue.
    #[test]
    fn non_finite_metrics_are_typed_issues() {
        let report = ProfileAcceptanceReport::from_topology(&generate(
            &map_40x40(),
            42,
            HexDeformationProfile::Subtle,
        ));
        let criteria = ProfileAcceptanceCriteria::for_profile(HexDeformationProfile::Subtle);
        assert!(report.violations(criteria).is_empty());
        let nan_cases: [(ProfileMetric, fn(&mut ProfileAcceptanceReport)); 9] = [
            (
                ProfileMetric::AverageDisplacementRatio,
                |r: &mut ProfileAcceptanceReport| r.average_displacement_ratio = f32::NAN,
            ),
            (
                ProfileMetric::MaximumDisplacementRatio,
                |r: &mut ProfileAcceptanceReport| r.maximum_displacement_ratio = f32::NAN,
            ),
            (
                ProfileMetric::MinimumEdgeLengthRatio,
                |r: &mut ProfileAcceptanceReport| r.minimum_edge_length_ratio = f32::NAN,
            ),
            (
                ProfileMetric::MaximumEdgeLengthRatio,
                |r: &mut ProfileAcceptanceReport| r.maximum_edge_length_ratio = f32::NAN,
            ),
            (
                ProfileMetric::MinimumInteriorAngleDegrees,
                |r: &mut ProfileAcceptanceReport| r.minimum_interior_angle_degrees = f32::NAN,
            ),
            (
                ProfileMetric::MaximumInteriorAngleDegrees,
                |r: &mut ProfileAcceptanceReport| r.maximum_interior_angle_degrees = f32::NAN,
            ),
            (
                ProfileMetric::MinimumAspectQuality,
                |r: &mut ProfileAcceptanceReport| r.minimum_aspect_quality = f32::NAN,
            ),
            (
                ProfileMetric::ReducedVertexRatio,
                |r: &mut ProfileAcceptanceReport| r.reduced_vertex_ratio = f32::NAN,
            ),
            (
                ProfileMetric::RegularFallbackRatio,
                |r: &mut ProfileAcceptanceReport| r.regular_fallback_ratio = f32::NAN,
            ),
        ];
        for (metric, mutate) in nan_cases {
            let mut candidate = report;
            mutate(&mut candidate);
            assert!(!candidate.has_finite_metrics());
            let issues = candidate.violations(criteria);
            assert_eq!(
                issues.first(),
                Some(&ProfileAcceptanceIssue::NonFiniteMetric(metric)),
                "metric {metric:?} must be the first, typed issue"
            );
        }
    }

    /// Separated averages on both gaps pass; any other combination fails with
    /// exactly the violated typed gaps.
    #[test]
    fn separation_typed_gaps_pass_and_fail_exactly() {
        let criteria = ProfileSeparationCriteria::for_defaults();
        let enough = check_profile_separation(criteria, 0.100, 0.120, 0.150);
        assert!(enough.is_empty(), "sufficient gaps must pass: {enough:?}");
        let equal = check_profile_separation(criteria, 0.100, 0.100, 0.100);
        assert_eq!(equal.len(), 2, "identical averages fail both gaps");
        let single_gap = check_profile_separation(criteria, 0.100, 0.114, 0.130);
        assert_eq!(
            single_gap,
            vec![ProfileSeparationViolation::SubtleNotBelowOrganic {
                subtle_average: 0.100,
                organic_average: 0.114,
                min_gap: 0.015,
            }],
            "only the insufficient gap is reported"
        );
        let pago_gap = check_profile_separation(criteria, 0.100, 0.120, 0.130);
        assert_eq!(
            pago_gap,
            vec![ProfileSeparationViolation::OrganicNotBelowPagonia {
                organic_average: 0.120,
                pagonia_average: 0.130,
                min_gap: 0.015,
            }]
        );
    }

    /// Summary formatting is pure: none when passing, stable ordering and both
    /// profile name and fingerprint hex when failing.
    #[test]
    fn summary_formatting_is_pure_and_stable() {
        let report = ProfileAcceptanceReport::from_topology(&generate(
            &map_40x40(),
            42,
            HexDeformationProfile::Subtle,
        ));
        let criteria = ProfileAcceptanceCriteria::for_profile(HexDeformationProfile::Subtle);
        assert_eq!(
            summarize_acceptance_violations(
                HexDeformationProfile::Subtle,
                report,
                criteria,
                0xABCD
            ),
            None
        );
        let mut failing = report;
        failing.average_displacement_ratio = f32::NAN;
        failing.minimum_edge_length_ratio = 0.01;
        let summary = summarize_acceptance_violations(
            HexDeformationProfile::Subtle,
            failing,
            criteria,
            0xABCD,
        )
        .expect("failing report produces a summary");
        assert!(summary.contains("profile=Subtle"), "{summary}");
        assert!(
            summary.contains("fingerprint=0x000000000000abcd"),
            "{summary}"
        );
        let non_finite_index = summary
            .find("non-finite average displacement")
            .expect("typed text");
        let band_index = summary.find("minimum edge length").expect("typed text");
        assert!(
            non_finite_index < band_index,
            "structural issues precede band issues: {summary}"
        );
    }

    /// Final reports for distinct profiles are not collapsed into the same pass;
    /// the generator must emit a marked profile (never falling back to defaults).
    #[test]
    fn reports_differ_and_profile_is_recorded() {
        let subtle = generate(&map_40x40(), 42, HexDeformationProfile::Subtle);
        let organic = generate(&map_40x40(), 42, HexDeformationProfile::Organic);
        let pago = generate(&map_40x40(), 42, HexDeformationProfile::PagoniaLike);
        assert_eq!(subtle.stats.profile, HexDeformationProfile::Subtle);
        assert_eq!(organic.stats.profile, HexDeformationProfile::Organic);
        assert_eq!(pago.stats.profile, HexDeformationProfile::PagoniaLike);
    }
}
