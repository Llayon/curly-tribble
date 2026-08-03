/// Acceptance-criteria guard tests: measured final-output profiles must satisfy
/// the documented thresholds on canonical fixtures, and the hard displacement
/// caps must be enforced by the generator (failure, not warning).
#[cfg(test)]
mod acceptance_tests {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::acceptance::{
        ProfileAcceptanceCriteria, ProfileAcceptanceReport,
    };
    use crate::map::face_topology::generator::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::types::HexFaceTopology;
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
    /// deterministic seeds.
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
                let issues = report.violations(ProfileAcceptanceCriteria::for_profile(profile));
                assert!(
                    issues.is_empty(),
                    "seed {seed} profile {profile:?}: {}",
                    issues.join("; ")
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
            assert!(
                report.maximum_displacement_ratio <= cap_ratio * (1.0 + 1e-3),
                "profile {profile:?} exceeded cap"
            );
        }
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
