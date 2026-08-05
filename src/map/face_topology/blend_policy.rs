//! Reliability-floor policy for the blend direction correction.
//!
//! This module is a **dependency leaf**: it imports nothing from the blend
//! implementation, so policy decisions can never couple to arithmetic detail.
//! The activation mode, threshold ratio, and preference margin are collected
//! here so threshold scans can build genuinely different geometries instead of
//! re-classifying corners over one shared topology.

/// Q16 fixed-point one.
const Q16: i64 = 65_536;

/// Weighted-length ratio below which a blend direction is unreliable
/// (Q16 units: `1_024` == `1/64`).
pub const MIN_RELIABLE_DIRECTION_RATIO_Q16: i64 = 1_024;

/// Reference tie-break band as a ratio of the larger weighted length
/// (Q16 units: `8_192` == `1/8`).
pub const CORRELATED_PREFERENCE_MARGIN_Q16: i64 = 8_192;

/// Which measurement decides that a corner is below the reliability floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendActivation {
    /// Compare the raw weighted length to the floor.
    WeightedLength,
    /// Compare the raw projection onto the reference to the floor.
    ReferenceProjection,
}

/// Deterministic reliability-floor law for one generation run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendReliabilityPolicy {
    minimum_direction_ratio_q16: i64,
    correlated_preference_margin_q16: i64,
    activation: BlendActivation,
}

impl BlendReliabilityPolicy {
    #[must_use]
    pub const fn new(
        minimum_direction_ratio_q16: i64,
        correlated_preference_margin_q16: i64,
        activation: BlendActivation,
    ) -> Self {
        Self {
            minimum_direction_ratio_q16,
            correlated_preference_margin_q16,
            activation,
        }
    }

    #[must_use]
    pub const fn minimum_direction_ratio_q16(self) -> i64 {
        self.minimum_direction_ratio_q16
    }

    #[must_use]
    pub const fn correlated_preference_margin_q16(self) -> i64 {
        self.correlated_preference_margin_q16
    }

    #[must_use]
    pub const fn activation(self) -> BlendActivation {
        self.activation
    }

    /// Whether this blended corner must be corrected.
    ///
    /// Compared with cross multiplication in `i128` so the boundary is never
    /// rounded by an intermediate integer division: the exact predicate is
    /// `weighted / target < floor` (or the projection variant), evaluated
    /// without overflow or truncation at `floor +/- 1`. A zero target cannot
    /// exceed any non-negative floor, so it never triggers.
    #[must_use]
    pub fn is_below_floor(
        self,
        weighted_length_q16: i64,
        raw_projection_q16: i64,
        target_q16: i64,
    ) -> bool {
        if target_q16 <= 0 {
            return false;
        }
        let target = i128::from(target_q16);
        let floor = i128::from(self.minimum_direction_ratio_q16);
        match self.activation {
            BlendActivation::WeightedLength => {
                i128::from(weighted_length_q16) * i128::from(Q16) < target * floor
            }
            BlendActivation::ReferenceProjection => {
                i128::from(raw_projection_q16) * i128::from(Q16) < target * floor
            }
        }
    }
}

/// The law the production blend uses today: 1/64 length threshold, 1/8
/// correlated preference. This is the bit-identical surface of `9ad12ae`.
pub const PRODUCTION_BLEND_RELIABILITY_POLICY: BlendReliabilityPolicy = BlendReliabilityPolicy::new(
    MIN_RELIABLE_DIRECTION_RATIO_Q16,
    CORRELATED_PREFERENCE_MARGIN_Q16,
    BlendActivation::WeightedLength,
);

/// The raw legacy law: a zero floor never triggers correction, so every corner
/// keeps the pre-floor normalization exactly.
pub const DISABLED_BLEND_RELIABILITY_POLICY: BlendReliabilityPolicy = BlendReliabilityPolicy::new(
    0,
    CORRELATED_PREFERENCE_MARGIN_Q16,
    BlendActivation::WeightedLength,
);

/// Builds a scanning candidate from an explicit ratio and activation mode.
#[must_use]
pub const fn candidate_policy(
    minimum_direction_ratio_q16: i64,
    activation: BlendActivation,
) -> BlendReliabilityPolicy {
    BlendReliabilityPolicy::new(
        minimum_direction_ratio_q16,
        CORRELATED_PREFERENCE_MARGIN_Q16,
        activation,
    )
}
