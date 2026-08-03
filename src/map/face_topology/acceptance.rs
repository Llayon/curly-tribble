//! Central profile acceptance criteria and measured-output reporting.
//!
//! Criteria are deliberately centralized here so thresholds are never scattered
//! through tests or runtime systems. Two contract groups are kept distinct:
//!
//! - **Generation configuration** (in [`crate::map::face_topology::profiles`]):
//!   component magnitude ranges, weights, macro scale, and the absolute
//!   displacement cap. These are *inputs*.
//! - **Measured final-output acceptance** (this module): observed statistics on
//!   a generated topology. These are *outputs*, never guarantees.
use crate::map::data::HEX_SIZE;
use crate::map::face_topology::metrics::compute_topology_metrics;
use crate::map::face_topology::profiles::HexDeformationProfile;
use crate::map::face_topology::types::HexFaceTopology;

/// Thresholds that the measured final output must satisfy.
///
/// `maximum_displacement_ratio` is a hard safety cap enforced by the generator
/// (a final vertex displacement exceeding the profile cap is a generation
/// failure). The remaining fields are visual acceptance targets: a missed
/// target is reported as a warning and fails canonical-fixture tests, but never
/// affects production terrain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileAcceptanceCriteria {
    pub average_displacement_min_ratio: f32,
    pub average_displacement_max_ratio: f32,
    /// Absolute final displacement cap as a ratio of the hex radius.
    pub maximum_displacement_ratio: f32,
    pub minimum_edge_length_ratio: f32,
    pub minimum_interior_angle_degrees: f32,
    pub maximum_interior_angle_degrees: f32,
    pub minimum_aspect_quality: f32,
    pub maximum_reduced_vertex_ratio: f32,
    pub maximum_regular_fallback_ratio: f32,
}

/// Broad, non-brittle measured values chosen from the profile matrix
/// measurement pass (all six shapes x the eight fast deterministic seeds).
/// The absolute caps are the hard safety limits from the profile config.
impl ProfileAcceptanceCriteria {
    #[must_use]
    pub const fn for_profile(profile: HexDeformationProfile) -> Self {
        match profile {
            HexDeformationProfile::Subtle => Self {
                average_displacement_min_ratio: 0.070,
                average_displacement_max_ratio: 0.120,
                maximum_displacement_ratio: 0.160,
                minimum_edge_length_ratio: 0.550,
                minimum_interior_angle_degrees: 80.0,
                maximum_interior_angle_degrees: 155.0,
                minimum_aspect_quality: 0.550,
                maximum_reduced_vertex_ratio: 0.150,
                maximum_regular_fallback_ratio: 0.150,
            },
            HexDeformationProfile::Organic => Self {
                average_displacement_min_ratio: 0.050,
                average_displacement_max_ratio: 0.140,
                maximum_displacement_ratio: 0.220,
                minimum_edge_length_ratio: 0.500,
                minimum_interior_angle_degrees: 80.0,
                maximum_interior_angle_degrees: 155.0,
                minimum_aspect_quality: 0.500,
                maximum_reduced_vertex_ratio: 0.150,
                maximum_regular_fallback_ratio: 0.150,
            },
            HexDeformationProfile::PagoniaLike => Self {
                average_displacement_min_ratio: 0.050,
                average_displacement_max_ratio: 0.200,
                maximum_displacement_ratio: 0.280,
                minimum_edge_length_ratio: 0.500,
                minimum_interior_angle_degrees: 80.0,
                maximum_interior_angle_degrees: 155.0,
                minimum_aspect_quality: 0.500,
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

    /// Lists every criterion that the measured report misses.
    #[must_use]
    pub fn violations(self, criteria: ProfileAcceptanceCriteria) -> Vec<String> {
        let mut issues = Vec::new();
        if self.average_displacement_ratio < criteria.average_displacement_min_ratio
            || self.average_displacement_ratio > criteria.average_displacement_max_ratio
        {
            issues.push(format!(
                "average displacement ratio {:.4} outside [{}, {}]",
                self.average_displacement_ratio,
                criteria.average_displacement_min_ratio,
                criteria.average_displacement_max_ratio
            ));
        }
        if self.minimum_edge_length_ratio < criteria.minimum_edge_length_ratio {
            issues.push(format!(
                "minimum edge length ratio {:.4} below {}",
                self.minimum_edge_length_ratio, criteria.minimum_edge_length_ratio
            ));
        }
        if self.minimum_interior_angle_degrees < criteria.minimum_interior_angle_degrees
            || self.maximum_interior_angle_degrees > criteria.maximum_interior_angle_degrees
        {
            issues.push(format!(
                "interior angles [{:.2}, {:.2}] outside [{}, {}]",
                self.minimum_interior_angle_degrees,
                self.maximum_interior_angle_degrees,
                criteria.minimum_interior_angle_degrees,
                criteria.maximum_interior_angle_degrees
            ));
        }
        if self.minimum_aspect_quality < criteria.minimum_aspect_quality {
            issues.push(format!(
                "minimum aspect quality {:.4} below {}",
                self.minimum_aspect_quality, criteria.minimum_aspect_quality
            ));
        }
        if self.reduced_vertex_ratio > criteria.maximum_reduced_vertex_ratio {
            issues.push(format!(
                "reduced vertex ratio {:.4} above {}",
                self.reduced_vertex_ratio, criteria.maximum_reduced_vertex_ratio
            ));
        }
        if self.regular_fallback_ratio > criteria.maximum_regular_fallback_ratio {
            issues.push(format!(
                "regular fallback ratio {:.4} above {}",
                self.regular_fallback_ratio, criteria.maximum_regular_fallback_ratio
            ));
        }
        issues
    }
}

#[must_use]
fn size_ratio(count: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        count as f32 / total as f32
    }
}

/// Emits a single concise warning per profile-acceptance miss. Intended to be
/// called only when the topology is (re)generated so it never repeats every
/// frame.
pub fn warn_on_acceptance_misses(topology: &HexFaceTopology, profile: HexDeformationProfile) {
    let report = ProfileAcceptanceReport::from_topology(topology);
    for issue in report.violations(ProfileAcceptanceCriteria::for_profile(profile)) {
        bevy::log::tracing::event!(
            bevy::log::tracing::Level::WARN,
            profile = profile.name(),
            issue,
            "Profile visual acceptance miss"
        );
    }
}
