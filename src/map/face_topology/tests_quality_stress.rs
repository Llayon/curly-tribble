//! Full-matrix quality scans for the tuned deformation profiles.
//!
//! The permanent zero-backoff stress tier lives in `tests_profiles.rs` (it
//! keeps the documented test name). This file hosts the measurement scans for
//! the 4,608-topology matrix: global quality extrema per profile, the audit of
//! the previously relaxed `Organic`/`PagoniaLike` acceptance limits, and the
//! canonical 40x40 extrema used to pick final thresholds.
#[cfg(test)]
mod quality_stress_tests {
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::tests_quality_shared::shared::CaseQuality;

    #[derive(Debug, Clone, Copy)]
    enum Track {
        Min,
        Max,
    }

    /// One measured worst case with its full reproduction context.
    #[derive(Debug, Clone, Copy)]
    struct Worst {
        value: f32,
        shape: &'static str,
        seed: u32,
        profile: HexDeformationProfile,
    }

    impl Worst {
        fn format(self) -> String {
            format!(
                "{:.5} (shape={} seed={} profile={:?})",
                self.value, self.shape, self.seed, self.profile
            )
        }
    }

    /// Per-profile global extrema across a matrix of cases.
    #[derive(Debug, Default, Clone, Copy)]
    struct QualityScan {
        min_edge: Option<Worst>,
        min_angle: Option<Worst>,
        max_angle: Option<Worst>,
        min_aspect: Option<Worst>,
        max_displacement_ratio: Option<Worst>,
        max_reduction_ratio: Option<Worst>,
        max_fallback_ratio: Option<Worst>,
    }

    impl QualityScan {
        fn record(
            &mut self,
            shape: &'static str,
            seed: u32,
            profile: HexDeformationProfile,
            quality: CaseQuality,
        ) {
            Self::record_value(
                &mut self.min_edge,
                shape,
                seed,
                profile,
                quality.minimum_edge_length_ratio,
                Track::Min,
            );
            Self::record_value(
                &mut self.min_angle,
                shape,
                seed,
                profile,
                quality.minimum_interior_angle_degrees,
                Track::Min,
            );
            Self::record_value(
                &mut self.max_angle,
                shape,
                seed,
                profile,
                quality.maximum_interior_angle_degrees,
                Track::Max,
            );
            Self::record_value(
                &mut self.min_aspect,
                shape,
                seed,
                profile,
                quality.minimum_aspect_quality,
                Track::Min,
            );
            Self::record_value(
                &mut self.max_displacement_ratio,
                shape,
                seed,
                profile,
                quality.maximum_displacement_ratio,
                Track::Max,
            );
        }

        fn record_reduction(
            &mut self,
            shape: &'static str,
            seed: u32,
            profile: HexDeformationProfile,
            reduced: f32,
            fallback: f32,
        ) {
            Self::record_value(
                &mut self.max_reduction_ratio,
                shape,
                seed,
                profile,
                reduced,
                Track::Max,
            );
            Self::record_value(
                &mut self.max_fallback_ratio,
                shape,
                seed,
                profile,
                fallback,
                Track::Max,
            );
        }

        fn record_value(
            slot: &mut Option<Worst>,
            shape: &'static str,
            seed: u32,
            profile: HexDeformationProfile,
            value: f32,
            track: Track,
        ) {
            let candidate = Worst {
                value,
                shape,
                seed,
                profile,
            };
            let keeps = match (track, &*slot) {
                (Track::Min, Some(current)) => value < current.value,
                (Track::Max, Some(current)) => value > current.value,
                (_, None) => true,
            };
            if keeps {
                *slot = Some(candidate);
            }
        }

        fn format(self) -> String {
            let fmt = |slot: Option<Worst>| slot.map_or("n/a".to_string(), Worst::format);
            format!(
                "min_edge={} min_angle={} max_angle={} min_aspect={} max_displacement={} \
                 max_reduction={} max_fallback={}",
                fmt(self.min_edge),
                fmt(self.min_angle),
                fmt(self.max_angle),
                fmt(self.min_aspect),
                fmt(self.max_displacement_ratio),
                fmt(self.max_reduction_ratio),
                fmt(self.max_fallback_ratio)
            )
        }
    }

    fn scan_matrix(seeds: &[u32]) -> [QualityScan; 3] {
        let mut scans = [QualityScan::default(); 3];
        for (shape, map) in q::all_shapes() {
            for &seed in seeds {
                for profile in q::all_profiles() {
                    let quality = q::measured_quality(&map, seed, profile);
                    let topology = q::generate(&map, seed, profile);
                    let index = profile_index(profile);
                    scans[index].record(shape, seed, profile, quality);
                    scans[index].record_reduction(
                        shape,
                        seed,
                        profile,
                        ratio(topology.stats.reduced_vertices, topology.vertices.len()),
                        ratio(
                            topology.stats.regular_position_fallbacks,
                            topology.vertices.len(),
                        ),
                    );
                }
            }
        }
        scans
    }

    fn profile_index(profile: HexDeformationProfile) -> usize {
        match profile {
            HexDeformationProfile::Subtle => 0,
            HexDeformationProfile::Organic => 1,
            HexDeformationProfile::PagoniaLike => 2,
        }
    }

    fn ratio(count: usize, total: usize) -> f32 {
        if total == 0 {
            0.0
        } else {
            count as f32 / total as f32
        }
    }

    fn print_scan(label: &str, scans: &[QualityScan; 3]) {
        for profile in q::all_profiles() {
            println!(
                "{label} {profile:?}: {}",
                scans[profile_index(profile)].format()
            );
        }
    }

    /// Full 4,608-topology global quality extrema (one-off, ignored).
    #[test]
    #[ignore = "full quality extrema scan"]
    fn full_4608_quality_extrema_scan() {
        let seeds: Vec<u32> = (0..256).collect();
        print_scan("all_shapes_256_seeds", &scan_matrix(&seeds));
    }

    /// Canonical 40x40 quality extrema across all 256 seeds (ignored).
    #[test]
    #[ignore = "canonical quality extrema scan"]
    fn canonical_40x40_quality_extrema_scan_256_seeds() {
        let map = q::map_40x40();
        let mut scans = [QualityScan::default(); 3];
        for seed in 0..256_u32 {
            for profile in q::all_profiles() {
                let quality = q::measured_quality(&map, seed, profile);
                let index = profile_index(profile);
                scans[index].record("40x40", seed, profile, quality);
                let topology = q::generate(&map, seed, profile);
                scans[index].record_reduction(
                    "40x40",
                    seed,
                    profile,
                    ratio(topology.stats.reduced_vertices, topology.vertices.len()),
                    ratio(
                        topology.stats.regular_position_fallbacks,
                        topology.vertices.len(),
                    ),
                );
            }
        }
        print_scan("40x40_256_seeds", &scans);
    }

    /// Audits the previously relaxed acceptance limits against the measured
    /// 4,608 matrix (ignored). Old `Organic`: max angle 155 / aspect 0.500.
    /// Old `PagoniaLike`: min angle 80 / max angle 155 / aspect 0.500.
    #[test]
    #[ignore = "old threshold audit"]
    fn old_organic_and_pagonia_threshold_audit() {
        let mut totals = [(0_usize, 0_usize); 2];
        let mut worst = [None; 2];
        for (shape, map) in q::all_shapes() {
            for seed in 0..256_u32 {
                let organic = q::measured_quality(&map, seed, HexDeformationProfile::Organic);
                let pago = q::measured_quality(&map, seed, HexDeformationProfile::PagoniaLike);
                let hits = [
                    organic.maximum_interior_angle_degrees > 155.0
                        || organic.minimum_aspect_quality < 0.500,
                    pago.minimum_interior_angle_degrees < 80.0
                        || pago.maximum_interior_angle_degrees > 155.0
                        || pago.minimum_aspect_quality < 0.500,
                ];
                for (index, quality) in [organic, pago].into_iter().enumerate() {
                    if !hits[index] {
                        continue;
                    }
                    totals[index].0 += 1;
                    if shape == "40x40" {
                        totals[index].1 += 1;
                    }
                    let score = quality.minimum_aspect_quality;
                    if worst[index].is_none_or(|entry: (f32, f32, &str, u32)| score < entry.0) {
                        worst[index] =
                            Some((score, quality.maximum_interior_angle_degrees, shape, seed));
                    }
                }
            }
        }
        println!(
            "old Organic limits: {} failures (40x40: {}) worst={:?}",
            totals[0].0, totals[0].1, worst[0]
        );
        println!(
            "old Pago limits: {} failures (40x40: {}) worst={:?}",
            totals[1].0, totals[1].1, worst[1]
        );
    }
}
