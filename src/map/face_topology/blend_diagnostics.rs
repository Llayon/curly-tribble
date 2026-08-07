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

/// Resolves a [`BlendReference`] into a concrete vector with length >= 1.
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

/// Projection of `vector` onto a resolved reference in Q16 fixed point.
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
    pub radial_length_increase_q16: i64,
    pub minimum_reliable_length_q16: i64,
    pub stabilized_x_q16: i64,
    pub stabilized_y_q16: i64,
    pub stabilized_length_q16: i64,
    pub stabilized_projection_q16: i64,
    pub stabilized_length_ratio_q16: i64,
    pub stabilized_projection_ratio_q16: i64,
}

/// Deterministic floor integer square root (Newton, from above).
#[must_use]
pub fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let (mut estimate, mut next) = (value, value.div_ceil(2));
    while next < estimate {
        estimate = next;
        next = estimate.midpoint(value / estimate);
    }
    estimate
}

/// Q16 magnitude of a vector, or zero when it has no mass.
fn length_q16(x: i64, y: i64) -> i64 {
    if x == 0 && y == 0 {
        0
    } else {
        #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
        {
            integer_sqrt((x * x + y * y) as u64) as i64
        }
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

/// Deterministic Q16 ratio of a value to a target, zero when target is zero.
fn ratio_q16(value: i64, target: i64) -> i64 {
    if target == 0 {
        0
    } else {
        value * Q16 / target
    }
}

/// Scaling of a vector component to the reliability floor (Option B: Production Domain).
///
/// Computes `ceil(|component| * target_floor / denominator) * sign(component)` using `u128`
/// intermediate arithmetic. Mathematically total for all production inputs (`denominator >= 1`).
#[must_use]
#[allow(clippy::cast_possible_truncation)]
#[rustfmt::skip]
pub(crate) fn scale_radial_component_q16(
    component: i64,
    target_floor: i64,
    denominator: i64,
) -> i64 {
    debug_assert!(denominator >= 1, "denominator must be >= 1 in production domain");
    debug_assert!(target_floor >= 0, "target_floor must be >= 0 in production domain");
    if denominator <= 0 {
        return 0;
    }
    let den = u128::from(denominator.unsigned_abs());
    let (abs_comp, is_neg) = if component < 0 {
        (u128::from(component.unsigned_abs()), true)
    } else {
        (u128::from(component.unsigned_abs()), false)
    };
    let num = abs_comp * u128::from(target_floor.unsigned_abs());
    let quotient = num.div_ceil(den);
    let signed = quotient as i64;
    if is_neg {
        -signed
    } else {
        signed
    }
}

/// Division away from zero for integer fixed-point radial scaling (Option C: Generic Checked Helper).
///
/// Computes `ceil(|numerator| / denominator) * sign(numerator)` using `u128`
/// magnitude arithmetic. Returns `None` if `denominator <= 0` or if the quotient overflows `i64`.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn div_away_from_zero(numerator: i128, denominator: i64) -> Option<i64> {
    if denominator <= 0 {
        return None;
    }
    let den = u128::from(denominator.unsigned_abs());
    let (abs_num, is_neg) = if numerator < 0 {
        (numerator.unsigned_abs(), true)
    } else {
        (numerator.unsigned_abs(), false)
    };
    let quotient = abs_num.div_ceil(den);
    if quotient > i64::MAX as u128 {
        return None;
    }
    let signed = quotient as i64;
    Some(if is_neg { -signed } else { signed })
}

/// Applies the reliability-floor correction under an explicit policy.
#[must_use]
#[rustfmt::skip]
pub(crate) fn stabilize_blend_direction(
    weighted: FixedVectorQ16, weighted_length_q16: i64, target_magnitude_q16: i64,
    correlated: FixedVectorQ16, local: FixedVectorQ16, reference: BlendReference, policy: BlendReliabilityPolicy,
) -> BlendStabilization {
    let resolved = resolve_reference(reference, correlated, local);
    let raw_projection = projection_onto_reference_q16(weighted, resolved);
    let minimum_reliable_length = target_magnitude_q16 * policy.minimum_direction_ratio_q16() / Q16;
    let (applied, stabilized_x, stabilized_y) = if policy.is_below_floor(weighted_length_q16, raw_projection, target_magnitude_q16) {
        let scale = |v: i64, d: i64| scale_radial_component_q16(v, minimum_reliable_length, d);
        let (sx, sy) = if weighted_length_q16 == 0 {
            match reference {
                BlendReference::FixedPositiveX => (minimum_reliable_length, 0),
                BlendReference::Correlated | BlendReference::Local => (scale(resolved.vector.x, resolved.length_q16), scale(resolved.vector.y, resolved.length_q16)),
            }
        } else {
            (scale(weighted.x, weighted_length_q16), scale(weighted.y, weighted_length_q16))
        };
        (true, sx, sy)
    } else {
        (false, weighted.x, weighted.y)
    };
    let stabilized_len = length_q16(stabilized_x, stabilized_y);
    let stab_vec = FixedVectorQ16 { x: stabilized_x, y: stabilized_y };
    BlendStabilization {
        applied, raw_projection_q16: raw_projection,
        radial_length_increase_q16: if applied { stabilized_len - weighted_length_q16 } else { 0 },
        minimum_reliable_length_q16: minimum_reliable_length,
        stabilized_x_q16: stabilized_x, stabilized_y_q16: stabilized_y,
        stabilized_length_q16: if applied { stabilized_len } else { weighted_length_q16 },
        stabilized_projection_q16: if applied { projection_onto_reference_q16(stab_vec, resolved) } else { raw_projection },
    }
}

/// Computes the pre-normalization weighted-blend intermediates under the
/// production policy; the single source for both the produced displacement and
/// the near-zero diagnostics.
#[must_use]
pub fn weighted_blend_diagnostics(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    cw: u32,
    lw: u32,
) -> WeightedBlendDiagnostics {
    weighted_blend_diagnostics_with_policy(
        correlated,
        local,
        cw,
        lw,
        super::blend_policy::PRODUCTION_BLEND_RELIABILITY_POLICY,
    )
}

/// Like [`weighted_blend_diagnostics`], but under an arbitrary policy.
#[must_use]
#[rustfmt::skip]
pub fn weighted_blend_diagnostics_with_policy(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_weight_q16: u32,
    local_weight_q16: u32,
    policy: BlendReliabilityPolicy,
) -> WeightedBlendDiagnostics {
    let (wc, wl) = (
        i64::from(correlated_weight_q16),
        i64::from(local_weight_q16),
    );
    let weighted_x = (correlated.x * wc + local.x * wl) / Q16;
    let weighted_y = (correlated.y * wc + local.y * wl) / Q16;
    let (correlated_length, local_length) = (
        component_length_q16(correlated),
        component_length_q16(local),
    );
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
        FixedVectorQ16 { x: weighted_x, y: weighted_y },
        weighted_length, target, correlated, local, reference, policy,
    );
    WeightedBlendDiagnostics {
        weighted_x_q16: weighted_x, weighted_y_q16: weighted_y, weighted_length_q16: weighted_length,
        correlated_length_q16: correlated_length, local_length_q16: local_length, target_magnitude_q16: target,
        weighted_over_target_q16: weighted_over_target, component_dot_q32: dot, anti_aligned: dot < 0, reference,
        components_are_zero: correlated_length == 0 && local_length == 0, weighted_sum_zero: weighted_x == 0 && weighted_y == 0,
        stabilization_applied: stabilization.applied, raw_projection_q16: stabilization.raw_projection_q16,
        radial_length_increase_q16: stabilization.radial_length_increase_q16, minimum_reliable_length_q16: stabilization.minimum_reliable_length_q16,
        stabilized_x_q16: stabilization.stabilized_x_q16, stabilized_y_q16: stabilization.stabilized_y_q16,
        stabilized_length_q16: stabilization.stabilized_length_q16, stabilized_projection_q16: stabilization.stabilized_projection_q16,
        stabilized_length_ratio_q16: ratio_q16(stabilization.stabilized_length_q16, target),
        stabilized_projection_ratio_q16: ratio_q16(stabilization.stabilized_projection_q16, target),
    }
}
