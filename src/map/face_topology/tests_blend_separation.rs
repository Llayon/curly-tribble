//! Production-only profile separation stress: the green channel keeps
//! `Organic >= Subtle + 0.015` and `Pago >= Organic + 0.015` (the dual gap)
//! per seed on the canonical map. This is the production accept boundary; it
//! is measured on the production pipeline only, never on candidate policies.
#[cfg(test)]
mod blend_separation_tests {
    use crate::map::face_topology::acceptance::ProfileAcceptanceReport;
    use crate::map::face_topology::generator::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::WorldSeed;

    /// Documented minimum average-displacement gaps (never weakened).
    const DUAL_GAP: f32 = 0.015;

    fn separation(seed: u32) -> (f32, f32, f32) {
        let report = |profile| {
            ProfileAcceptanceReport::from_topology(
                &generate_hex_face_topology_with_profile(
                    &q::map_40x40(),
                    WorldSeed::new(seed),
                    profile,
                )
                .expect("profile"),
            )
        };
        (
            report(HexDeformationProfile::Subtle).average_displacement_ratio,
            report(HexDeformationProfile::Organic).average_displacement_ratio,
            report(HexDeformationProfile::PagoniaLike).average_displacement_ratio,
        )
    }

    /// The fast separation tier asserts the explicit dual gap per seed.
    #[test]
    fn canonical_40x40_dual_gap_satisfied_per_fast_seed() {
        for seed in q::FAST_SEEDS {
            let (subtle, organic, pago) = separation(seed);
            assert!(
                organic >= subtle + DUAL_GAP,
                "seed {seed}: organic={organic:.5} subtle={subtle:.5} gap below {DUAL_GAP}"
            );
            assert!(
                pago >= organic + DUAL_GAP,
                "seed {seed}: pago={pago:.5} organic={organic:.5} gap below {DUAL_GAP}"
            );
        }
    }

    /// Full 256-seed dual-gap sweep (ignored): the production accept boundary
    /// on the canonical map, tracking both profile gaps independently.
    #[test]
    #[ignore = "full canonical separation sweep"]
    fn full_canonical_profile_separation_stress_256_seeds() {
        let mut min_subtle_organic_gap = f32::INFINITY;
        let mut min_subtle_organic_seed = 0_u32;
        let mut min_organic_pago_gap = f32::INFINITY;
        let mut min_organic_pago_seed = 0_u32;

        let mut subtle_min_avg = f32::INFINITY;
        let mut subtle_max_avg = f32::NEG_INFINITY;
        let mut organic_min_avg = f32::INFINITY;
        let mut organic_max_avg = f32::NEG_INFINITY;
        let mut pago_min_avg = f32::INFINITY;
        let mut pago_max_avg = f32::NEG_INFINITY;

        for seed in 0..256_u32 {
            let (subtle, organic, pago) = separation(seed);
            subtle_min_avg = subtle_min_avg.min(subtle);
            subtle_max_avg = subtle_max_avg.max(subtle);
            organic_min_avg = organic_min_avg.min(organic);
            organic_max_avg = organic_max_avg.max(organic);
            pago_min_avg = pago_min_avg.min(pago);
            pago_max_avg = pago_max_avg.max(pago);

            let organic_gap = organic - subtle;
            let pago_gap = pago - organic;
            assert!(
                organic_gap >= DUAL_GAP,
                "seed {seed}: organic={organic:.5} subtle={subtle:.5} gap={organic_gap:.5}"
            );
            assert!(
                pago_gap >= DUAL_GAP,
                "seed {seed}: pago={pago:.5} organic={organic:.5} gap={pago_gap:.5}"
            );

            if organic_gap < min_subtle_organic_gap {
                min_subtle_organic_gap = organic_gap;
                min_subtle_organic_seed = seed;
            }
            if pago_gap < min_organic_pago_gap {
                min_organic_pago_gap = pago_gap;
                min_organic_pago_seed = seed;
            }
        }
        println!(
            "separation stress 256 seeds: \
             Subtle->Organic min gap {min_subtle_organic_gap:.5} at seed {min_subtle_organic_seed}, \
             Organic->PagoniaLike min gap {min_organic_pago_gap:.5} at seed {min_organic_pago_seed}; \
             averages: Subtle [{subtle_min_avg:.5}..{subtle_max_avg:.5}], \
             Organic [{organic_min_avg:.5}..{organic_max_avg:.5}], \
             PagoniaLike [{pago_min_avg:.5}..{pago_max_avg:.5}]"
        );
    }
}
