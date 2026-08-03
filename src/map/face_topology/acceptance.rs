//! Central profile acceptance criteria and measured-output reporting.
//!
//! Thresholds are centralized here, never scattered through tests or runtime.
//! Generation-config inputs live in `profiles`; typed output issues in
//! `acceptance_issues`; the separation contract in `separation`.
use crate::map::data::HEX_SIZE;
use crate::map::face_topology::acceptance_issues::{
    format_issue, ProfileAcceptanceIssue, ProfileMetric,
};
use crate::map::face_topology::metrics::compute_topology_metrics;
use crate::map::face_topology::profiles::HexDeformationProfile;
use crate::map::face_topology::types::{HexFaceTopology, HexFaceTopologyError};

/// Small floating-point tolerance allowed above the profile displacement cap.
pub const DISPLACEMENT_CAP_EPSILON: f32 = 1e-3;

/// Thresholds the measured final output must satisfy. The absolute displacement
/// cap is **not** here: it lives once in the profile config and is enforced by
/// [`validate_profile_displacement_cap`]. These are visual targets: a miss warns
/// and fails canonical-fixture tests, but never affects production terrain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileAcceptanceCriteria {
    pub average_displacement_min_ratio: f32,
    pub average_displacement_max_ratio: f32,
    pub minimum_edge_length_ratio: f32,
    pub minimum_interior_angle_degrees: f32,
    pub maximum_interior_angle_degrees: f32,
    pub minimum_aspect_quality: f32,
    pub maximum_reduced_vertex_ratio: f32,
    pub maximum_regular_fallback_ratio: f32,
}

/// Broad, non-brittle measured values chosen from the profile matrix
/// measurement pass (all six shapes x the eight fast deterministic seeds).
impl ProfileAcceptanceCriteria {
    #[must_use]
    pub const fn for_profile(profile: HexDeformationProfile) -> Self {
        match profile {
            HexDeformationProfile::Subtle => Self {
                average_displacement_min_ratio: 0.070,
                average_displacement_max_ratio: 0.120,
                minimum_edge_length_ratio: 0.550,
                minimum_interior_angle_degrees: 80.0,
                maximum_interior_angle_degrees: 155.0,
                minimum_aspect_quality: 0.550,
                maximum_reduced_vertex_ratio: 0.150,
                maximum_regular_fallback_ratio: 0.150,
            },
            HexDeformationProfile::Organic => Self {
                average_displacement_min_ratio: 0.110,
                average_displacement_max_ratio: 0.175,
                minimum_edge_length_ratio: 0.500,
                minimum_interior_angle_degrees: 80.0,
                maximum_interior_angle_degrees: 165.0,
                minimum_aspect_quality: 0.470,
                maximum_reduced_vertex_ratio: 0.150,
                maximum_regular_fallback_ratio: 0.150,
            },
            HexDeformationProfile::PagoniaLike => Self {
                average_displacement_min_ratio: 0.150,
                average_displacement_max_ratio: 0.235,
                minimum_edge_length_ratio: 0.500,
                minimum_interior_angle_degrees: 75.0,
                maximum_interior_angle_degrees: 175.0,
                minimum_aspect_quality: 0.380,
                maximum_reduced_vertex_ratio: 0.200,
                maximum_regular_fallback_ratio: 0.150,
            },
        }
    }
}

/// Measured final-output statistics for one generated topology.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileAcceptanceReport {
    pub average_displacement_ratio: f32,
    pub maximum_displacement_ratio: f32,
    pub minimum_edge_length_ratio: f32,
    pub maximum_edge_length_ratio: f32,
    pub minimum_interior_angle_degrees: f32,
    pub maximum_interior_angle_degrees: f32,
    pub minimum_aspect_quality: f32,
    pub reduced_vertex_ratio: f32,
    pub regular_fallback_ratio: f32,
}

impl ProfileAcceptanceReport {
    /// Builds the report from a generated topology. Ratios use `HEX_SIZE`.
    #[must_use]
    pub fn from_topology(topology: &HexFaceTopology) -> Self {
        let metrics = compute_topology_metrics(topology);
        let vertex_count = topology.vertices.len();
        Self {
            average_displacement_ratio: metrics.average_displacement / HEX_SIZE,
            maximum_displacement_ratio: metrics.max_displacement / HEX_SIZE,
            minimum_edge_length_ratio: metrics.min_edge_length / HEX_SIZE,
            maximum_edge_length_ratio: metrics.max_edge_length / HEX_SIZE,
            minimum_interior_angle_degrees: metrics.min_interior_angle.to_degrees(),
            maximum_interior_angle_degrees: metrics.max_interior_angle.to_degrees(),
            minimum_aspect_quality: metrics.min_aspect_quality,
            reduced_vertex_ratio: size_ratio(topology.stats.reduced_vertices, vertex_count),
            regular_fallback_ratio: size_ratio(
                topology.stats.regular_position_fallbacks,
                vertex_count,
            ),
        }
    }

    /// True when every measured statistic is finite.
    #[must_use]
    pub fn has_finite_metrics(self) -> bool {
        self.average_displacement_ratio.is_finite()
            && self.maximum_displacement_ratio.is_finite()
            && self.minimum_edge_length_ratio.is_finite()
            && self.maximum_edge_length_ratio.is_finite()
            && self.minimum_interior_angle_degrees.is_finite()
            && self.maximum_interior_angle_degrees.is_finite()
            && self.minimum_aspect_quality.is_finite()
            && self.reduced_vertex_ratio.is_finite()
            && self.regular_fallback_ratio.is_finite()
    }

    /// Lists every criterion that the measured report misses. Non-finite
    /// metrics are always listed first.
    #[must_use]
    pub fn violations(self, criteria: ProfileAcceptanceCriteria) -> Vec<ProfileAcceptanceIssue> {
        let mut issues = Vec::new();
        for (metric, value) in [
            (
                ProfileMetric::AverageDisplacementRatio,
                self.average_displacement_ratio,
            ),
            (
                ProfileMetric::MaximumDisplacementRatio,
                self.maximum_displacement_ratio,
            ),
            (
                ProfileMetric::MinimumEdgeLengthRatio,
                self.minimum_edge_length_ratio,
            ),
            (
                ProfileMetric::MaximumEdgeLengthRatio,
                self.maximum_edge_length_ratio,
            ),
            (
                ProfileMetric::MinimumInteriorAngleDegrees,
                self.minimum_interior_angle_degrees,
            ),
            (
                ProfileMetric::MaximumInteriorAngleDegrees,
                self.maximum_interior_angle_degrees,
            ),
            (
                ProfileMetric::MinimumAspectQuality,
                self.minimum_aspect_quality,
            ),
            (ProfileMetric::ReducedVertexRatio, self.reduced_vertex_ratio),
            (
                ProfileMetric::RegularFallbackRatio,
                self.regular_fallback_ratio,
            ),
        ] {
            if !value.is_finite() {
                issues.push(ProfileAcceptanceIssue::NonFiniteMetric(metric));
            }
        }
        if self.average_displacement_ratio < criteria.average_displacement_min_ratio
            || self.average_displacement_ratio > criteria.average_displacement_max_ratio
        {
            issues.push(ProfileAcceptanceIssue::AverageDisplacementOutside {
                measured: self.average_displacement_ratio,
                min: criteria.average_displacement_min_ratio,
                max: criteria.average_displacement_max_ratio,
            });
        }
        if self.minimum_edge_length_ratio < criteria.minimum_edge_length_ratio {
            issues.push(ProfileAcceptanceIssue::MinimumEdgeLengthBelow {
                measured: self.minimum_edge_length_ratio,
                min: criteria.minimum_edge_length_ratio,
            });
        }
        if self.minimum_interior_angle_degrees < criteria.minimum_interior_angle_degrees
            || self.maximum_interior_angle_degrees > criteria.maximum_interior_angle_degrees
        {
            issues.push(ProfileAcceptanceIssue::InteriorAngleOutside {
                measured_min: self.minimum_interior_angle_degrees,
                measured_max: self.maximum_interior_angle_degrees,
                min: criteria.minimum_interior_angle_degrees,
                max: criteria.maximum_interior_angle_degrees,
            });
        }
        if self.minimum_aspect_quality < criteria.minimum_aspect_quality {
            issues.push(ProfileAcceptanceIssue::MinimumAspectBelow {
                measured: self.minimum_aspect_quality,
                min: criteria.minimum_aspect_quality,
            });
        }
        if self.reduced_vertex_ratio > criteria.maximum_reduced_vertex_ratio {
            issues.push(ProfileAcceptanceIssue::ReducedVertexRatioAbove {
                measured: self.reduced_vertex_ratio,
                max: criteria.maximum_reduced_vertex_ratio,
            });
        }
        if self.regular_fallback_ratio > criteria.maximum_regular_fallback_ratio {
            issues.push(ProfileAcceptanceIssue::RegularFallbackRatioAbove {
                measured: self.regular_fallback_ratio,
                max: criteria.maximum_regular_fallback_ratio,
            });
        }
        issues
    }
}

/// Enforces the profile's absolute displacement cap on a measured ratio.
///
/// The cap itself and one `DISPLACEMENT_CAP_EPSILON` of slack are accepted;
/// anything above is a generation failure; non-finite values are rejected.
/// This is the single cap authority consumed by the generator.
///
/// # Errors
/// `ProfileDisplacementNotFinite` for NaN/infinity; `ProfileDisplacementCapExceeded`
/// when the ratio exceeds the cap plus epsilon.
pub fn validate_profile_displacement_cap(
    profile: HexDeformationProfile,
    measured_maximum_displacement_ratio: f32,
) -> Result<(), HexFaceTopologyError> {
    if !measured_maximum_displacement_ratio.is_finite() {
        return Err(HexFaceTopologyError::ProfileDisplacementNotFinite {
            max_displacement_ratio: measured_maximum_displacement_ratio,
        });
    }
    let cap_ratio = profile.config().absolute_displacement_cap_ratio();
    if measured_maximum_displacement_ratio > cap_ratio + DISPLACEMENT_CAP_EPSILON {
        return Err(HexFaceTopologyError::ProfileDisplacementCapExceeded {
            profile,
            max_displacement: measured_maximum_displacement_ratio * HEX_SIZE,
            cap_radius: cap_ratio * HEX_SIZE,
        });
    }
    Ok(())
}

/// Builds the single warning string for a report, or `None` when it passes.
///
/// Pure formatting: issue ordering is stable (non-finite metrics first, then the
/// fixed band order) and the text is reproducible, so tests need no subscriber.
#[must_use]
pub fn summarize_acceptance_violations(
    profile: HexDeformationProfile,
    report: ProfileAcceptanceReport,
    criteria: ProfileAcceptanceCriteria,
    geometry_fingerprint: u64,
) -> Option<String> {
    let issues = report.violations(criteria);
    if issues.is_empty() {
        return None;
    }
    let mut text = format!(
        "profile={} fingerprint={geometry_fingerprint:#018x} acceptance misses:",
        profile.name()
    );
    for issue in issues {
        text.push(' ');
        text.push_str(&format_issue(&issue));
    }
    Some(text)
}

#[must_use]
fn size_ratio(count: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        count as f32 / total as f32
    }
}

/// Emits a single combined acceptance warning per (re)generation. Called only
/// from the regeneration path, so it never repeats per frame.
pub fn warn_on_acceptance_misses(
    topology: &HexFaceTopology,
    profile: HexDeformationProfile,
    geometry_fingerprint: u64,
) {
    let report = ProfileAcceptanceReport::from_topology(topology);
    if let Some(summary) = summarize_acceptance_violations(
        profile,
        report,
        ProfileAcceptanceCriteria::for_profile(profile),
        geometry_fingerprint,
    ) {
        bevy::log::tracing::event!(
            bevy::log::tracing::Level::WARN,
            profile = profile.name(),
            "{}",
            summary
        );
    }
}
