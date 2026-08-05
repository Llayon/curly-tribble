//! Shared fixed-point arithmetic behind the blend in [`super::blend`].
//!
//! Reference selection and projection live here so no policy decision couples
//! to the blend implementation shape; the law itself lives in
//! [`super::blend_policy`] (a dependency leaf this module consumes).

const Q16: i64 = 65_536;

use super::blend::{blend_reference_with_margin, BlendStabilization, FixedVectorQ16};
use super::blend_policy::BlendReliabilityPolicy;

/// The component a stabilized blend direction is projected onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendReference {
    /// The correlated component (near-ties prefer this smooth field).
    Correlated,
    /// The local component.
    Local,
    /// No component has mass (both are exactly zero); use +X.
    FixedPositiveX,
}

/// A resolved projection target; `length_q16 >= 1` by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedBlendReference {
    vector: FixedVectorQ16,
    length_q16: i64,
    kind: BlendReference,
}

/// Resolves a [`BlendReference`] into a concrete vector whose length is
/// always at least 1, so projection can never divide by zero.
#[must_use]
pub fn resolve_reference(
    reference: BlendReference,
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
) -> ResolvedBlendReference {
    let (vector, length_q16) = match reference {
        BlendReference::Correlated => (correlated, component_length_q16(correlated)),
        BlendReference::Local => (local, component_length_q16(local)),
        BlendReference::FixedPositiveX => (FixedVectorQ16 { x: 1, y: 0 }, 1),
    };
    ResolvedBlendReference {
        vector,
        length_q16,
        kind: reference,
    }
}

/// Projection of `vector` onto a resolved reference, in Q16 fixed point.
///
/// Computed in `i128` so the sum of the two products can never overflow.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn projection_onto_reference_q16(
    vector: FixedVectorQ16,
    reference: ResolvedBlendReference,
) -> i64 {
    let numerator = i128::from(vector.x) * i128::from(reference.vector.x)
        + i128::from(vector.y) * i128::from(reference.vector.y);
    (numerator / i128::from(reference.length_q16)) as i64
}

/// Pre-normalization intermediates: `weighted_*` are the raw values, the
/// `stabilized_*` replace them only below the reliability floor. All
/// arithmetic matches [`super::blend::blend_to_displacement_q16`], so the
/// diagnostics never disagree with the displacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct WeightedBlendDiagnostics {
    pub weighted_x_q16: i64,
    pub weighted_y_q16: i64,
    pub weighted_length_q16: i64,
    pub correlated_length_q16: i64,
    pub local_length_q16: i64,
    pub target_magnitude_q16: i64,
    pub weighted_over_target_q16: i64,
    pub component_dot_q32: i64,
    pub anti_aligned: bool,
    pub reference: BlendReference,
    pub components_are_zero: bool,
    pub weighted_sum_zero: bool,
    pub stabilization_applied: bool,
    pub raw_projection_q16: i64,
    pub correction_q16: i64,
    pub minimum_projection_q16: i64,
    pub stabilized_x_q16: i64,
    pub stabilized_y_q16: i64,
    pub stabilized_length_q16: i64,
    pub stabilized_projection_q16: i64,
    pub stabilized_length_ratio_q16: i64,
    pub stabilized_projection_ratio_q16: i64,
}

/// Deterministic floor integer square root (Newton, from above). Normalization
/// stays integer so the blend is exactly platform-independent.
#[must_use]
pub fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut estimate = value;
    let mut next = estimate.div_ceil(2);
    while next < estimate {
        estimate = next;
        next = estimate.midpoint(value / estimate);
    }
    estimate
}

/// Q16 magnitude of a vector, or zero when it has no mass.
fn length_q16(x: i64, y: i64) -> i64 {
    if x == 0 && y == 0 {
        return 0;
    }
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    {
        integer_sqrt((x * x + y * y) as u64) as i64
    }
}

/// Q16 magnitude of a fixed-point vector (`isqrt(x^2 + y^2)`).
#[must_use]
pub fn component_length_q16(vector: FixedVectorQ16) -> i64 {
    length_q16(vector.x, vector.y)
}

/// Deterministic profile target magnitude: the larger component length.
#[must_use]
pub fn target_magnitude_q16(correlated: FixedVectorQ16, local: FixedVectorQ16) -> i64 {
    length_q16(correlated.x, correlated.y).max(length_q16(local.x, local.y))
}

/// Deterministic Q16 ratio of a value to a target, zero when the target is.
fn ratio_q16(value: i64, target: i64) -> i64 {
    if target == 0 {
        0
    } else {
        value * Q16 / target
    }
}

/// Applies the reliability-floor correction under an explicit policy.
#[must_use]
pub(crate) fn stabilize_blend_direction(
    weighted: FixedVectorQ16,
    weighted_length_q16: i64,
    target_magnitude_q16: i64,
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    reference: BlendReference,
    policy: BlendReliabilityPolicy,
) -> BlendStabilization {
    let resolved = resolve_reference(reference, correlated, local);
    let raw_projection = projection_onto_reference_q16(weighted, resolved);
    let minimum_projection = target_magnitude_q16 * policy.minimum_direction_ratio_q16() / Q16;
    if !policy.is_below_floor(weighted_length_q16, raw_projection, target_magnitude_q16) {
        return BlendStabilization {
            applied: false,
            projection_q16: raw_projection,
            correction_q16: 0,
            minimum_projection_q16: minimum_projection,
            stabilized_x_q16: weighted.x,
            stabilized_y_q16: weighted.y,
            stabilized_length_q16: weighted_length_q16,
            stabilized_projection_q16: raw_projection,
        };
    }
    let correction = minimum_projection - raw_projection;
    let (stabilized_x, stabilized_y) = match reference {
        BlendReference::FixedPositiveX => (weighted.x + correction, weighted.y),
        BlendReference::Correlated | BlendReference::Local => (
            weighted.x + resolved.vector.x * correction / resolved.length_q16,
            weighted.y + resolved.vector.y * correction / resolved.length_q16,
        ),
    };
    let stabilized = FixedVectorQ16 {
        x: stabilized_x,
        y: stabilized_y,
    };
    BlendStabilization {
        applied: true,
        projection_q16: raw_projection,
        correction_q16: correction,
        minimum_projection_q16: minimum_projection,
        stabilized_x_q16: stabilized_x,
        stabilized_y_q16: stabilized_y,
        stabilized_length_q16: length_q16(stabilized_x, stabilized_y),
        stabilized_projection_q16: projection_onto_reference_q16(stabilized, resolved),
    }
}

/// Computes the pre-normalization weighted-blend intermediates under the
/// production policy; the single source for both the produced displacement and
/// the near-zero diagnostics.
#[must_use]
pub fn weighted_blend_diagnostics(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_weight_q16: u32,
    local_weight_q16: u32,
) -> WeightedBlendDiagnostics {
    weighted_blend_diagnostics_with_policy(
        correlated,
        local,
        correlated_weight_q16,
        local_weight_q16,
        super::blend_policy::PRODUCTION_BLEND_RELIABILITY_POLICY,
    )
}

/// Like [`weighted_blend_diagnostics`], but under an arbitrary policy.
#[must_use]
pub fn weighted_blend_diagnostics_with_policy(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_weight_q16: u32,
    local_weight_q16: u32,
    policy: BlendReliabilityPolicy,
) -> WeightedBlendDiagnostics {
    let wc = i64::from(correlated_weight_q16);
    let wl = i64::from(local_weight_q16);
    let weighted_x = (correlated.x * wc + local.x * wl) / Q16;
    let weighted_y = (correlated.y * wc + local.y * wl) / Q16;
    let correlated_length = component_length_q16(correlated);
    let local_length = component_length_q16(local);
    let target = correlated_length.max(local_length);
    let weighted_length = length_q16(weighted_x, weighted_y);
    let weighted_over_target = ratio_q16(weighted_length, target);
    let dot = correlated.x * local.x + correlated.y * local.y;
    let reference = blend_reference_with_margin(
        correlated,
        local,
        correlated_weight_q16,
        local_weight_q16,
        policy.correlated_preference_margin_q16(),
    );
    let stabilization = stabilize_blend_direction(
        FixedVectorQ16 {
            x: weighted_x,
            y: weighted_y,
        },
        weighted_length,
        target,
        correlated,
        local,
        reference,
        policy,
    );
    WeightedBlendDiagnostics {
        weighted_x_q16: weighted_x,
        weighted_y_q16: weighted_y,
        weighted_length_q16: weighted_length,
        correlated_length_q16: correlated_length,
        local_length_q16: local_length,
        target_magnitude_q16: target,
        weighted_over_target_q16: weighted_over_target,
        component_dot_q32: dot,
        anti_aligned: dot < 0,
        reference,
        components_are_zero: correlated_length == 0 && local_length == 0,
        weighted_sum_zero: weighted_x == 0 && weighted_y == 0,
        stabilization_applied: stabilization.applied,
        raw_projection_q16: stabilization.projection_q16,
        correction_q16: stabilization.correction_q16,
        minimum_projection_q16: stabilization.minimum_projection_q16,
        stabilized_x_q16: stabilization.stabilized_x_q16,
        stabilized_y_q16: stabilization.stabilized_y_q16,
        stabilized_length_q16: stabilization.stabilized_length_q16,
        stabilized_projection_q16: stabilization.stabilized_projection_q16,
        stabilized_length_ratio_q16: ratio_q16(stabilization.stabilized_length_q16, target),
        stabilized_projection_ratio_q16: ratio_q16(stabilization.stabilized_projection_q16, target),
    }
}
