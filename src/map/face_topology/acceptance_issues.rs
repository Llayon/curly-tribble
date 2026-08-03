//! Typed observed-output issues for profile acceptance reporting.
//!
//! The `ProfileMetric` identity enum and the typed `ProfileAcceptanceIssue`
//! variants live here so `acceptance.rs` stays under the project's line limit.
//! Structural checks (non-finite metrics) always precede visual band checks,
//! giving a stable issue ordering.

/// Identity of a single measured metric, used by typed issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileMetric {
    AverageDisplacementRatio,
    MaximumDisplacementRatio,
    MinimumEdgeLengthRatio,
    MaximumEdgeLengthRatio,
    MinimumInteriorAngleDegrees,
    MaximumInteriorAngleDegrees,
    MinimumAspectQuality,
    ReducedVertexRatio,
    RegularFallbackRatio,
}

impl ProfileMetric {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AverageDisplacementRatio => "average displacement",
            Self::MaximumDisplacementRatio => "maximum displacement",
            Self::MinimumEdgeLengthRatio => "minimum edge",
            Self::MaximumEdgeLengthRatio => "maximum edge",
            Self::MinimumInteriorAngleDegrees => "minimum interior angle",
            Self::MaximumInteriorAngleDegrees => "maximum interior angle",
            Self::MinimumAspectQuality => "minimum aspect quality",
            Self::ReducedVertexRatio => "reduced vertex ratio",
            Self::RegularFallbackRatio => "regular fallback ratio",
        }
    }
}

/// Typed observed-output issues. Structural checks (non-finite metrics) always
/// precede the visual band checks, giving a stable issue ordering.
#[derive(Debug, Clone, PartialEq)]
pub enum ProfileAcceptanceIssue {
    NonFiniteMetric(ProfileMetric),
    AverageDisplacementOutside {
        measured: f32,
        min: f32,
        max: f32,
    },
    MinimumEdgeLengthBelow {
        measured: f32,
        min: f32,
    },
    InteriorAngleOutside {
        measured_min: f32,
        measured_max: f32,
        min: f32,
        max: f32,
    },
    MinimumAspectBelow {
        measured: f32,
        min: f32,
    },
    ReducedVertexRatioAbove {
        measured: f32,
        max: f32,
    },
    RegularFallbackRatioAbove {
        measured: f32,
        max: f32,
    },
}

/// Formats one issue into a single-line, reproducible string for the combined
/// warning and for test assertions.
#[must_use]
pub fn format_issue(issue: &ProfileAcceptanceIssue) -> String {
    match *issue {
        ProfileAcceptanceIssue::NonFiniteMetric(metric) => format!("non-finite {}", metric.name()),
        ProfileAcceptanceIssue::AverageDisplacementOutside { measured, min, max } => {
            format!("average displacement {measured:.4} outside [{min}, {max}]")
        }
        ProfileAcceptanceIssue::MinimumEdgeLengthBelow { measured, min } => {
            format!("minimum edge length {measured:.4} below {min}")
        }
        ProfileAcceptanceIssue::InteriorAngleOutside {
            measured_min,
            measured_max,
            min,
            max,
        } => {
            format!("interior angles [{measured_min:.2}, {measured_max:.2}] outside [{min}, {max}]")
        }
        ProfileAcceptanceIssue::MinimumAspectBelow { measured, min } => {
            format!("minimum aspect quality {measured:.4} below {min}")
        }
        ProfileAcceptanceIssue::ReducedVertexRatioAbove { measured, max } => {
            format!("reduced vertex ratio {measured:.4} above {max}")
        }
        ProfileAcceptanceIssue::RegularFallbackRatioAbove { measured, max } => {
            format!("regular fallback ratio {measured:.4} above {max}")
        }
    }
}
